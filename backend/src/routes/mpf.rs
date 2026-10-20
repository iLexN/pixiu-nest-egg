use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::Serialize;

use super::AppState;
use super::patch::{merge, merge_text};
use crate::calc::{MpfAccountInput, MpfTotals, mpf_totals, validate_mpf_account};
use crate::error::{ApiError, ErrorBody};
use crate::models::{MpfAccount, MpfAccountPatch, MpfHistoryRow, MpfNotePatch, NewMpfAccount};
use crate::service::mpf::present_account;
use crate::store::mpf;
use crate::store::today;

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct MpfResponse {
    pub today: String,
    pub accounts: Vec<MpfAccount>,
    pub totals: MpfTotals,
    pub note: Option<String>,
    pub history: Vec<MpfHistoryRow>,
}

#[utoipa::path(
    get,
    path = "/api/mpf",
    tag = "mpf",
    responses(
        (status = 200, description = "MPF accounts with derived figures, history, note, and totals", body = MpfResponse),
    )
)]
pub async fn overview(State(state): State<AppState>) -> Result<Json<MpfResponse>, ApiError> {
    let today = today();
    let stored = mpf::load_all_stored(&state.pool).await?;
    let history = mpf::load_history(&state.pool, None).await?;
    let accounts: Vec<MpfAccount> = stored
        .iter()
        .map(|account| present_account(account, &history, today))
        .collect();
    let facts: Vec<_> = stored
        .iter()
        .map(|account| mpf::facts_of(account, &history))
        .collect();
    let seed_max = crate::store::settings::mpf_seed_max(&state.pool).await?;
    let totals = mpf_totals(&facts, today, seed_max.0, seed_max.1);
    let note = crate::store::settings::mpf_note(&state.pool).await?;
    Ok(Json(MpfResponse {
        today: today.to_string(),
        accounts,
        totals,
        note,
        history,
    }))
}

#[utoipa::path(
    post,
    path = "/api/mpf/accounts",
    tag = "mpf",
    request_body = NewMpfAccount,
    responses(
        (status = 201, description = "MPF account created", body = MpfAccount),
        (status = 400, description = "Missing or invalid fields", body = ErrorBody),
    )
)]
pub async fn create(
    State(state): State<AppState>,
    Json(body): Json<NewMpfAccount>,
) -> Result<(StatusCode, Json<MpfAccount>), ApiError> {
    let contributions = body.contributions.unwrap_or(0.0);
    let balance = body.balance.unwrap_or(0.0);
    validate_mpf_account(MpfAccountInput {
        label: &body.label,
        contributions,
        balance,
    })
    .map_err(ApiError::Validation)?;

    let sort_order = mpf::next_sort_order(&state.pool).await?;
    let id = mpf::insert_account(
        &state.pool,
        body.label.trim(),
        body.trustee.as_deref(),
        contributions,
        balance,
        body.plan_name.as_deref(),
        body.member_no.as_deref(),
        sort_order,
    )
    .await?;

    let stored = mpf::load_stored(&state.pool, id).await?;
    Ok((
        StatusCode::CREATED,
        Json(present_account(&stored, &[], today())),
    ))
}

#[utoipa::path(
    patch,
    path = "/api/mpf/accounts/{id}",
    tag = "mpf",
    params(("id" = i64, Path, description = "MPF account id")),
    request_body = MpfAccountPatch,
    responses(
        (status = 200, description = "Updated MPF account (a balance update also records history)", body = MpfAccount),
        (status = 400, description = "Missing or invalid fields", body = ErrorBody),
        (status = 404, description = "MPF account not found", body = ErrorBody),
    )
)]
pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(patch): Json<MpfAccountPatch>,
) -> Result<Json<MpfAccount>, ApiError> {
    let existing = mpf::load_stored(&state.pool, id).await?;

    let label = patch.label.unwrap_or(existing.label);
    let trustee = merge_text(patch.trustee, existing.trustee);
    let plan_name = merge_text(patch.plan_name, existing.plan_name);
    let member_no = merge_text(patch.member_no, existing.member_no);
    let contributions = merge(patch.contributions.flatten(), existing.contributions);
    let balance = merge(patch.balance.flatten(), existing.balance);

    validate_mpf_account(MpfAccountInput {
        label: &label,
        contributions,
        balance,
    })
    .map_err(ApiError::Validation)?;

    let values_changed = contributions != existing.contributions || balance != existing.balance;

    let mut tx = state.pool.begin().await?;
    mpf::update_account(
        &mut tx,
        id,
        label.trim(),
        &trustee,
        contributions,
        balance,
        &plan_name,
        &member_no,
    )
    .await?;
    if values_changed {
        mpf::record_history(
            &mut tx,
            id,
            (existing.contributions, existing.balance),
            (contributions, balance),
            today(),
            existing.created_on,
        )
        .await?;
    }
    tx.commit().await?;

    let stored = mpf::load_stored(&state.pool, id).await?;
    let history = mpf::load_history(&state.pool, Some(id)).await?;
    Ok(Json(present_account(&stored, &history, today())))
}

/// An account with history rows is refused so the record never loses its past.
#[utoipa::path(
    delete,
    path = "/api/mpf/accounts/{id}",
    tag = "mpf",
    params(("id" = i64, Path, description = "MPF account id")),
    responses(
        (status = 204, description = "MPF account and its history deleted"),
        (status = 404, description = "MPF account not found", body = ErrorBody),
    )
)]
pub async fn remove(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    mpf::load_stored(&state.pool, id).await?;
    let rows = mpf::count_history(&state.pool, id).await?;
    if rows > 0 {
        return Err(ApiError::Conflict(format!(
            "mpf account {id} has history records"
        )));
    }
    mpf::delete_account(&state.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct NoteResponse {
    pub note: Option<String>,
}

#[utoipa::path(
    patch,
    path = "/api/mpf/note",
    tag = "mpf",
    request_body = MpfNotePatch,
    responses(
        (status = 200, description = "Stored MPF page note (null clears it)", body = NoteResponse),
    )
)]
pub async fn update_note(
    State(state): State<AppState>,
    Json(body): Json<MpfNotePatch>,
) -> Result<Json<NoteResponse>, ApiError> {
    crate::store::settings::set_mpf_note(&state.pool, body.note.as_deref()).await?;
    Ok(Json(NoteResponse {
        note: crate::store::settings::mpf_note(&state.pool).await?,
    }))
}

/// Deleting a stale row recomputes last-month/max without it on next read.
#[utoipa::path(
    delete,
    path = "/api/mpf/history/{id}",
    tag = "mpf",
    params(("id" = i64, Path, description = "History row id")),
    responses(
        (status = 204, description = "History row deleted"),
        (status = 404, description = "History row not found", body = ErrorBody),
    )
)]
pub async fn remove_history(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    if mpf::delete_history(&state.pool, id).await? == 0 {
        return Err(ApiError::NotFound(format!(
            "mpf history row {id} not found"
        )));
    }
    Ok(StatusCode::NO_CONTENT)
}
