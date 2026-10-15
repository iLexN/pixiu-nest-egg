//! Family deposits (家人 定期): the same registry and 收訖 lifecycle as
//! `deposits.rs`, but hard-isolated — this module must never credit a cash
//! row or record a month item, because family money is not the user's money.

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use chrono::Datelike;
use serde::{Deserialize, Serialize};

use super::AppState;
use crate::calc::{DepositFacts, DepositInput, active_totals, validate_deposit};
use crate::error::{ApiError, ErrorBody};
use crate::models::{FamilyDeposit, FamilyDepositPatch, MpfNotePatch, NewFamilyDeposit};
use crate::store::family as store;
use crate::store::meta::{keys_like, meta_get, meta_put};
use crate::store::today;

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
    let received = match query.status.as_deref() {
        Some(status) => match status.to_ascii_lowercase().as_str() {
            "active" => Some(false),
            "ended" => Some(true),
            _ => return Err(ApiError::field("status", "status must be active or ended")),
        },
        None => None,
    };
    let descending =
        matches!(query.order.as_deref(), Some(order) if order.eq_ignore_ascii_case("desc"));
    store::list(
        &state.pool,
        received,
        query.holder.as_deref(),
        query.year,
        descending,
    )
    .await
    .map(Json)
    .map_err(ApiError::from)
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

    let sort_order = store::next_sort_order(&state.pool).await?;
    let note = body
        .note
        .as_deref()
        .map(str::trim)
        .filter(|note| !note.is_empty());
    let id =
        store::insert_family_deposit(&state.pool, &holder, &validated, note, sort_order).await?;
    Ok((
        StatusCode::CREATED,
        Json(store::load_one(&state.pool, id).await?),
    ))
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
    let existing = store::load_one(&state.pool, id).await?;

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

    store::update_family_deposit(&state.pool, id, &holder, &validated, &note).await?;

    Ok(Json(store::load_one(&state.pool, id).await?))
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
    if store::delete_family_deposit(&state.pool, id).await? == 0 {
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
    let existing = store::load_one(&state.pool, id).await?;
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

    store::mark_received(&state.pool, id, &received_at, interest).await?;

    Ok(Json(store::load_one(&state.pool, id).await?))
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
    let key = store::note_key(&holder);
    let note = body
        .note
        .map(|note| note.trim().to_string())
        .filter(|note| !note.is_empty());
    meta_put(&state.pool, &key, note.as_deref()).await?;
    Ok(Json(HolderNoteResponse {
        holder,
        note: meta_get(&state.pool, &key).await?,
    }))
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct FamilyHolderSummary {
    pub holder: String,
    pub note: Option<String>,
    /// Unreceived deposits, earliest maturity first.
    pub upcoming: Vec<FamilyDeposit>,
    /// Σ principal over this holder's unreceived deposits.
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
    let deposits = store::load_all(&state.pool).await?;

    // Holders = distinct deposit holders ∪ holders with a stored note.
    let note_keys = keys_like(&state.pool, "family.note.%").await?;
    let mut holders: Vec<String> = deposits
        .iter()
        .map(|deposit| deposit.holder.clone())
        .collect();
    holders.extend(note_keys.iter().filter_map(|key| {
        key.strip_prefix(store::FAMILY_NOTE_PREFIX)
            .map(str::to_string)
    }));
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
                    received: deposit.received_at.is_some(),
                })
            })
            .collect::<Result<_, ApiError>>()?;
        summaries.push(FamilyHolderSummary {
            note: meta_get(&state.pool, &store::note_key(&holder)).await?,
            holder,
            upcoming,
            active_principal: active_totals(&facts).principal,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connect_memory;

    #[tokio::test]
    async fn matured_unreceived_deposit_counts_in_the_holders_active_principal() {
        let pool = connect_memory().await.expect("memory db");
        let validated = validate_deposit(DepositInput {
            label: Some("SC-9179"),
            bank: Some("SC"),
            principal: Some(100000.0),
            rate: None,
            interest: Some(500.0),
            start_date: None,
            end_date: "2020-01-01",
        })
        .expect("valid");
        store::insert_family_deposit(&pool, "媽媽", &validated, None, 1)
            .await
            .unwrap();

        let response = summary(State(AppState { pool })).await.expect("summary").0;
        let holder = response
            .holders
            .iter()
            .find(|holder| holder.holder == "媽媽")
            .expect("holder");
        assert_eq!(holder.active_principal, 100000.0);
        assert_eq!(holder.upcoming.len(), 1);
    }
}
