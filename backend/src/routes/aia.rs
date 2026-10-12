use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};

use super::AppState;
use super::mpf::NoteResponse;
use crate::calc::{
    AiaEventInput, AiaPolicyFacts, AiaPolicyInput, AiaTotals, aia_totals, apply_aia_payment,
    apply_aia_withdrawal, next_premium_due, undo_aia_event_amount, validate_aia_event,
    validate_aia_policy,
};
use crate::error::{ApiError, ErrorBody};
use crate::models::{
    AiaEvent, AiaEventKind, AiaNotePatch, AiaPolicy, AiaPolicyPatch, AiaRatePatch, NewAiaEvent,
    NewAiaPolicy,
};
use crate::store::aia as store;
use crate::store::{now_timestamp, today};

fn parse_day(date: &str) -> Result<chrono::NaiveDate, ApiError> {
    chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|_| ApiError::Conflict(format!("stored date {date} is not valid")))
}

#[utoipa::path(
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
    let mut tx = state.pool.begin().await?;
    if store::delete_with_events(&mut tx, id).await? == 0 {
        return Err(ApiError::NotFound(format!("aia policy {id} not found")));
    }
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

/// A policy with its payment/withdrawal history attached.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct PolicyWithEvents {
    #[serde(flatten)]
    pub policy: AiaPolicy,
    pub events: Vec<AiaEvent>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct AiaSummaryResponse {
    pub today: String,
    /// The manual USD→HKD rate; HKD figures are absent while it is unset.
    pub rate: Option<f64>,
    /// Earliest premium-due date still ahead — the premium the user owes next.
    pub next_premium_due: Option<String>,
    /// The section's free-text note (`aia.note` in `app_meta`).
    pub note: Option<String>,
    pub totals: AiaTotals,
    pub policies: Vec<PolicyWithEvents>,
}

/// Every figure here is derived from the stored rows on each read.
#[utoipa::path(
    get,
    path = "/api/aia/summary",
    tag = "aia",
    responses(
        (status = 200, description = "Policies with events, USD→HKD rate, and totals", body = AiaSummaryResponse),
    )
)]
pub async fn summary(State(state): State<AppState>) -> Result<Json<AiaSummaryResponse>, ApiError> {
    let today = today();
    let policies = store::load_all_policies(&state.pool).await?;
    let rate = crate::store::meta::meta_get(&state.pool, store::RATE_KEY)
        .await?
        .and_then(|raw| raw.parse::<f64>().ok());
    let facts: Vec<AiaPolicyFacts> = policies.iter().map(store::facts_of).collect();
    let totals = aia_totals(&facts, rate);
    let next_due = next_premium_due(
        policies
            .iter()
            .filter_map(|policy| policy.next_pay_date.as_deref())
            .map(parse_day)
            .collect::<Result<Vec<_>, _>>()?
            .into_iter(),
        today,
    );

    let mut with_events = Vec::with_capacity(policies.len());
    for policy in policies {
        let events = store::load_policy_events(&state.pool, policy.id).await?;
        with_events.push(PolicyWithEvents { policy, events });
    }

    let note = crate::store::meta::meta_get(&state.pool, store::NOTE_KEY).await?;
    Ok(Json(AiaSummaryResponse {
        today: today.to_string(),
        rate,
        next_premium_due: next_due.map(|date| date.to_string()),
        note,
        totals,
        policies: with_events,
    }))
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct RateResponse {
    pub rate: Option<f64>,
}

/// The section's free-text note, kept in `app_meta` under `aia.note` — the
/// same facility as the MPF page note; an empty note deletes the key.
#[utoipa::path(
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
    crate::store::meta::meta_put(&state.pool, store::NOTE_KEY, body.note.as_deref()).await?;
    Ok(Json(NoteResponse {
        note: crate::store::meta::meta_get(&state.pool, store::NOTE_KEY).await?,
    }))
}

/// The workbook keeps a live GOOGLEFINANCE rate in `Overview!N3`; the app keeps
/// a manual copy in `app_meta` — same facility as the MPF note.
#[utoipa::path(
    patch,
    path = "/api/aia/rate",
    tag = "aia",
    request_body = AiaRatePatch,
    responses(
        (status = 200, description = "Stored USD→HKD rate (null clears it)", body = RateResponse),
        (status = 400, description = "Rate must be positive", body = ErrorBody),
    )
)]
pub async fn update_rate(
    State(state): State<AppState>,
    Json(body): Json<AiaRatePatch>,
) -> Result<Json<RateResponse>, ApiError> {
    if let Some(rate) = body.rate
        && (!rate.is_finite() || rate <= 0.0)
    {
        return Err(ApiError::field("rate", "rate must be a positive number"));
    }
    let value = body.rate.map(|rate| rate.to_string());
    crate::store::meta::meta_put(&state.pool, store::RATE_KEY, value.as_deref()).await?;
    let rate = crate::store::meta::meta_get(&state.pool, store::RATE_KEY)
        .await?
        .and_then(|raw| raw.parse::<f64>().ok());
    Ok(Json(RateResponse { rate }))
}

#[derive(Debug, Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
pub struct EventListQuery {
    pub policy_id: Option<i64>,
}

#[utoipa::path(
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

    let mut tx = state.pool.begin().await?;
    let policy = store::row_to_policy(&store::policy_row(&mut *tx, body.policy_id).await?)?;

    let (prev_next_pay_date, prev_remaining_years) = match validated.kind {
        AiaEventKind::Payment => (policy.next_pay_date.clone(), policy.remaining_years),
        AiaEventKind::Withdrawal => (None, None),
    };

    let now = now_timestamp();
    let id = store::insert_event(
        &mut tx,
        body.policy_id,
        validated.kind.as_str(),
        &validated.event_date,
        validated.amount_usd,
        body.note.as_deref(),
        prev_next_pay_date.as_deref(),
        prev_remaining_years,
        &now,
    )
    .await?;

    match validated.kind {
        AiaEventKind::Payment => {
            let current_next = policy.next_pay_date.as_deref().map(parse_day).transpose()?;
            let submitted_next = validated
                .next_pay_date
                .as_deref()
                .map(parse_day)
                .transpose()?;
            let outcome = apply_aia_payment(
                policy.premium_usd,
                policy.remaining_years,
                current_next,
                validated.amount_usd,
                submitted_next,
            );
            store::apply_payment_to_policy(
                &mut tx,
                body.policy_id,
                outcome.premium_usd,
                outcome.remaining_years,
                outcome.next_pay_date.map(|date| date.to_string()),
                &now,
            )
            .await?;
        }
        AiaEventKind::Withdrawal => {
            let withdrew = apply_aia_withdrawal(policy.withdrew_usd, validated.amount_usd);
            store::apply_withdrawal_to_policy(&mut tx, body.policy_id, withdrew, &now).await?;
        }
    }

    tx.commit().await?;
    Ok((
        StatusCode::CREATED,
        Json(store::load_event(&state.pool, id).await?),
    ))
}

/// Deleting an event undoes it: the amount leaves its cumulative field and the
/// policy fields the event recorded snap back.
#[utoipa::path(
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
    let mut tx = state.pool.begin().await?;
    let event = store::event_row(&mut tx, id).await?;
    let policy = store::event_policy_row(&mut tx, event.policy_id).await?;

    match event.kind {
        AiaEventKind::Payment => {
            store::apply_payment_to_policy(
                &mut tx,
                event.policy_id,
                undo_aia_event_amount(policy.premium_usd, event.amount_usd),
                event.prev_remaining_years,
                event.prev_next_pay_date.clone(),
                &now_timestamp(),
            )
            .await?;
        }
        AiaEventKind::Withdrawal => {
            store::apply_withdrawal_to_policy(
                &mut tx,
                event.policy_id,
                undo_aia_event_amount(policy.withdrew_usd, event.amount_usd),
                &now_timestamp(),
            )
            .await?;
        }
    }

    store::delete_event(&mut tx, id).await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}
