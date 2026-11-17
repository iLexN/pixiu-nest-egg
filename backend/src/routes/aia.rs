use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde::Deserialize;

use super::AppState;
use super::mpf::NoteResponse;
use crate::calc::{AiaEventInput, AiaPolicyInput, validate_aia_event, validate_aia_policy};
use crate::error::{ApiError, ErrorBody};
use crate::models::{AiaEvent, AiaNotePatch, AiaPolicy, AiaPolicyPatch, NewAiaEvent, NewAiaPolicy};
use crate::patch::{merge_num, merge_text};
use crate::store::aia as store;
use crate::store::now_timestamp;

pub use crate::service::aia::{AiaSummaryResponse, PolicyWithEvents};

#[utoipa::path(
    operation_id = "aia_list",
    get,
    path = "/api/aia/policies",
    tag = "aia",
    responses(
        (status = 200, description = "List AIA policies", body = [AiaPolicy]),
    )
)]
pub async fn list(State(state): State<AppState>) -> Result<Json<Vec<AiaPolicy>>, ApiError> {
    store::load_all_policies(&state.pool)
        .await
        .map(Json)
        .map_err(ApiError::from)
}

#[utoipa::path(
    operation_id = "aia_create",
    post,
    path = "/api/aia/policies",
    tag = "aia",
    request_body = NewAiaPolicy,
    responses(
        (status = 201, description = "AIA policy created", body = AiaPolicy),
        (status = 400, description = "Missing or invalid fields", body = ErrorBody),
    )
)]
pub async fn create(
    State(state): State<AppState>,
    Json(body): Json<NewAiaPolicy>,
) -> Result<(StatusCode, Json<AiaPolicy>), ApiError> {
    let validated = validate_aia_policy(AiaPolicyInput {
        label: &body.label,
        policy_no: body.policy_no.as_deref(),
        next_pay_date: body.next_pay_date.as_deref(),
        premium_usd: body.premium_usd,
        value_usd: body.value_usd,
        remaining_years: body.remaining_years,
        withdrew_usd: body.withdrew_usd,
        note: body.note.as_deref(),
        link: body.link.as_deref(),
        excluded: body.excluded,
        in_account: body.in_account,
    })
    .map_err(ApiError::Validation)?;

    let sort_order = store::next_sort_order(&state.pool).await?;
    let id = store::insert_policy(&state.pool, &validated, sort_order).await?;
    Ok((
        StatusCode::CREATED,
        Json(store::load_policy(&state.pool, id).await?),
    ))
}

#[utoipa::path(
    operation_id = "aia_update",
    patch,
    path = "/api/aia/policies/{id}",
    tag = "aia",
    params(("id" = i64, Path, description = "Policy id")),
    request_body = AiaPolicyPatch,
    responses(
        (status = 200, description = "Updated policy", body = AiaPolicy),
        (status = 400, description = "Missing or invalid fields", body = ErrorBody),
        (status = 404, description = "Policy not found", body = ErrorBody),
    )
)]
pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(patch): Json<AiaPolicyPatch>,
) -> Result<Json<AiaPolicy>, ApiError> {
    let existing = store::load_policy(&state.pool, id).await?;

    let label = patch.label.unwrap_or(existing.label);
    let policy_no = merge_text(patch.policy_no, existing.policy_no);
    let next_pay_date = merge_text(patch.next_pay_date, existing.next_pay_date);
    let premium_usd = patch.premium_usd.unwrap_or(existing.premium_usd);
    let value_usd = patch.value_usd.unwrap_or(existing.value_usd);
    let remaining_years = merge_num(patch.remaining_years, existing.remaining_years);
    let withdrew_usd = patch.withdrew_usd.unwrap_or(existing.withdrew_usd);
    let note = merge_text(patch.note, existing.note);
    let link = merge_text(patch.link, existing.link);
    let excluded = patch.excluded.unwrap_or(existing.excluded);
    let in_account = patch.in_account.unwrap_or(existing.in_account);

    let validated = validate_aia_policy(AiaPolicyInput {
        label: &label,
        policy_no: policy_no.as_deref(),
        next_pay_date: next_pay_date.as_deref(),
        premium_usd,
        value_usd,
        remaining_years,
        withdrew_usd,
        note: note.as_deref(),
        link: link.as_deref(),
        excluded,
        in_account,
    })
    .map_err(ApiError::Validation)?;

    // A new current value is like a new manual price: restamp its timestamp.
    let value_updated_at = if patch.value_usd.is_some() && value_usd != existing.value_usd {
        Some(now_timestamp())
    } else {
        existing.value_updated_at
    };

    store::update_policy(&state.pool, id, &validated, &value_updated_at).await?;

    Ok(Json(store::load_policy(&state.pool, id).await?))
}

/// Deleting a policy removes its events: nothing outside them references a
/// policy, so delete is always permitted.
#[utoipa::path(
    operation_id = "aia_remove",
    delete,
    path = "/api/aia/policies/{id}",
    tag = "aia",
    params(("id" = i64, Path, description = "Policy id")),
    responses(
        (status = 204, description = "Policy and its events deleted"),
        (status = 404, description = "Policy not found", body = ErrorBody),
    )
)]
pub async fn remove(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    if store::delete_with_events(&state.pool, id).await? == 0 {
        return Err(ApiError::NotFound(format!("aia policy {id} not found")));
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Every figure here is derived from the stored rows on each read.
#[utoipa::path(
    operation_id = "aia_summary",
    get,
    path = "/api/aia/summary",
    tag = "aia",
    responses(
        (status = 200, description = "Policies with events, USD→HKD rate, and totals", body = AiaSummaryResponse),
    )
)]
pub async fn summary(State(state): State<AppState>) -> Result<Json<AiaSummaryResponse>, ApiError> {
    Ok(Json(crate::service::aia::summary(&state.pool).await?))
}

/// The section's free-text note, kept in `app_meta` under `aia.note` — the
/// same facility as the MPF page note; an empty note deletes the key.
#[utoipa::path(
    operation_id = "aia_update_note",
    patch,
    path = "/api/aia/note",
    tag = "aia",
    request_body = AiaNotePatch,
    responses(
        (status = 200, description = "Stored AIA page note (null clears it)", body = NoteResponse),
    )
)]
pub async fn update_note(
    State(state): State<AppState>,
    Json(body): Json<AiaNotePatch>,
) -> Result<Json<NoteResponse>, ApiError> {
    crate::store::settings::set_aia_note(&state.pool, body.note.as_deref()).await?;
    Ok(Json(NoteResponse {
        note: crate::store::settings::aia_note(&state.pool).await?,
    }))
}

#[derive(Debug, Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
pub struct EventListQuery {
    pub policy_id: Option<i64>,
}

#[utoipa::path(
    operation_id = "aia_list_events",
    get,
    path = "/api/aia/events",
    tag = "aia",
    params(EventListQuery),
    responses(
        (status = 200, description = "List AIA events (premium payments / withdrawals)", body = [AiaEvent]),
    )
)]
pub async fn list_events(
    State(state): State<AppState>,
    Query(query): Query<EventListQuery>,
) -> Result<Json<Vec<AiaEvent>>, ApiError> {
    store::list_events(&state.pool, query.policy_id)
        .await
        .map(Json)
        .map_err(ApiError::from)
}

/// Recording an event is one transaction: snapshot the policy fields it will
/// touch onto the event row, insert the row, then apply the update — a payment
/// grows the premium, counts down a remaining year and moves the next due
/// date; a withdrawal grows the withdrew figure.
#[utoipa::path(
    operation_id = "aia_create_event",
    post,
    path = "/api/aia/events",
    tag = "aia",
    request_body = NewAiaEvent,
    responses(
        (status = 201, description = "AIA event created", body = AiaEvent),
        (status = 400, description = "Missing or invalid fields", body = ErrorBody),
        (status = 404, description = "Policy not found", body = ErrorBody),
    )
)]
pub async fn create_event(
    State(state): State<AppState>,
    Json(body): Json<NewAiaEvent>,
) -> Result<(StatusCode, Json<AiaEvent>), ApiError> {
    let validated = validate_aia_event(AiaEventInput {
        kind: body.kind,
        event_date: &body.event_date,
        amount_usd: body.amount_usd,
        next_pay_date: body.next_pay_date.as_deref(),
    })
    .map_err(ApiError::Validation)?;

    let id = crate::service::aia::create_event(
        &state.pool,
        body.policy_id,
        &validated,
        body.note.as_deref(),
    )
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(store::load_event(&state.pool, id).await?),
    ))
}

/// Deleting an event undoes it: the amount leaves its cumulative field and the
/// policy fields the event recorded snap back.
#[utoipa::path(
    operation_id = "aia_remove_event",
    delete,
    path = "/api/aia/events/{id}",
    tag = "aia",
    params(("id" = i64, Path, description = "Event id")),
    responses(
        (status = 204, description = "Event deleted (undoes its derived effects)"),
        (status = 404, description = "Event not found", body = ErrorBody),
    )
)]
pub async fn remove_event(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    crate::service::aia::remove_event(&state.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
