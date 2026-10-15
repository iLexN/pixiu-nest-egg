//! Family deposits (家人 定期): the same registry and 收訖 lifecycle as
//! `deposits.rs`, but hard-isolated — this module must never credit a cash
//! row or record a month item, because family money is not the user's money.

use chrono::Datelike;
use sqlx::Row;
use sqlx::SqlitePool;
use sqlx::sqlite::SqliteRow;

use crate::calc::{ValidatedDeposit, deposit_total};
use crate::models::{DepositStatus, FamilyDeposit};
use crate::store::{StoreError, now_timestamp};

/// `app_meta` key prefix for the per-holder note: `family.note.<holder>`.
pub const FAMILY_NOTE_PREFIX: &str = "family.note.";

pub fn note_key(holder: &str) -> String {
    format!("{FAMILY_NOTE_PREFIX}{holder}")
}

pub const FAMILY_DEPOSIT_COLUMNS: &str = "id, holder, label, bank, principal, interest, \
     start_date, end_date, received_at, note, sort_order";

/// Like `row_to_deposit` minus the bank-in columns family deposits don't have;
/// `status` derives from `received_at` only (never auto-ended by date).
pub fn row_to_family_deposit(row: &SqliteRow) -> Result<FamilyDeposit, StoreError> {
    let end_date: String = row.try_get("end_date")?;
    let parsed = chrono::NaiveDate::parse_from_str(&end_date, "%Y-%m-%d")
        .map_err(|_| StoreError::Corrupt(format!("stored end_date {end_date} is not valid")))?;
    let principal: Option<f64> = row.try_get("principal")?;
    let interest: Option<f64> = row.try_get("interest")?;
    let received_at: Option<String> = row.try_get("received_at")?;
    Ok(FamilyDeposit {
        id: row.try_get("id")?,
        holder: row.try_get("holder")?,
        label: row.try_get("label")?,
        bank: row.try_get("bank")?,
        principal,
        interest,
        start_date: row.try_get("start_date")?,
        end_date,
        received_at: received_at.clone(),
        note: row.try_get("note")?,
        sort_order: row.try_get("sort_order")?,
        total: deposit_total(principal, interest),
        status: if received_at.is_some() {
            DepositStatus::End
        } else {
            DepositStatus::Active
        },
        end_year: parsed.year(),
        end_month: parsed.month(),
    })
}

/// `received`: Some(false) lists only 未收訖 rows, Some(true) only received.
pub async fn list(
    pool: &SqlitePool,
    received: Option<bool>,
    holder: Option<&str>,
    year: Option<i32>,
    descending: bool,
) -> Result<Vec<FamilyDeposit>, StoreError> {
    let mut sql = format!("SELECT {FAMILY_DEPOSIT_COLUMNS} FROM family_deposits");
    let mut conditions: Vec<&str> = Vec::new();
    if let Some(received) = received {
        if received {
            conditions.push("received_at IS NOT NULL");
        } else {
            conditions.push("received_at IS NULL");
        }
    }
    if holder.is_some() {
        conditions.push("holder = ?");
    }
    if year.is_some() {
        conditions.push("end_date >= ? AND end_date <= ?");
    }
    if !conditions.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&conditions.join(" AND "));
    }
    sql.push_str(if descending {
        " ORDER BY end_date DESC, sort_order DESC, id DESC"
    } else {
        " ORDER BY end_date ASC, sort_order ASC, id ASC"
    });

    let mut statement = sqlx::query(sqlx::AssertSqlSafe(sql));
    if let Some(holder) = holder {
        statement = statement.bind(holder);
    }
    if let Some(year) = year {
        statement = statement
            .bind(format!("{year:04}-01-01"))
            .bind(format!("{year:04}-12-31"));
    }

    statement
        .fetch_all(pool)
        .await?
        .iter()
        .map(row_to_family_deposit)
        .collect::<Result<Vec<_>, _>>()
}

pub async fn next_sort_order(pool: &SqlitePool) -> Result<i64, StoreError> {
    Ok(
        sqlx::query_scalar("SELECT COALESCE(MAX(sort_order), 0) + 1 FROM family_deposits")
            .fetch_one(pool)
            .await?,
    )
}

/// Unlike `deposits`, a past end_date is never auto-received — the user
/// confirms receipt explicitly with 收訖.
pub async fn insert_family_deposit(
    pool: &SqlitePool,
    holder: &str,
    validated: &ValidatedDeposit,
    note: Option<&str>,
    sort_order: i64,
) -> Result<i64, StoreError> {
    let now = now_timestamp();
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO family_deposits (holder, label, bank, principal, interest, start_date, \
         end_date, note, sort_order, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(holder)
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
    .fetch_one(pool)
    .await?;
    Ok(id)
}

pub async fn update_family_deposit(
    pool: &SqlitePool,
    id: i64,
    holder: &str,
    validated: &ValidatedDeposit,
    note: &Option<String>,
) -> Result<(), StoreError> {
    sqlx::query(
        "UPDATE family_deposits SET holder = ?, label = ?, bank = ?, principal = ?, \
         interest = ?, start_date = ?, end_date = ?, note = ?, updated_at = ? WHERE id = ?",
    )
    .bind(holder)
    .bind(validated.label.as_deref())
    .bind(validated.bank.as_deref())
    .bind(validated.principal)
    .bind(validated.interest)
    .bind(validated.start_date.as_deref())
    .bind(&validated.end_date)
    .bind(note)
    .bind(now_timestamp())
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Rows deleted, so the handler can tell "not found" apart from success.
pub async fn delete_family_deposit(pool: &SqlitePool, id: i64) -> Result<u64, StoreError> {
    Ok(sqlx::query("DELETE FROM family_deposits WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?
        .rows_affected())
}

pub async fn mark_received(
    pool: &SqlitePool,
    id: i64,
    received_at: &str,
    interest: Option<f64>,
) -> Result<(), StoreError> {
    sqlx::query(
        "UPDATE family_deposits SET received_at = ?, interest = ?, updated_at = ? WHERE id = ?",
    )
    .bind(received_at)
    .bind(interest)
    .bind(now_timestamp())
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn load_one(pool: &SqlitePool, id: i64) -> Result<FamilyDeposit, StoreError> {
    let row = sqlx::query(sqlx::AssertSqlSafe(format!(
        "SELECT {FAMILY_DEPOSIT_COLUMNS} FROM family_deposits WHERE id = ?"
    )))
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| StoreError::NotFound(format!("family deposit {id} not found")))?;
    row_to_family_deposit(&row)
}

pub async fn load_all(pool: &SqlitePool) -> Result<Vec<FamilyDeposit>, StoreError> {
    let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
        "SELECT {FAMILY_DEPOSIT_COLUMNS} FROM family_deposits ORDER BY sort_order, id"
    )))
    .fetch_all(pool)
    .await?;
    rows.iter()
        .map(row_to_family_deposit)
        .collect::<Result<Vec<_>, _>>()
}
