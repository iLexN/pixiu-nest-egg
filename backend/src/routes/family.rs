//! Family deposits (家人 定期): the same registry and 收訖 lifecycle as
//! `deposits.rs`, but hard-isolated — this module must never credit a cash
//! row or record a month item, because family money is not the user's money.

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};

use super::{AppState, finite_or, parse_order, parse_received_at, parse_status};
use crate::calc::{DepositInput, validate_deposit};
use crate::error::{ApiError, ErrorBody};
use crate::models::{FamilyDeposit, FamilyDepositPatch, MpfNotePatch, NewFamilyDeposit};
use crate::patch::{merge_num, merge_text};
use crate::store::family as store;

pub use crate::service::family::{FamilyDepositSummary, FamilyHolderSummary};

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
    operation_id = "family_list",
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
    let received = parse_status(query.status.as_deref(), "active", "ended")?;
    let descending = parse_order(query.order.as_deref());
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
    operation_id = "family_create",
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
    operation_id = "family_update",
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
    operation_id = "family_remove",
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
    operation_id = "family_receive",
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
    let received_at = parse_received_at(body.received_at.as_deref())?;
    let interest = finite_or("interest", body.interest, existing.interest)?;

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
    operation_id = "family_update_note",
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
    let note = body
        .note
        .map(|note| note.trim().to_string())
        .filter(|note| !note.is_empty());
    crate::store::settings::set_family_note(&state.pool, &holder, note.as_deref()).await?;
    Ok(Json(HolderNoteResponse {
        note: crate::store::settings::family_note(&state.pool, &holder).await?,
        holder,
    }))
}

/// Every figure here is derived from the stored family deposits on each read.
#[utoipa::path(
    operation_id = "family_summary",
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
    Ok(Json(crate::service::family::summary(&state.pool).await?))
}
