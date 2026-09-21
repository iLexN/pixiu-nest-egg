use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqliteRow;
use sqlx::{Row, SqlitePool};

use super::{now_timestamp, today, AppState};
use crate::calc::{
    aia_balance_pct, aia_totals, apply_aia_payment, apply_aia_withdrawal, next_premium_due,
    undo_aia_event_amount, validate_aia_event, validate_aia_policy, AiaEventInput, AiaPolicyFacts,
    AiaPolicyInput, AiaTotals, ValidatedAiaPolicy,
};
use crate::error::ApiError;
use crate::models::{
    AiaEvent, AiaEventKind, AiaPolicy, AiaPolicyPatch, AiaRatePatch, NewAiaEvent, NewAiaPolicy,
};

/// `app_meta` key holding the manual USD→HKD rate (the workbook's
/// `Overview!N3` GOOGLEFINANCE cell, which the app cannot fetch).
pub const RATE_KEY: &str = "aia.usd_hkd_rate";

pub const AIA_SELECT: &str = "SELECT p.id, p.label, p.policy_no, p.next_pay_date, \
     p.premium_usd, p.value_usd, p.value_updated_at, p.remaining_years, \
     p.withdrew_usd, p.note, p.link, p.excluded, p.in_account, p.sort_order \
     FROM aia_policies p";

pub fn row_to_policy(row: &SqliteRow) -> Result<AiaPolicy, ApiError> {
    let premium_usd: f64 = row.try_get("premium_usd")?;
    let value_usd: f64 = row.try_get("value_usd")?;
    let withdrew_usd: f64 = row.try_get("withdrew_usd")?;
    Ok(AiaPolicy {
        id: row.try_get("id")?,
        label: row.try_get("label")?,
        policy_no: row.try_get("policy_no")?,
        next_pay_date: row.try_get("next_pay_date")?,
        premium_usd,
        value_usd,
        value_updated_at: row.try_get("value_updated_at")?,
        remaining_years: row.try_get("remaining_years")?,
        withdrew_usd,
        note: row.try_get("note")?,
        link: row.try_get("link")?,
        excluded: row.try_get::<i64, _>("excluded")? != 0,
        in_account: row.try_get::<i64, _>("in_account")? != 0,
        sort_order: row.try_get("sort_order")?,
        balance_pct: aia_balance_pct(value_usd, withdrew_usd, premium_usd),
    })
}

const EVENT_SELECT: &str = "SELECT e.id, e.policy_id, e.kind, e.event_date, e.amount_usd, \
     e.note, e.prev_next_pay_date, e.prev_remaining_years FROM aia_events e";

fn row_to_event(row: &SqliteRow) -> Result<AiaEvent, ApiError> {
    let kind: String = row.try_get("kind")?;
    Ok(AiaEvent {
        id: row.try_get("id")?,
        policy_id: row.try_get("policy_id")?,
        kind: AiaEventKind::parse(&kind)
            .ok_or_else(|| ApiError::Conflict(format!("stored event kind {kind} is not valid")))?,
        event_date: row.try_get("event_date")?,
        amount_usd: row.try_get("amount_usd")?,
        note: row.try_get("note")?,
        prev_next_pay_date: row.try_get("prev_next_pay_date")?,
        prev_remaining_years: row.try_get("prev_remaining_years")?,
    })
}

fn parse_day(date: &str) -> Result<chrono::NaiveDate, ApiError> {
    chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|_| ApiError::Conflict(format!("stored date {date} is not valid")))
}

pub fn facts_of(policy: &AiaPolicy) -> AiaPolicyFacts {
    AiaPolicyFacts {
        premium_usd: policy.premium_usd,
        value_usd: policy.value_usd,
        withdrew_usd: policy.withdrew_usd,
        excluded: policy.excluded,
        in_account: policy.in_account,
    }
}

pub async fn load_policy(pool: &SqlitePool, id: i64) -> Result<AiaPolicy, ApiError> {
    let row = sqlx::query(&format!("{AIA_SELECT} WHERE p.id = ?"))
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("aia policy {id} not found")))?;
    row_to_policy(&row)
}

pub async fn load_all_policies(pool: &SqlitePool) -> Result<Vec<AiaPolicy>, ApiError> {
    let rows = sqlx::query(&format!("{AIA_SELECT} ORDER BY p.sort_order, p.id"))
        .fetch_all(pool)
        .await?;
    rows.iter()
        .map(row_to_policy)
        .collect::<Result<Vec<_>, _>>()
}

pub async fn load_event(pool: &SqlitePool, id: i64) -> Result<AiaEvent, ApiError> {
    let row = sqlx::query(&format!("{EVENT_SELECT} WHERE e.id = ?"))
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("aia event {id} not found")))?;
    row_to_event(&row)
}

pub async fn list(State(state): State<AppState>) -> Result<Json<Vec<AiaPolicy>>, ApiError> {
    load_all_policies(&state.pool).await.map(Json)
}

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

    let sort_order: i64 =
        sqlx::query_scalar("SELECT COALESCE(MAX(sort_order), 0) + 1 FROM aia_policies")
            .fetch_one(&state.pool)
            .await?;
    let id = insert_policy(&state.pool, &validated, sort_order).await?;
    Ok((
        StatusCode::CREATED,
        Json(load_policy(&state.pool, id).await?),
    ))
}

pub async fn insert_policy(
    pool: &SqlitePool,
    policy: &ValidatedAiaPolicy,
    sort_order: i64,
) -> Result<i64, ApiError> {
    let now = now_timestamp();
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO aia_policies (label, policy_no, next_pay_date, premium_usd, value_usd, \
         value_updated_at, remaining_years, withdrew_usd, note, link, excluded, in_account, \
         sort_order, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(&policy.label)
    .bind(policy.policy_no.as_deref())
    .bind(policy.next_pay_date.as_deref())
    .bind(policy.premium_usd)
    .bind(policy.value_usd)
    .bind((policy.value_usd > 0.0).then_some(now.as_str()))
    .bind(policy.remaining_years)
    .bind(policy.withdrew_usd)
    .bind(policy.note.as_deref())
    .bind(policy.link.as_deref())
    .bind(policy.excluded as i64)
    .bind(policy.in_account as i64)
    .bind(sort_order)
    .bind(&now)
    .bind(&now)
    .fetch_one(pool)
    .await?;
    Ok(id)
}

pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(patch): Json<AiaPolicyPatch>,
) -> Result<Json<AiaPolicy>, ApiError> {
    let existing = load_policy(&state.pool, id).await?;

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

    sqlx::query(
        "UPDATE aia_policies SET label = ?, policy_no = ?, next_pay_date = ?, premium_usd = ?, \
         value_usd = ?, value_updated_at = ?, remaining_years = ?, withdrew_usd = ?, note = ?, \
         link = ?, excluded = ?, in_account = ?, updated_at = ? WHERE id = ?",
    )
    .bind(&validated.label)
    .bind(validated.policy_no.as_deref())
    .bind(validated.next_pay_date.as_deref())
    .bind(validated.premium_usd)
    .bind(validated.value_usd)
    .bind(&value_updated_at)
    .bind(validated.remaining_years)
    .bind(validated.withdrew_usd)
    .bind(validated.note.as_deref())
    .bind(validated.link.as_deref())
    .bind(validated.excluded as i64)
    .bind(validated.in_account as i64)
    .bind(now_timestamp())
    .bind(id)
    .execute(&state.pool)
    .await?;

    Ok(Json(load_policy(&state.pool, id).await?))
}

/// Deleting a policy removes its events: nothing outside them references a
/// policy, so delete is always permitted.
pub async fn remove(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    let mut tx = state.pool.begin().await?;
    sqlx::query("DELETE FROM aia_events WHERE policy_id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    let result = sqlx::query("DELETE FROM aia_policies WHERE id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound(format!("aia policy {id} not found")));
    }
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

/// A policy with its payment/withdrawal history attached.
#[derive(Debug, Serialize)]
pub struct PolicyWithEvents {
    #[serde(flatten)]
    pub policy: AiaPolicy,
    pub events: Vec<AiaEvent>,
}

#[derive(Debug, Serialize)]
pub struct AiaSummaryResponse {
    pub today: String,
    /// The manual USD→HKD rate; HKD figures are absent while it is unset.
    pub rate: Option<f64>,
    /// Earliest premium-due date still ahead — the premium the user owes next.
    pub next_premium_due: Option<String>,
    pub totals: AiaTotals,
    pub policies: Vec<PolicyWithEvents>,
}

/// Every figure here is derived from the stored rows on each read.
pub async fn summary(State(state): State<AppState>) -> Result<Json<AiaSummaryResponse>, ApiError> {
    let today = today();
    let policies = load_all_policies(&state.pool).await?;
    let rate = crate::mpf::meta_get(&state.pool, RATE_KEY)
        .await?
        .and_then(|raw| raw.parse::<f64>().ok());
    let facts: Vec<AiaPolicyFacts> = policies.iter().map(facts_of).collect();
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
        let events = load_policy_events(&state.pool, policy.id).await?;
        with_events.push(PolicyWithEvents { policy, events });
    }

    Ok(Json(AiaSummaryResponse {
        today: today.to_string(),
        rate,
        next_premium_due: next_due.map(|date| date.to_string()),
        totals,
        policies: with_events,
    }))
}

#[derive(Debug, Serialize)]
pub struct RateResponse {
    pub rate: Option<f64>,
}

/// The workbook keeps a live GOOGLEFINANCE rate in `Overview!N3`; the app keeps
/// a manual copy in `app_meta` — same facility as the MPF note.
pub async fn update_rate(
    State(state): State<AppState>,
    Json(body): Json<AiaRatePatch>,
) -> Result<Json<RateResponse>, ApiError> {
    if let Some(rate) = body.rate {
        if !rate.is_finite() || rate <= 0.0 {
            return Err(ApiError::field("rate", "rate must be a positive number"));
        }
    }
    let value = body.rate.map(|rate| rate.to_string());
    crate::mpf::meta_put(&state.pool, RATE_KEY, value.as_deref()).await?;
    let rate = crate::mpf::meta_get(&state.pool, RATE_KEY)
        .await?
        .and_then(|raw| raw.parse::<f64>().ok());
    Ok(Json(RateResponse { rate }))
}

#[derive(Debug, Deserialize)]
pub struct EventListQuery {
    pub policy_id: Option<i64>,
}

pub async fn list_events(
    State(state): State<AppState>,
    Query(query): Query<EventListQuery>,
) -> Result<Json<Vec<AiaEvent>>, ApiError> {
    let mut sql = EVENT_SELECT.to_string();
    if query.policy_id.is_some() {
        sql.push_str(" WHERE e.policy_id = ?");
    }
    sql.push_str(" ORDER BY e.event_date, e.id");
    let mut statement = sqlx::query(&sql);
    if let Some(policy_id) = query.policy_id {
        statement = statement.bind(policy_id);
    }
    statement
        .fetch_all(&state.pool)
        .await?
        .iter()
        .map(row_to_event)
        .collect::<Result<Vec<_>, _>>()
        .map(Json)
}

pub async fn load_policy_events(
    pool: &SqlitePool,
    policy_id: i64,
) -> Result<Vec<AiaEvent>, ApiError> {
    let rows = sqlx::query(&format!(
        "{EVENT_SELECT} WHERE e.policy_id = ? ORDER BY e.event_date, e.id"
    ))
    .bind(policy_id)
    .fetch_all(pool)
    .await?;
    rows.iter().map(row_to_event).collect::<Result<Vec<_>, _>>()
}

/// Recording an event is one transaction: snapshot the policy fields it will
/// touch onto the event row, insert the row, then apply the update — a payment
/// grows the premium, counts down a remaining year and moves the next due
/// date; a withdrawal grows the withdrew figure.
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
    let row = sqlx::query(&format!("{AIA_SELECT} WHERE p.id = ?"))
        .bind(body.policy_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("aia policy {} not found", body.policy_id)))?;
    let policy = row_to_policy(&row)?;

    let (prev_next_pay_date, prev_remaining_years) = match validated.kind {
        AiaEventKind::Payment => (policy.next_pay_date.clone(), policy.remaining_years),
        AiaEventKind::Withdrawal => (None, None),
    };

    let now = now_timestamp();
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO aia_events (policy_id, kind, event_date, amount_usd, note, \
         prev_next_pay_date, prev_remaining_years, created_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(body.policy_id)
    .bind(validated.kind.as_str())
    .bind(&validated.event_date)
    .bind(validated.amount_usd)
    .bind(body.note.as_deref())
    .bind(prev_next_pay_date.as_deref())
    .bind(prev_remaining_years)
    .bind(&now)
    .fetch_one(&mut *tx)
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
            sqlx::query(
                "UPDATE aia_policies SET premium_usd = ?, remaining_years = ?, \
                 next_pay_date = ?, updated_at = ? WHERE id = ?",
            )
            .bind(outcome.premium_usd)
            .bind(outcome.remaining_years)
            .bind(outcome.next_pay_date.map(|date| date.to_string()))
            .bind(&now)
            .bind(body.policy_id)
            .execute(&mut *tx)
            .await?;
        }
        AiaEventKind::Withdrawal => {
            let withdrew = apply_aia_withdrawal(policy.withdrew_usd, validated.amount_usd);
            sqlx::query("UPDATE aia_policies SET withdrew_usd = ?, updated_at = ? WHERE id = ?")
                .bind(withdrew)
                .bind(&now)
                .bind(body.policy_id)
                .execute(&mut *tx)
                .await?;
        }
    }

    tx.commit().await?;
    Ok((
        StatusCode::CREATED,
        Json(load_event(&state.pool, id).await?),
    ))
}

/// Deleting an event undoes it: the amount leaves its cumulative field and the
/// policy fields the event recorded snap back.
pub async fn remove_event(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    let mut tx = state.pool.begin().await?;
    let row = sqlx::query(&format!("{EVENT_SELECT} WHERE e.id = ?"))
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("aia event {id} not found")))?;
    let event = row_to_event(&row)?;

    let policy_row = sqlx::query(&format!("{AIA_SELECT} WHERE p.id = ?"))
        .bind(event.policy_id)
        .fetch_one(&mut *tx)
        .await?;
    let policy = row_to_policy(&policy_row)?;

    match event.kind {
        AiaEventKind::Payment => {
            sqlx::query(
                "UPDATE aia_policies SET premium_usd = ?, remaining_years = ?, \
                 next_pay_date = ?, updated_at = ? WHERE id = ?",
            )
            .bind(undo_aia_event_amount(policy.premium_usd, event.amount_usd))
            .bind(event.prev_remaining_years)
            .bind(event.prev_next_pay_date.as_deref())
            .bind(now_timestamp())
            .bind(event.policy_id)
            .execute(&mut *tx)
            .await?;
        }
        AiaEventKind::Withdrawal => {
            sqlx::query("UPDATE aia_policies SET withdrew_usd = ?, updated_at = ? WHERE id = ?")
                .bind(undo_aia_event_amount(policy.withdrew_usd, event.amount_usd))
                .bind(now_timestamp())
                .bind(event.policy_id)
                .execute(&mut *tx)
                .await?;
        }
    }

    sqlx::query("DELETE FROM aia_events WHERE id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}
