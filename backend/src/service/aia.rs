//! AIA policy events: a payment/withdrawal event applies its derived effects
//! to the policy in the same transaction it is recorded in, and deleting it
//! undoes them — the same apply/undo shape as `service::deposits`.

use serde::Serialize;
use sqlx::SqlitePool;

use crate::calc::{
    AiaPolicyFacts, AiaTotals, ValidatedAiaEvent, aia_totals, apply_aia_payment,
    apply_aia_withdrawal, next_premium_due, undo_aia_event_amount,
};
use crate::models::{AiaEvent, AiaEventKind, AiaPolicy};
use crate::service::ServiceError;
use crate::store::{StoreError, aia, now_timestamp, stored_date, today};

/// A stored `next_pay_date`/`event_date` the DB could not parse is a data
/// problem, not a request one — `Corrupt`, which the route maps to 409.
fn parse_day(date: &str) -> Result<chrono::NaiveDate, StoreError> {
    stored_date("date", date)
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

/// The AIA page read model — every figure derived from the stored rows on
/// each read.
pub async fn summary(pool: &SqlitePool) -> Result<AiaSummaryResponse, StoreError> {
    let today = today();
    let policies = aia::load_all_policies(pool).await?;
    let rate = crate::store::fx::usd_rate(pool).await?;
    let facts: Vec<AiaPolicyFacts> = policies.iter().map(aia::facts_of).collect();
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
        let events = aia::load_policy_events(pool, policy.id).await?;
        with_events.push(PolicyWithEvents { policy, events });
    }

    let note = crate::store::settings::aia_note(pool).await?;
    Ok(AiaSummaryResponse {
        today: today.to_string(),
        rate,
        next_premium_due: next_due.map(|date| date.to_string()),
        note,
        totals,
        policies: with_events,
    })
}

/// Record an event and apply its effects to the policy in one transaction: a
/// payment snapshots the policy's `next_pay_date`/`remaining_years` onto the
/// event (so a later delete can restore them) then adds the amount and rolls
/// the due date forward; a withdrawal just accumulates. `Err(NotFound)` while
/// `policy_id` names no policy.
pub async fn create_event(
    pool: &SqlitePool,
    policy_id: i64,
    validated: &ValidatedAiaEvent,
    note: Option<&str>,
) -> Result<i64, ServiceError> {
    let mut tx = pool.begin().await.map_err(StoreError::from)?;
    let policy = aia::row_to_policy(&aia::policy_row(&mut *tx, policy_id).await?)?;

    let (prev_next_pay_date, prev_remaining_years) = match validated.kind {
        AiaEventKind::Payment => (policy.next_pay_date.clone(), policy.remaining_years),
        AiaEventKind::Withdrawal => (None, None),
    };

    let now = now_timestamp();
    let id = aia::insert_event(
        &mut tx,
        policy_id,
        validated.kind.as_str(),
        &validated.event_date,
        validated.amount_usd,
        note,
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
            aia::apply_payment_to_policy(
                &mut tx,
                policy_id,
                outcome.premium_usd,
                outcome.remaining_years,
                outcome.next_pay_date.map(|date| date.to_string()),
                &now,
            )
            .await?;
        }
        AiaEventKind::Withdrawal => {
            let withdrew = apply_aia_withdrawal(policy.withdrew_usd, validated.amount_usd);
            aia::apply_withdrawal_to_policy(&mut tx, policy_id, withdrew, &now).await?;
        }
    }

    tx.commit().await.map_err(StoreError::from)?;
    Ok(id)
}

/// Delete an event and undo it in one transaction: the amount leaves its
/// cumulative field and the policy fields the event recorded snap back.
/// `Err(NotFound)` while `id` names no event.
pub async fn remove_event(pool: &SqlitePool, id: i64) -> Result<(), ServiceError> {
    let mut tx = pool.begin().await.map_err(StoreError::from)?;
    let event = aia::event_row(&mut tx, id).await?;
    let policy = aia::event_policy_row(&mut tx, event.policy_id).await?;
    let now = now_timestamp();

    match event.kind {
        AiaEventKind::Payment => {
            aia::apply_payment_to_policy(
                &mut tx,
                event.policy_id,
                undo_aia_event_amount(policy.premium_usd, event.amount_usd),
                event.prev_remaining_years,
                event.prev_next_pay_date.clone(),
                &now,
            )
            .await?;
        }
        AiaEventKind::Withdrawal => {
            aia::apply_withdrawal_to_policy(
                &mut tx,
                event.policy_id,
                undo_aia_event_amount(policy.withdrew_usd, event.amount_usd),
                &now,
            )
            .await?;
        }
    }

    aia::delete_event(&mut tx, id).await?;
    tx.commit().await.map_err(StoreError::from)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn policy(pool: &SqlitePool) -> i64 {
        let now = crate::store::now_timestamp();
        sqlx::query_scalar(
            "INSERT INTO aia_policies (label, policy_no, next_pay_date, premium_usd, value_usd, \
             value_updated_at, remaining_years, withdrew_usd, note, link, excluded, in_account, \
             sort_order, created_at, updated_at) \
             VALUES ('P', NULL, '2030-01-01', 1000.0, 0.0, NULL, 5.0, 0.0, NULL, NULL, 0, 0, 0, ?, ?) \
             RETURNING id",
        )
        .bind(&now)
        .bind(&now)
        .fetch_one(pool)
        .await
        .expect("policy")
    }

    async fn figures(pool: &SqlitePool, id: i64) -> (f64, Option<f64>, Option<String>) {
        let policy = aia::row_to_policy(&aia::policy_row(pool, id).await.unwrap()).unwrap();
        (
            policy.premium_usd,
            policy.remaining_years,
            policy.next_pay_date,
        )
    }

    #[tokio::test]
    async fn payment_applies_its_effects_and_delete_undoes_them() {
        let pool = crate::db::connect_memory().await.expect("memory db");
        let policy_id = policy(&pool).await;

        let validated = ValidatedAiaEvent {
            kind: AiaEventKind::Payment,
            event_date: "2026-01-10".to_string(),
            amount_usd: 100.0,
            next_pay_date: None,
        };
        let id = create_event(&pool, policy_id, &validated, None)
            .await
            .expect("event");

        let (premium, years, next) = figures(&pool, policy_id).await;
        assert_eq!(premium, 1100.0);
        assert_eq!(years, Some(4.0));
        assert_eq!(next.as_deref(), Some("2031-01-01"));

        remove_event(&pool, id).await.expect("remove");
        let (premium, years, next) = figures(&pool, policy_id).await;
        assert_eq!(premium, 1000.0);
        assert_eq!(years, Some(5.0));
        assert_eq!(next.as_deref(), Some("2030-01-01"));
    }
}
