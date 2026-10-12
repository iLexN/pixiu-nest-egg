use sqlx::Row;
use sqlx::SqlitePool;
use sqlx::sqlite::SqliteRow;

use crate::calc::{AiaPolicyFacts, SuggestionAiaPayment, ValidatedAiaPolicy, aia_balance_pct};
use crate::models::{AiaEvent, AiaEventKind, AiaPolicy};
use crate::store::{StoreError, now_timestamp};

/// `app_meta` key holding the manual USD→HKD rate (the workbook's
/// `Overview!N3` GOOGLEFINANCE cell, which the app cannot fetch).
pub const RATE_KEY: &str = "aia.usd_hkd_rate";

/// `app_meta` key holding the section's free-text note — the same facility
/// as the MPF page's `mpf.note`.
pub const NOTE_KEY: &str = "aia.note";

pub const AIA_SELECT: &str = "SELECT p.id, p.label, p.policy_no, p.next_pay_date, \
     p.premium_usd, p.value_usd, p.value_updated_at, p.remaining_years, \
     p.withdrew_usd, p.note, p.link, p.excluded, p.in_account, p.sort_order \
     FROM aia_policies p";

pub fn row_to_policy(row: &SqliteRow) -> Result<AiaPolicy, StoreError> {
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

fn row_to_event(row: &SqliteRow) -> Result<AiaEvent, StoreError> {
    let kind: String = row.try_get("kind")?;
    Ok(AiaEvent {
        id: row.try_get("id")?,
        policy_id: row.try_get("policy_id")?,
        kind: AiaEventKind::parse(&kind)
            .ok_or_else(|| StoreError::Corrupt(format!("stored event kind {kind} is not valid")))?,
        event_date: row.try_get("event_date")?,
        amount_usd: row.try_get("amount_usd")?,
        note: row.try_get("note")?,
        prev_next_pay_date: row.try_get("prev_next_pay_date")?,
        prev_remaining_years: row.try_get("prev_remaining_years")?,
    })
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

/// The stored row of one policy. `exec` may be the pool or a transaction —
/// `POST /api/aia/events` reads it inside its own transaction.
pub async fn policy_row<'e, E>(exec: E, id: i64) -> Result<SqliteRow, StoreError>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    sqlx::query(sqlx::AssertSqlSafe(format!("{AIA_SELECT} WHERE p.id = ?")))
        .bind(id)
        .fetch_optional(exec)
        .await?
        .ok_or_else(|| StoreError::NotFound(format!("aia policy {id} not found")))
}

pub async fn load_policy(pool: &SqlitePool, id: i64) -> Result<AiaPolicy, StoreError> {
    row_to_policy(&policy_row(pool, id).await?)
}

pub async fn load_all_policies(pool: &SqlitePool) -> Result<Vec<AiaPolicy>, StoreError> {
    let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
        "{AIA_SELECT} ORDER BY p.sort_order, p.id"
    )))
    .fetch_all(pool)
    .await?;
    rows.iter()
        .map(row_to_policy)
        .collect::<Result<Vec<_>, _>>()
}

pub async fn load_event(pool: &SqlitePool, id: i64) -> Result<AiaEvent, StoreError> {
    let row = sqlx::query(sqlx::AssertSqlSafe(format!(
        "{EVENT_SELECT} WHERE e.id = ?"
    )))
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| StoreError::NotFound(format!("aia event {id} not found")))?;
    row_to_event(&row)
}

/// The stored row of one event, inside the caller's transaction.
pub async fn event_row(conn: &mut sqlx::SqliteConnection, id: i64) -> Result<AiaEvent, StoreError> {
    let row = sqlx::query(sqlx::AssertSqlSafe(format!(
        "{EVENT_SELECT} WHERE e.id = ?"
    )))
    .bind(id)
    .fetch_optional(&mut *conn)
    .await?
    .ok_or_else(|| StoreError::NotFound(format!("aia event {id} not found")))?;
    row_to_event(&row)
}

/// The stored row of the event's policy, inside the caller's transaction.
pub async fn event_policy_row(
    conn: &mut sqlx::SqliteConnection,
    id: i64,
) -> Result<AiaPolicy, StoreError> {
    let row = sqlx::query(sqlx::AssertSqlSafe(format!("{AIA_SELECT} WHERE p.id = ?")))
        .bind(id)
        .fetch_one(&mut *conn)
        .await?;
    row_to_policy(&row)
}

pub async fn next_sort_order(pool: &SqlitePool) -> Result<i64, StoreError> {
    Ok(
        sqlx::query_scalar("SELECT COALESCE(MAX(sort_order), 0) + 1 FROM aia_policies")
            .fetch_one(pool)
            .await?,
    )
}

pub async fn insert_policy(
    pool: &SqlitePool,
    policy: &ValidatedAiaPolicy,
    sort_order: i64,
) -> Result<i64, StoreError> {
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

pub async fn update_policy(
    pool: &SqlitePool,
    id: i64,
    validated: &ValidatedAiaPolicy,
    value_updated_at: &Option<String>,
) -> Result<(), StoreError> {
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
    .bind(value_updated_at)
    .bind(validated.remaining_years)
    .bind(validated.withdrew_usd)
    .bind(validated.note.as_deref())
    .bind(validated.link.as_deref())
    .bind(validated.excluded as i64)
    .bind(validated.in_account as i64)
    .bind(now_timestamp())
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Deleting a policy removes its events: nothing outside them references a
/// policy. Returns the policy rows deleted, so the handler can tell "not
/// found" apart from success.
pub async fn delete_with_events(
    conn: &mut sqlx::SqliteConnection,
    id: i64,
) -> Result<u64, StoreError> {
    sqlx::query("DELETE FROM aia_events WHERE policy_id = ?")
        .bind(id)
        .execute(&mut *conn)
        .await?;
    Ok(sqlx::query("DELETE FROM aia_policies WHERE id = ?")
        .bind(id)
        .execute(&mut *conn)
        .await?
        .rows_affected())
}

pub async fn list_events(
    pool: &SqlitePool,
    policy_id: Option<i64>,
) -> Result<Vec<AiaEvent>, StoreError> {
    let mut sql = EVENT_SELECT.to_string();
    if policy_id.is_some() {
        sql.push_str(" WHERE e.policy_id = ?");
    }
    sql.push_str(" ORDER BY e.event_date, e.id");
    let mut statement = sqlx::query(sqlx::AssertSqlSafe(sql));
    if let Some(policy_id) = policy_id {
        statement = statement.bind(policy_id);
    }
    statement
        .fetch_all(pool)
        .await?
        .iter()
        .map(row_to_event)
        .collect::<Result<Vec<_>, _>>()
}

pub async fn load_policy_events(
    pool: &SqlitePool,
    policy_id: i64,
) -> Result<Vec<AiaEvent>, StoreError> {
    let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
        "{EVENT_SELECT} WHERE e.policy_id = ? ORDER BY e.event_date, e.id"
    )))
    .bind(policy_id)
    .fetch_all(pool)
    .await?;
    rows.iter().map(row_to_event).collect::<Result<Vec<_>, _>>()
}

/// Insert the event row inside the caller's transaction, recording the
/// policy fields it is about to touch for undo.
#[allow(clippy::too_many_arguments)]
pub async fn insert_event(
    conn: &mut sqlx::SqliteConnection,
    policy_id: i64,
    kind: &str,
    event_date: &str,
    amount_usd: f64,
    note: Option<&str>,
    prev_next_pay_date: Option<&str>,
    prev_remaining_years: Option<f64>,
    now: &str,
) -> Result<i64, StoreError> {
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO aia_events (policy_id, kind, event_date, amount_usd, note, \
         prev_next_pay_date, prev_remaining_years, created_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(policy_id)
    .bind(kind)
    .bind(event_date)
    .bind(amount_usd)
    .bind(note)
    .bind(prev_next_pay_date)
    .bind(prev_remaining_years)
    .bind(now)
    .fetch_one(&mut *conn)
    .await?;
    Ok(id)
}

/// Apply a payment's outcome to the policy, inside the caller's transaction.
pub async fn apply_payment_to_policy(
    conn: &mut sqlx::SqliteConnection,
    policy_id: i64,
    premium_usd: f64,
    remaining_years: Option<f64>,
    next_pay_date: Option<String>,
    now: &str,
) -> Result<(), StoreError> {
    sqlx::query(
        "UPDATE aia_policies SET premium_usd = ?, remaining_years = ?, \
         next_pay_date = ?, updated_at = ? WHERE id = ?",
    )
    .bind(premium_usd)
    .bind(remaining_years)
    .bind(next_pay_date)
    .bind(now)
    .bind(policy_id)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// Apply a withdrawal to the policy, inside the caller's transaction.
pub async fn apply_withdrawal_to_policy(
    conn: &mut sqlx::SqliteConnection,
    policy_id: i64,
    withdrew_usd: f64,
    now: &str,
) -> Result<(), StoreError> {
    sqlx::query("UPDATE aia_policies SET withdrew_usd = ?, updated_at = ? WHERE id = ?")
        .bind(withdrew_usd)
        .bind(now)
        .bind(policy_id)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

pub async fn delete_event(conn: &mut sqlx::SqliteConnection, id: i64) -> Result<(), StoreError> {
    sqlx::query("DELETE FROM aia_events WHERE id = ?")
        .bind(id)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

/// AIA premium payments in `[first, next)` as suggestion sources.
pub async fn suggestion_payments(
    pool: &SqlitePool,
    first: &str,
    next: &str,
) -> Result<Vec<SuggestionAiaPayment>, StoreError> {
    let rows = sqlx::query(
        "SELECT e.id, e.event_date, e.amount_usd, p.label FROM aia_events e \
         JOIN aia_policies p ON p.id = e.policy_id \
         WHERE e.kind = 'payment' AND e.event_date >= ? AND e.event_date < ?",
    )
    .bind(first)
    .bind(next)
    .fetch_all(pool)
    .await?;
    let mut aia_payments = Vec::with_capacity(rows.len());
    for row in &rows {
        let event_date: String = row.try_get("event_date")?;
        aia_payments.push(SuggestionAiaPayment {
            id: row.try_get("id")?,
            date: chrono::NaiveDate::parse_from_str(&event_date, "%Y-%m-%d").map_err(|_| {
                StoreError::Corrupt(format!("stored date {event_date} is not valid"))
            })?,
            amount_usd: row.try_get("amount_usd")?,
            policy: row.try_get("label")?,
        });
    }
    Ok(aia_payments)
}
