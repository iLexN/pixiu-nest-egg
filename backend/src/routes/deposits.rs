use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use chrono::Datelike;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use super::{AppState, DEPOSIT_COLUMNS, now_timestamp, record_receipt_item, row_to_deposit, today};
use crate::calc::{
    ActiveMonthBucket, ActiveTotals, BankRollup, DepositFacts, DepositInput, ValidatedDeposit,
    YearRollup, active_month_rollup, active_totals, bank_rollup, validate_deposit, year_rollups,
};
use crate::error::{ApiError, ErrorBody};
use crate::models::{Deposit, DepositPatch, NewDeposit};

#[derive(Debug, Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
pub struct ListQuery {
    /// `active` (not yet 收訖, even past end date) or `ended` (received).
    pub status: Option<String>,
    /// Restrict to deposits ending in this year.
    pub year: Option<i32>,
    /// `asc` (default) or `desc`, by end date.
    pub order: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/deposits",
    tag = "deposits",
    params(ListQuery),
    responses(
        (status = 200, description = "List 定期 deposits", body = [Deposit]),
    )
)]
pub async fn list(
    State(state): State<AppState>,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<Deposit>>, ApiError> {
    let mut sql = format!("SELECT {DEPOSIT_COLUMNS} FROM deposits");
    let mut conditions: Vec<&str> = Vec::new();
    if let Some(status) = query.status.as_deref() {
        match status.to_ascii_lowercase().as_str() {
            "active" => conditions.push("received_at IS NULL"),
            "ended" => conditions.push("received_at IS NOT NULL"),
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

    let mut statement = sqlx::query(sqlx::AssertSqlSafe(sql));
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

#[utoipa::path(
    post,
    path = "/api/deposits",
    tag = "deposits",
    request_body = NewDeposit,
    responses(
        (status = 201, description = "Deposit created", body = Deposit),
        (status = 400, description = "Missing or invalid fields", body = ErrorBody),
    )
)]
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
        start_date: body.start_date.as_deref(),
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

#[utoipa::path(
    patch,
    path = "/api/deposits/{id}",
    tag = "deposits",
    params(("id" = i64, Path, description = "Deposit id")),
    request_body = DepositPatch,
    responses(
        (status = 200, description = "Updated deposit", body = Deposit),
        (status = 400, description = "Missing or invalid fields", body = ErrorBody),
        (status = 404, description = "Deposit not found", body = ErrorBody),
    )
)]
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
    let start_date = merge_text(patch.start_date, existing.start_date);
    let end_date = patch.end_date.unwrap_or(existing.end_date);

    let validated = validate_deposit(DepositInput {
        label: label.as_deref(),
        bank: bank.as_deref(),
        principal,
        rate,
        interest,
        start_date: start_date.as_deref(),
        end_date: &end_date,
    })
    .map_err(ApiError::Validation)?;

    sqlx::query(
        "UPDATE deposits SET label = ?, bank = ?, principal = ?, rate = ?, interest = ?, \
         start_date = ?, end_date = ?, note1 = ?, note2 = ?, updated_at = ? WHERE id = ?",
    )
    .bind(&validated.label)
    .bind(&validated.bank)
    .bind(validated.principal)
    .bind(validated.rate)
    .bind(validated.interest)
    .bind(&validated.start_date)
    .bind(&validated.end_date)
    .bind(&note1)
    .bind(&note2)
    .bind(now_timestamp())
    .bind(id)
    .execute(&state.pool)
    .await?;

    Ok(Json(load_one(&state.pool, id).await?))
}

#[utoipa::path(
    delete,
    path = "/api/deposits/{id}",
    tag = "deposits",
    params(("id" = i64, Path, description = "Deposit id")),
    responses(
        (status = 204, description = "Deposit deleted"),
        (status = 404, description = "Deposit not found", body = ErrorBody),
    )
)]
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

#[derive(Debug, Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
pub struct ReceiveDeposit {
    /// 收訖日 (`YYYY-MM-DD`); defaults to today.
    pub received_at: Option<String>,
    /// Corrects the stored interest to the amount actually received.
    pub interest: Option<f64>,
    /// Cash `manual_assets` row to credit the returned money into.
    pub credit_asset_id: Option<i64>,
    /// Amount credited; defaults to principal + interest.
    pub credit_amount: Option<f64>,
}

/// 收訖: mark the deposit received, optionally credit its principal + interest
/// to a cash manual asset, and record the month's `dep-end` adjustment item —
/// the whole sheet "定期 end step" in one action.
#[utoipa::path(
    post,
    path = "/api/deposits/{id}/receive",
    tag = "deposits",
    params(("id" = i64, Path, description = "Deposit id")),
    request_body = ReceiveDeposit,
    responses(
        (status = 200, description = "Deposit marked 收訖 with optional bank-in", body = Deposit),
        (status = 400, description = "Missing or invalid fields", body = ErrorBody),
        (status = 404, description = "Deposit not found", body = ErrorBody),
        (status = 409, description = "Deposit already received", body = ErrorBody),
    )
)]
pub async fn receive(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<ReceiveDeposit>,
) -> Result<Json<Deposit>, ApiError> {
    let existing = load_one(&state.pool, id).await?;
    if existing.received_at.is_some() {
        return Err(ApiError::Conflict(format!(
            "deposit {id} is already received"
        )));
    }
    let received_at = match body.received_at.as_deref() {
        Some(date) => chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
            .map_err(|_| ApiError::field("received_at", "received_at must be YYYY-MM-DD"))?
            .to_string(),
        None => today().to_string(),
    };
    let interest = match body.interest {
        Some(value) if !value.is_finite() => {
            return Err(ApiError::field("interest", "interest must be a number"));
        }
        Some(value) => Some(value),
        None => existing.interest,
    };
    let credit_amount = body
        .credit_amount
        .unwrap_or_else(|| existing.principal.unwrap_or(0.0) + interest.unwrap_or(0.0));
    if !credit_amount.is_finite() {
        return Err(ApiError::field(
            "credit_amount",
            "credit_amount must be a number",
        ));
    }
    if let Some(asset_id) = body.credit_asset_id {
        let kind: Option<String> =
            sqlx::query_scalar("SELECT kind FROM manual_assets WHERE id = ?")
                .bind(asset_id)
                .fetch_optional(&state.pool)
                .await?;
        match kind.as_deref() {
            Some("cash") => {}
            Some(_) => {
                return Err(ApiError::field(
                    "credit_asset_id",
                    "credit target must be a 活期 cash row",
                ));
            }
            None => {
                return Err(ApiError::NotFound(format!(
                    "manual asset {asset_id} not found"
                )));
            }
        }
    }

    let month = format!("{}-01", &existing.end_date[..7]);
    let auto_key = format!("dep-end:{id}");
    let total = existing.principal.unwrap_or(0.0) + interest.unwrap_or(0.0);
    let now = now_timestamp();
    let mut tx = state.pool.begin().await?;
    sqlx::query(
        "UPDATE deposits SET received_at = ?, interest = ?, credited_asset_id = ?, \
         credited_amount = ?, updated_at = ? WHERE id = ?",
    )
    .bind(&received_at)
    .bind(interest)
    .bind(body.credit_asset_id)
    .bind(body.credit_asset_id.map(|_| credit_amount))
    .bind(&now)
    .bind(id)
    .execute(&mut *tx)
    .await?;
    if let Some(asset_id) = body.credit_asset_id {
        sqlx::query("UPDATE manual_assets SET amount = amount + ?, updated_at = ? WHERE id = ?")
            .bind(credit_amount)
            .bind(&now)
            .bind(asset_id)
            .execute(&mut *tx)
            .await?;
    }
    // The dep-end inflow item the suggestion would have created.
    let label = existing.label.as_deref().unwrap_or("定期");
    record_receipt_item(&mut tx, &month, &auto_key, label, total).await?;
    tx.commit().await?;

    Ok(Json(load_one(&state.pool, id).await?))
}

/// Undo a 收訖: reverse the stored cash credit, drop the auto-created dep-end
/// item, and clear the received flag so the deposit returns to 未到期定期.
#[utoipa::path(
    post,
    path = "/api/deposits/{id}/unreceive",
    tag = "deposits",
    params(("id" = i64, Path, description = "Deposit id")),
    responses(
        (status = 200, description = "收訖 cleared and bank-in reversed", body = Deposit),
        (status = 404, description = "Deposit not found", body = ErrorBody),
        (status = 409, description = "Deposit is not received", body = ErrorBody),
    )
)]
pub async fn unreceive(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Deposit>, ApiError> {
    let existing = load_one(&state.pool, id).await?;
    if existing.received_at.is_none() {
        return Err(ApiError::Conflict(format!("deposit {id} is not received")));
    }
    let credited: Option<(Option<i64>, Option<f64>)> =
        sqlx::query_as("SELECT credited_asset_id, credited_amount FROM deposits WHERE id = ?")
            .bind(id)
            .fetch_optional(&state.pool)
            .await?;

    let month = format!("{}-01", &existing.end_date[..7]);
    let now = now_timestamp();
    let mut tx = state.pool.begin().await?;
    if let Some((Some(asset_id), Some(amount))) = credited {
        sqlx::query("UPDATE manual_assets SET amount = amount - ?, updated_at = ? WHERE id = ?")
            .bind(amount)
            .bind(&now)
            .bind(asset_id)
            .execute(&mut *tx)
            .await?;
    }
    sqlx::query("DELETE FROM month_items WHERE month = ? AND auto_key = ?")
        .bind(&month)
        .bind(format!("dep-end:{id}"))
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "UPDATE deposits SET received_at = NULL, credited_asset_id = NULL, \
         credited_amount = NULL, updated_at = ? WHERE id = ?",
    )
    .bind(&now)
    .bind(id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(Json(load_one(&state.pool, id).await?))
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
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
#[utoipa::path(
    get,
    path = "/api/deposits/summary",
    tag = "deposits",
    responses(
        (status = 200, description = "Active deposits plus month/bank/year rollups", body = DepositSummaryResponse),
    )
)]
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

    // The 未到期定期 list shows deposits until 收訖 — an end_date in the past
    // without receipt means "matured, awaiting confirmation". The totals and
    // rollups below still follow the sheet's end_date rule.
    let mut upcoming: Vec<Deposit> = facts
        .iter()
        .filter(|(deposit, _)| deposit.received_at.is_none())
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
    // A deposit entered already past its end date is recorded as received —
    // its interest fed 利息 all along. 收訖 only applies to future deposits.
    let received_at = (deposit.end_date.as_str() <= today().to_string().as_str())
        .then(|| deposit.end_date.clone());
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO deposits (label, bank, principal, rate, interest, start_date, end_date, \
         received_at, note1, note2, sort_order, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(deposit.label.as_deref())
    .bind(deposit.bank.as_deref())
    .bind(deposit.principal)
    .bind(deposit.rate)
    .bind(deposit.interest)
    .bind(deposit.start_date.as_deref())
    .bind(&deposit.end_date)
    .bind(&received_at)
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
    let row = sqlx::query(sqlx::AssertSqlSafe(format!(
        "SELECT {DEPOSIT_COLUMNS} FROM deposits WHERE id = ?"
    )))
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| ApiError::NotFound(format!("deposit {id} not found")))?;
    row_to_deposit(&row, today())
}

pub async fn load_all(pool: &SqlitePool) -> Result<Vec<Deposit>, ApiError> {
    let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
        "SELECT {DEPOSIT_COLUMNS} FROM deposits ORDER BY sort_order, id"
    )))
    .fetch_all(pool)
    .await?;
    let today = today();
    rows.iter()
        .map(|row| row_to_deposit(row, today))
        .collect::<Result<Vec<_>, _>>()
}
