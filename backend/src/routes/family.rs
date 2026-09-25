//! Family deposits (家人 定期): the same registry and 收訖 lifecycle as
//! `deposits.rs`, but hard-isolated — this module must never credit a cash
//! row or record a month item, because family money is not the user's money.

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use chrono::Datelike;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use super::{AppState, FAMILY_DEPOSIT_COLUMNS, now_timestamp, row_to_family_deposit, today};
use crate::calc::{DepositFacts, DepositInput, active_totals, validate_deposit};
use crate::error::{ApiError, ErrorBody};
use crate::models::{FamilyDeposit, FamilyDepositPatch, MpfNotePatch, NewFamilyDeposit};
use crate::mpf;

/// `app_meta` key prefix for the per-holder note: `family.note.<holder>`.
pub const FAMILY_NOTE_PREFIX: &str = "family.note.";

fn note_key(holder: &str) -> String {
    format!("{FAMILY_NOTE_PREFIX}{holder}")
}

#[derive(Debug, Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
pub struct ListQuery {
    /// `active` (not yet 收訖, even past end date) or `ended` (received).
    pub status: Option<String>,
    /// Restrict to one holder (exact match).
    pub holder: Option<String>,
    /// Restrict to deposits ending in this year.
    pub year: Option<i32>,
    /// `asc` (default) or `desc`, by end date.
    pub order: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/family/deposits",
    tag = "family",
    params(ListQuery),
    responses(
        (status = 200, description = "List family deposits", body = [FamilyDeposit]),
    )
)]
pub async fn list(
    State(state): State<AppState>,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<FamilyDeposit>>, ApiError> {
    let mut sql = format!("SELECT {FAMILY_DEPOSIT_COLUMNS} FROM family_deposits");
    let mut conditions: Vec<&str> = Vec::new();
    if let Some(status) = query.status.as_deref() {
        match status.to_ascii_lowercase().as_str() {
            "active" => conditions.push("received_at IS NULL"),
            "ended" => conditions.push("received_at IS NOT NULL"),
            _ => return Err(ApiError::field("status", "status must be active or ended")),
        }
    }
    if query.holder.is_some() {
        conditions.push("holder = ?");
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
    if let Some(holder) = query.holder.as_deref() {
        statement = statement.bind(holder);
    }
    if let Some(year) = query.year {
        statement = statement
            .bind(format!("{year:04}-01-01"))
            .bind(format!("{year:04}-12-31"));
    }

    statement
        .fetch_all(&state.pool)
        .await?
        .iter()
        .map(row_to_family_deposit)
        .collect::<Result<Vec<_>, _>>()
        .map(Json)
}

fn validate_holder(holder: &str) -> Result<String, ApiError> {
    let holder = holder.trim();
    if holder.is_empty() {
        return Err(ApiError::field("holder", "holder is required"));
    }
    Ok(holder.to_string())
}

#[utoipa::path(
    post,
    path = "/api/family/deposits",
    tag = "family",
    request_body = NewFamilyDeposit,
    responses(
        (status = 201, description = "Family deposit created", body = FamilyDeposit),
        (status = 400, description = "Missing or invalid fields", body = ErrorBody),
    )
)]
pub async fn create(
    State(state): State<AppState>,
    Json(body): Json<NewFamilyDeposit>,
) -> Result<(StatusCode, Json<FamilyDeposit>), ApiError> {
    let holder = validate_holder(&body.holder)?;
    let validated = validate_deposit(DepositInput {
        label: body.label.as_deref(),
        bank: body.bank.as_deref(),
        principal: body.principal,
        rate: None,
        interest: body.interest,
        start_date: body.start_date.as_deref(),
        end_date: &body.end_date,
    })
    .map_err(ApiError::Validation)?;

    let sort_order: i64 =
        sqlx::query_scalar("SELECT COALESCE(MAX(sort_order), 0) + 1 FROM family_deposits")
            .fetch_one(&state.pool)
            .await?;
    let note = body
        .note
        .as_deref()
        .map(str::trim)
        .filter(|note| !note.is_empty());
    // Unlike `deposits`, a past end_date is never auto-received — the user
    // confirms receipt explicitly with 收訖.
    let now = now_timestamp();
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO family_deposits (holder, label, bank, principal, interest, start_date, \
         end_date, note, sort_order, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(&holder)
    .bind(validated.label.as_deref())
    .bind(validated.bank.as_deref())
    .bind(validated.principal)
    .bind(validated.interest)
    .bind(validated.start_date.as_deref())
    .bind(&validated.end_date)
    .bind(note)
    .bind(sort_order)
    .bind(&now)
    .bind(&now)
    .fetch_one(&state.pool)
    .await?;
    Ok((StatusCode::CREATED, Json(load_one(&state.pool, id).await?)))
}

#[utoipa::path(
    patch,
    path = "/api/family/deposits/{id}",
    tag = "family",
    params(("id" = i64, Path, description = "Family deposit id")),
    request_body = FamilyDepositPatch,
    responses(
        (status = 200, description = "Updated family deposit", body = FamilyDeposit),
        (status = 400, description = "Missing or invalid fields", body = ErrorBody),
        (status = 404, description = "Family deposit not found", body = ErrorBody),
    )
)]
pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(patch): Json<FamilyDepositPatch>,
) -> Result<Json<FamilyDeposit>, ApiError> {
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

    let holder = match patch.holder {
        Some(holder) => validate_holder(&holder)?,
        None => existing.holder,
    };
    let label = merge_text(patch.label, existing.label);
    let bank = merge_text(patch.bank, existing.bank);
    let note = merge_text(patch.note, existing.note);
    let principal = merge_num(patch.principal, existing.principal);
    let interest = merge_num(patch.interest, existing.interest);
    let start_date = merge_text(patch.start_date, existing.start_date);
    let end_date = patch.end_date.unwrap_or(existing.end_date);

    let validated = validate_deposit(DepositInput {
        label: label.as_deref(),
        bank: bank.as_deref(),
        principal,
        rate: None,
        interest,
        start_date: start_date.as_deref(),
        end_date: &end_date,
    })
    .map_err(ApiError::Validation)?;

    sqlx::query(
        "UPDATE family_deposits SET holder = ?, label = ?, bank = ?, principal = ?, \
         interest = ?, start_date = ?, end_date = ?, note = ?, updated_at = ? WHERE id = ?",
    )
    .bind(&holder)
    .bind(validated.label.as_deref())
    .bind(validated.bank.as_deref())
    .bind(validated.principal)
    .bind(validated.interest)
    .bind(validated.start_date.as_deref())
    .bind(&validated.end_date)
    .bind(&note)
    .bind(now_timestamp())
    .bind(id)
    .execute(&state.pool)
    .await?;

    Ok(Json(load_one(&state.pool, id).await?))
}

#[utoipa::path(
    delete,
    path = "/api/family/deposits/{id}",
    tag = "family",
    params(("id" = i64, Path, description = "Family deposit id")),
    responses(
        (status = 204, description = "Family deposit deleted"),
        (status = 404, description = "Family deposit not found", body = ErrorBody),
    )
)]
pub async fn remove(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    let result = sqlx::query("DELETE FROM family_deposits WHERE id = ?")
        .bind(id)
        .execute(&state.pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound(format!("family deposit {id} not found")));
    }
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
pub struct ReceiveFamilyDeposit {
    /// 收訖日 (`YYYY-MM-DD`); defaults to today.
    pub received_at: Option<String>,
    /// Corrects the stored interest to the amount actually received.
    pub interest: Option<f64>,
}

/// 收訖: mark the deposit received and optionally correct the interest — no
/// cash credit and no month item; family money never enters the ledger.
#[utoipa::path(
    post,
    path = "/api/family/deposits/{id}/receive",
    tag = "family",
    params(("id" = i64, Path, description = "Family deposit id")),
    request_body = ReceiveFamilyDeposit,
    responses(
        (status = 200, description = "Family deposit marked 收訖 (no side effects)", body = FamilyDeposit),
        (status = 400, description = "Missing or invalid fields", body = ErrorBody),
        (status = 404, description = "Family deposit not found", body = ErrorBody),
        (status = 409, description = "Family deposit already received", body = ErrorBody),
    )
)]
pub async fn receive(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<ReceiveFamilyDeposit>,
) -> Result<Json<FamilyDeposit>, ApiError> {
    let existing = load_one(&state.pool, id).await?;
    if existing.received_at.is_some() {
        return Err(ApiError::Conflict(format!(
            "family deposit {id} is already received"
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

    sqlx::query(
        "UPDATE family_deposits SET received_at = ?, interest = ?, updated_at = ? WHERE id = ?",
    )
    .bind(&received_at)
    .bind(interest)
    .bind(now_timestamp())
    .bind(id)
    .execute(&state.pool)
    .await?;

    Ok(Json(load_one(&state.pool, id).await?))
}

/// 取消收訖: clear the received flag so the deposit returns to 未到期.
#[utoipa::path(
    post,
    path = "/api/family/deposits/{id}/unreceive",
    tag = "family",
    params(("id" = i64, Path, description = "Family deposit id")),
    responses(
        (status = 200, description = "收訖 cleared", body = FamilyDeposit),
        (status = 404, description = "Family deposit not found", body = ErrorBody),
        (status = 409, description = "Family deposit is not received", body = ErrorBody),
    )
)]
pub async fn unreceive(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<FamilyDeposit>, ApiError> {
    let existing = load_one(&state.pool, id).await?;
    if existing.received_at.is_none() {
        return Err(ApiError::Conflict(format!(
            "family deposit {id} is not received"
        )));
    }
    sqlx::query("UPDATE family_deposits SET received_at = NULL, updated_at = ? WHERE id = ?")
        .bind(now_timestamp())
        .bind(id)
        .execute(&state.pool)
        .await?;

    Ok(Json(load_one(&state.pool, id).await?))
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct HolderNoteResponse {
    pub holder: String,
    pub note: Option<String>,
}

/// `PUT /family/holders/:holder/note` — an empty or `null` note clears it.
#[utoipa::path(
    put,
    path = "/api/family/holders/{holder}/note",
    tag = "family",
    params(("holder" = String, Path, description = "Holder name")),
    request_body = MpfNotePatch,
    responses(
        (status = 200, description = "Stored holder note (null clears it)", body = HolderNoteResponse),
    )
)]
pub async fn update_note(
    State(state): State<AppState>,
    Path(holder): Path<String>,
    Json(body): Json<MpfNotePatch>,
) -> Result<Json<HolderNoteResponse>, ApiError> {
    let holder = validate_holder(&holder)?;
    let key = note_key(&holder);
    let note = body
        .note
        .map(|note| note.trim().to_string())
        .filter(|note| !note.is_empty());
    mpf::meta_put(&state.pool, &key, note.as_deref()).await?;
    Ok(Json(HolderNoteResponse {
        holder,
        note: mpf::meta_get(&state.pool, &key).await?,
    }))
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct FamilyHolderSummary {
    pub holder: String,
    pub note: Option<String>,
    /// Unreceived deposits, earliest maturity first.
    pub upcoming: Vec<FamilyDeposit>,
    /// Σ principal over this holder's deposits ending in the future.
    pub active_principal: f64,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct FamilyDepositSummary {
    /// The date `status`/`active_principal` were derived against.
    pub today: String,
    /// Every holder with deposits or a stored note, sorted by name.
    pub holders: Vec<FamilyHolderSummary>,
    /// Distinct end years in the data plus the current year, for the filter.
    pub history_years: Vec<i32>,
}

/// Every figure here is derived from the stored family deposits on each read.
#[utoipa::path(
    get,
    path = "/api/family/deposits/summary",
    tag = "family",
    responses(
        (status = 200, description = "Per-holder family deposit summary", body = FamilyDepositSummary),
    )
)]
pub async fn summary(
    State(state): State<AppState>,
) -> Result<Json<FamilyDepositSummary>, ApiError> {
    let today = today();
    let deposits = load_all(&state.pool).await?;

    // Holders = distinct deposit holders ∪ holders with a stored note.
    let note_keys: Vec<String> =
        sqlx::query_scalar("SELECT key FROM app_meta WHERE key LIKE 'family.note.%' ORDER BY key")
            .fetch_all(&state.pool)
            .await?;
    let mut holders: Vec<String> = deposits
        .iter()
        .map(|deposit| deposit.holder.clone())
        .collect();
    holders.extend(
        note_keys
            .iter()
            .filter_map(|key| key.strip_prefix(FAMILY_NOTE_PREFIX).map(str::to_string)),
    );
    holders.sort();
    holders.dedup();

    let mut summaries = Vec::with_capacity(holders.len());
    for holder in holders {
        let rows: Vec<&FamilyDeposit> = deposits
            .iter()
            .filter(|deposit| deposit.holder == holder)
            .collect();
        let mut upcoming: Vec<FamilyDeposit> = rows
            .iter()
            .filter(|deposit| deposit.received_at.is_none())
            .map(|deposit| (*deposit).clone())
            .collect();
        upcoming.sort_by(|a, b| {
            a.end_date
                .cmp(&b.end_date)
                .then(a.sort_order.cmp(&b.sort_order))
        });
        let facts: Vec<DepositFacts<'_>> = rows
            .iter()
            .map(|deposit| {
                let end_date = chrono::NaiveDate::parse_from_str(&deposit.end_date, "%Y-%m-%d")
                    .map_err(|_| {
                        ApiError::Conflict(format!(
                            "stored end_date {} is not valid",
                            deposit.end_date
                        ))
                    })?;
                Ok(DepositFacts {
                    bank: None,
                    principal: deposit.principal,
                    interest: deposit.interest,
                    end_date,
                })
            })
            .collect::<Result<_, ApiError>>()?;
        summaries.push(FamilyHolderSummary {
            note: mpf::meta_get(&state.pool, &note_key(&holder)).await?,
            holder,
            upcoming,
            active_principal: active_totals(&facts, today).principal,
        });
    }

    let mut history_years: Vec<i32> = deposits.iter().map(|deposit| deposit.end_year).collect();
    history_years.push(today.year());
    history_years.sort_unstable();
    history_years.dedup();

    Ok(Json(FamilyDepositSummary {
        today: today.to_string(),
        holders: summaries,
        history_years,
    }))
}

async fn load_one(pool: &SqlitePool, id: i64) -> Result<FamilyDeposit, ApiError> {
    let row = sqlx::query(sqlx::AssertSqlSafe(format!(
        "SELECT {FAMILY_DEPOSIT_COLUMNS} FROM family_deposits WHERE id = ?"
    )))
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| ApiError::NotFound(format!("family deposit {id} not found")))?;
    row_to_family_deposit(&row)
}

async fn load_all(pool: &SqlitePool) -> Result<Vec<FamilyDeposit>, ApiError> {
    let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
        "SELECT {FAMILY_DEPOSIT_COLUMNS} FROM family_deposits ORDER BY sort_order, id"
    )))
    .fetch_all(pool)
    .await?;
    rows.iter()
        .map(row_to_family_deposit)
        .collect::<Result<Vec<_>, _>>()
}
