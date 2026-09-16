use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::Datelike;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use super::{now_timestamp, row_to_deposit, today, AppState, DEPOSIT_COLUMNS};
use crate::calc::{
    active_month_rollup, active_totals, bank_rollup, deposit_active, validate_deposit,
    year_rollups, ActiveMonthBucket, ActiveTotals, BankRollup, DepositFacts, DepositInput,
    ValidatedDeposit, YearRollup,
};
use crate::error::ApiError;
use crate::models::{Deposit, DepositPatch, NewDeposit};

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    /// `active` (end date in the future) or `ended`.
    pub status: Option<String>,
    /// Restrict to deposits ending in this year.
    pub year: Option<i32>,
    /// `asc` (default) or `desc`, by end date.
    pub order: Option<String>,
}

pub async fn list(
    State(state): State<AppState>,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<Deposit>>, ApiError> {
    let mut sql = format!("SELECT {DEPOSIT_COLUMNS} FROM deposits");
    let mut conditions: Vec<&str> = Vec::new();
    if let Some(status) = query.status.as_deref() {
        match status.to_ascii_lowercase().as_str() {
            "active" => conditions.push("end_date > ?"),
            "ended" => conditions.push("end_date <= ?"),
            _ => return Err(ApiError::field("status", "status must be active or ended")),
        }
    }
    if query.year.is_some() {
        conditions.push("end_date >= ? AND end_date <= ?");
    }
    if !conditions.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&conditions.join(" AND "));
    }
    let descending =
        matches!(query.order.as_deref(), Some(order) if order.eq_ignore_ascii_case("desc"));
    sql.push_str(if descending {
        " ORDER BY end_date DESC, sort_order DESC, id DESC"
    } else {
        " ORDER BY end_date ASC, sort_order ASC, id ASC"
    });

    let mut statement = sqlx::query(&sql);
    if query.status.is_some() {
        statement = statement.bind(today().to_string());
    }
    if let Some(year) = query.year {
        statement = statement
            .bind(format!("{year:04}-01-01"))
            .bind(format!("{year:04}-12-31"));
    }

    let today = today();
    statement
        .fetch_all(&state.pool)
        .await?
        .iter()
        .map(|row| row_to_deposit(row, today))
        .collect::<Result<Vec<_>, _>>()
        .map(Json)
}

pub async fn create(
    State(state): State<AppState>,
    Json(body): Json<NewDeposit>,
) -> Result<(StatusCode, Json<Deposit>), ApiError> {
    let validated = validate_deposit(DepositInput {
        label: body.label.as_deref(),
        bank: body.bank.as_deref(),
        principal: body.principal,
        rate: body.rate,
        interest: body.interest,
        end_date: &body.end_date,
    })
    .map_err(ApiError::Validation)?;

    let sort_order: i64 =
        sqlx::query_scalar("SELECT COALESCE(MAX(sort_order), 0) + 1 FROM deposits")
            .fetch_one(&state.pool)
            .await?;
    let id = insert_deposit(&state.pool, &validated, sort_order, &body).await?;
    Ok((StatusCode::CREATED, Json(load_one(&state.pool, id).await?)))
}

pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(patch): Json<DepositPatch>,
) -> Result<Json<Deposit>, ApiError> {
    let existing = load_one(&state.pool, id).await?;

    let merge_text = |patch: Option<Option<String>>, existing: Option<String>| match patch {
        Some(value) => value
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        None => existing,
    };
    let merge_num = |patch: Option<Option<f64>>, existing: Option<f64>| match patch {
        Some(value) => value,
        None => existing,
    };

    let label = merge_text(patch.label, existing.label);
    let bank = merge_text(patch.bank, existing.bank);
    let note1 = merge_text(patch.note1, existing.note1);
    let note2 = merge_text(patch.note2, existing.note2);
    let principal = merge_num(patch.principal, existing.principal);
    let rate = merge_num(patch.rate, existing.rate);
    let interest = merge_num(patch.interest, existing.interest);
    let end_date = patch.end_date.unwrap_or(existing.end_date);

    let validated = validate_deposit(DepositInput {
        label: label.as_deref(),
        bank: bank.as_deref(),
        principal,
        rate,
        interest,
        end_date: &end_date,
    })
    .map_err(ApiError::Validation)?;

    sqlx::query(
        "UPDATE deposits SET label = ?, bank = ?, principal = ?, rate = ?, interest = ?, \
         end_date = ?, note1 = ?, note2 = ?, updated_at = ? WHERE id = ?",
    )
    .bind(&validated.label)
    .bind(&validated.bank)
    .bind(validated.principal)
    .bind(validated.rate)
    .bind(validated.interest)
    .bind(&validated.end_date)
    .bind(&note1)
    .bind(&note2)
    .bind(now_timestamp())
    .bind(id)
    .execute(&state.pool)
    .await?;

    Ok(Json(load_one(&state.pool, id).await?))
}

pub async fn remove(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    let result = sqlx::query("DELETE FROM deposits WHERE id = ?")
        .bind(id)
        .execute(&state.pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound(format!("deposit {id} not found")));
    }
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Serialize)]
pub struct DepositSummaryResponse {
    /// The date `status` was derived against.
    pub today: String,
    /// Active deposits (end date in the future), earliest maturity first.
    pub upcoming: Vec<Deposit>,
    pub active_totals: ActiveTotals,
    /// Active deposits grouped by end (year, month).
    pub months: Vec<ActiveMonthBucket>,
    /// Active deposits grouped by label prefix.
    pub banks: Vec<BankRollup>,
    /// Per-year month tables over every deposit ending that year.
    pub years: Vec<YearRollup>,
    /// Distinct end years in the data plus the current year, for the filter.
    pub history_years: Vec<i32>,
}

/// Every figure here is derived from the stored deposits on each read.
pub async fn summary(
    State(state): State<AppState>,
) -> Result<Json<DepositSummaryResponse>, ApiError> {
    let today = today();
    let deposits = load_all(&state.pool).await?;

    let mut facts = Vec::with_capacity(deposits.len());
    for deposit in &deposits {
        let end_date =
            chrono::NaiveDate::parse_from_str(&deposit.end_date, "%Y-%m-%d").map_err(|_| {
                ApiError::Conflict(format!("stored end_date {} is not valid", deposit.end_date))
            })?;
        facts.push((
            deposit,
            DepositFacts {
                bank: deposit.bank.as_deref(),
                principal: deposit.principal,
                interest: deposit.interest,
                end_date,
            },
        ));
    }

    let mut upcoming: Vec<Deposit> = facts
        .iter()
        .filter(|(_, fact)| deposit_active(fact.end_date, today))
        .map(|(deposit, _)| (*deposit).clone())
        .collect();
    upcoming.sort_by(|a, b| {
        a.end_date
            .cmp(&b.end_date)
            .then(a.sort_order.cmp(&b.sort_order))
    });

    let fact_refs: Vec<DepositFacts<'_>> = facts.iter().map(|(_, fact)| *fact).collect();

    let mut history_years: Vec<i32> = deposits.iter().map(|deposit| deposit.end_year).collect();
    history_years.push(today.year());
    history_years.sort_unstable();
    history_years.dedup();

    Ok(Json(DepositSummaryResponse {
        today: today.to_string(),
        upcoming,
        active_totals: active_totals(&fact_refs, today),
        months: active_month_rollup(&fact_refs, today),
        banks: bank_rollup(&fact_refs, today),
        years: year_rollups(&fact_refs),
        history_years,
    }))
}

pub async fn insert_deposit(
    pool: &SqlitePool,
    deposit: &ValidatedDeposit,
    sort_order: i64,
    body: &NewDeposit,
) -> Result<i64, ApiError> {
    let now = now_timestamp();
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO deposits (label, bank, principal, rate, interest, end_date, note1, note2, \
         sort_order, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(deposit.label.as_deref())
    .bind(deposit.bank.as_deref())
    .bind(deposit.principal)
    .bind(deposit.rate)
    .bind(deposit.interest)
    .bind(&deposit.end_date)
    .bind(body.note1.as_deref())
    .bind(body.note2.as_deref())
    .bind(sort_order)
    .bind(&now)
    .bind(&now)
    .fetch_one(pool)
    .await?;
    Ok(id)
}

pub async fn load_one(pool: &SqlitePool, id: i64) -> Result<Deposit, ApiError> {
    let row = sqlx::query(&format!(
        "SELECT {DEPOSIT_COLUMNS} FROM deposits WHERE id = ?"
    ))
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| ApiError::NotFound(format!("deposit {id} not found")))?;
    row_to_deposit(&row, today())
}

pub async fn load_all(pool: &SqlitePool) -> Result<Vec<Deposit>, ApiError> {
    let rows = sqlx::query(&format!(
        "SELECT {DEPOSIT_COLUMNS} FROM deposits ORDER BY sort_order, id"
    ))
    .fetch_all(pool)
    .await?;
    let today = today();
    rows.iter()
        .map(|row| row_to_deposit(row, today))
        .collect::<Result<Vec<_>, _>>()
}
