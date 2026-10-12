use chrono::Datelike;
use sqlx::Row;
use sqlx::SqlitePool;
use sqlx::sqlite::SqliteRow;

use crate::calc::ValidatedDeposit;
use crate::calc::{DepositFacts, SuggestionDeposit, deposit_total};
use crate::models::{Deposit, DepositStatus, NewDeposit};
use crate::store::{StoreError, now_timestamp, today};

pub const DEPOSIT_COLUMNS: &str = "id, label, bank, principal, rate, interest, start_date, \
     end_date, received_at, credited_asset_id, credited_amount, note1, note2, sort_order";

pub fn row_to_deposit(row: &SqliteRow, _today: chrono::NaiveDate) -> Result<Deposit, StoreError> {
    let end_date: String = row.try_get("end_date")?;
    let parsed = chrono::NaiveDate::parse_from_str(&end_date, "%Y-%m-%d")
        .map_err(|_| StoreError::Corrupt(format!("stored end_date {end_date} is not valid")))?;
    let principal: Option<f64> = row.try_get("principal")?;
    let interest: Option<f64> = row.try_get("interest")?;
    let received_at: Option<String> = row.try_get("received_at")?;
    Ok(Deposit {
        id: row.try_get("id")?,
        label: row.try_get("label")?,
        bank: row.try_get("bank")?,
        principal,
        rate: row.try_get("rate")?,
        interest,
        start_date: row.try_get("start_date")?,
        end_date,
        received_at: received_at.clone(),
        note1: row.try_get("note1")?,
        note2: row.try_get("note2")?,
        sort_order: row.try_get("sort_order")?,
        total: deposit_total(principal, interest),
        // A deposit stays "active" until 收訖 — even past its end date.
        status: if received_at.is_some() {
            DepositStatus::End
        } else {
            DepositStatus::Active
        },
        end_year: parsed.year(),
        end_month: parsed.month(),
    })
}

/// `received`: Some(false) lists only 未收訖 rows (received_at IS NULL),
/// Some(true) only received rows.
pub async fn list(
    pool: &SqlitePool,
    received: Option<bool>,
    year: Option<i32>,
    descending: bool,
) -> Result<Vec<Deposit>, StoreError> {
    let mut sql = format!("SELECT {DEPOSIT_COLUMNS} FROM deposits");
    let mut conditions: Vec<&str> = Vec::new();
    if let Some(received) = received {
        if received {
            conditions.push("received_at IS NOT NULL");
        } else {
            conditions.push("received_at IS NULL");
        }
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
    if let Some(year) = year {
        statement = statement
            .bind(format!("{year:04}-01-01"))
            .bind(format!("{year:04}-12-31"));
    }

    let today = today();
    statement
        .fetch_all(pool)
        .await?
        .iter()
        .map(|row| row_to_deposit(row, today))
        .collect::<Result<Vec<_>, _>>()
}

/// `exec` may be the pool or a transaction — `POST /api/forecast-items/{id}/convert`
/// inserts inside its own transaction.
pub async fn insert_deposit<'e, E>(
    exec: E,
    deposit: &ValidatedDeposit,
    sort_order: i64,
    body: &NewDeposit,
) -> Result<i64, StoreError>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    let now = now_timestamp();
    // A deposit entered already past its end date is recorded as received —
    // its interest fed 利息 all along. 收訖 only applies to future deposits.
    let received_at = (deposit.end_date.as_str() <= today().to_string().as_str())
        .then(|| deposit.end_date.clone());
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO deposits (label, bank, principal, rate, interest, start_date, end_date, \
         received_at, note1, note2, sort_order, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(deposit.label.as_deref())
    .bind(deposit.bank.as_deref())
    .bind(deposit.principal)
    .bind(deposit.rate)
    .bind(deposit.interest)
    .bind(deposit.start_date.as_deref())
    .bind(&deposit.end_date)
    .bind(&received_at)
    .bind(body.note1.as_deref())
    .bind(body.note2.as_deref())
    .bind(sort_order)
    .bind(&now)
    .bind(&now)
    .fetch_one(exec)
    .await?;
    Ok(id)
}

pub async fn update_deposit(
    pool: &SqlitePool,
    id: i64,
    validated: &ValidatedDeposit,
    note1: &Option<String>,
    note2: &Option<String>,
) -> Result<(), StoreError> {
    sqlx::query(
        "UPDATE deposits SET label = ?, bank = ?, principal = ?, rate = ?, interest = ?, \
         start_date = ?, end_date = ?, note1 = ?, note2 = ?, updated_at = ? WHERE id = ?",
    )
    .bind(&validated.label)
    .bind(&validated.bank)
    .bind(validated.principal)
    .bind(validated.rate)
    .bind(validated.interest)
    .bind(&validated.start_date)
    .bind(&validated.end_date)
    .bind(note1)
    .bind(note2)
    .bind(now_timestamp())
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Rows deleted, so the handler can tell "not found" apart from success.
pub async fn delete_deposit(pool: &SqlitePool, id: i64) -> Result<u64, StoreError> {
    Ok(sqlx::query("DELETE FROM deposits WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?
        .rows_affected())
}

pub async fn next_sort_order(pool: &SqlitePool) -> Result<i64, StoreError> {
    Ok(
        sqlx::query_scalar("SELECT COALESCE(MAX(sort_order), 0) + 1 FROM deposits")
            .fetch_one(pool)
            .await?,
    )
}

pub async fn mark_received(
    conn: &mut sqlx::SqliteConnection,
    id: i64,
    received_at: &str,
    interest: Option<f64>,
    credit_asset_id: Option<i64>,
    credited_amount: Option<f64>,
    now: &str,
) -> Result<(), StoreError> {
    sqlx::query(
        "UPDATE deposits SET received_at = ?, interest = ?, credited_asset_id = ?, \
         credited_amount = ?, updated_at = ? WHERE id = ?",
    )
    .bind(received_at)
    .bind(interest)
    .bind(credit_asset_id)
    .bind(credited_amount)
    .bind(now)
    .bind(id)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// The stored bank-in fields of a received deposit: (credited_asset_id, credited_amount).
pub async fn credited_of(
    pool: &SqlitePool,
    id: i64,
) -> Result<Option<(Option<i64>, Option<f64>)>, StoreError> {
    Ok(
        sqlx::query_as("SELECT credited_asset_id, credited_amount FROM deposits WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await?,
    )
}

pub async fn clear_receipt(
    conn: &mut sqlx::SqliteConnection,
    id: i64,
    now: &str,
) -> Result<(), StoreError> {
    sqlx::query(
        "UPDATE deposits SET received_at = NULL, credited_asset_id = NULL, \
         credited_amount = NULL, updated_at = ? WHERE id = ?",
    )
    .bind(now)
    .bind(id)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

pub async fn load_one(pool: &SqlitePool, id: i64) -> Result<Deposit, StoreError> {
    let row = sqlx::query(sqlx::AssertSqlSafe(format!(
        "SELECT {DEPOSIT_COLUMNS} FROM deposits WHERE id = ?"
    )))
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| StoreError::NotFound(format!("deposit {id} not found")))?;
    row_to_deposit(&row, today())
}

pub async fn load_all(pool: &SqlitePool) -> Result<Vec<Deposit>, StoreError> {
    let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
        "SELECT {DEPOSIT_COLUMNS} FROM deposits ORDER BY sort_order, id"
    )))
    .fetch_all(pool)
    .await?;
    let today = today();
    rows.iter()
        .map(|row| row_to_deposit(row, today))
        .collect::<Result<Vec<_>, _>>()
}

/// The deposits reduced to principal/interest/end_date for the year review —
/// unordered, exactly as the original `deposit_facts` loader.
pub async fn year_facts(pool: &SqlitePool) -> Result<Vec<DepositFacts<'static>>, StoreError> {
    let rows = sqlx::query("SELECT principal, interest, end_date FROM deposits")
        .fetch_all(pool)
        .await?;
    rows_to_facts(&rows)
}

/// The same facts in `sort_order` — the live totals path used that ordering.
pub async fn active_facts(pool: &SqlitePool) -> Result<Vec<DepositFacts<'static>>, StoreError> {
    let rows =
        sqlx::query("SELECT principal, interest, end_date FROM deposits ORDER BY sort_order, id")
            .fetch_all(pool)
            .await?;
    rows_to_facts(&rows)
}

fn rows_to_facts(rows: &[SqliteRow]) -> Result<Vec<DepositFacts<'static>>, StoreError> {
    let mut facts = Vec::with_capacity(rows.len());
    for row in rows {
        let end_date: String = row.try_get("end_date")?;
        facts.push(DepositFacts {
            bank: None,
            principal: row.try_get("principal")?,
            interest: row.try_get("interest")?,
            end_date: chrono::NaiveDate::parse_from_str(&end_date, "%Y-%m-%d").map_err(|_| {
                StoreError::Corrupt(format!("stored end_date {end_date} is not valid"))
            })?,
        });
    }
    Ok(facts)
}

/// Deposits whose start or end date falls in `range` (`[first, next)`); `None`
/// loads them all — the derived 利息 needs every month at once.
pub async fn suggestion_events(
    pool: &SqlitePool,
    range: Option<(String, String)>,
) -> Result<Vec<SuggestionDeposit>, StoreError> {
    let rows =
        match &range {
            Some((first, next)) => sqlx::query(
                "SELECT id, label, bank, start_date, end_date, principal, interest, received_at \
                 FROM deposits \
             WHERE (start_date IS NOT NULL AND start_date >= ? AND start_date < ?) \
             OR (end_date >= ? AND end_date < ?)",
            )
            .bind(first)
            .bind(next)
            .bind(first)
            .bind(next)
            .fetch_all(pool)
            .await?,
            None => sqlx::query(
                "SELECT id, label, bank, start_date, end_date, principal, interest, received_at \
                 FROM deposits",
            )
            .fetch_all(pool)
            .await?,
        };
    rows.iter()
        .map(|row| {
            let start_date: Option<String> = row.try_get("start_date")?;
            let received_at: Option<String> = row.try_get("received_at")?;
            let end_date: String = row.try_get("end_date")?;
            Ok(SuggestionDeposit {
                id: row.try_get("id")?,
                start_date: start_date
                    .map(|date| {
                        chrono::NaiveDate::parse_from_str(&date, "%Y-%m-%d").map_err(|_| {
                            StoreError::Corrupt(format!("stored date {date} is not valid"))
                        })
                    })
                    .transpose()?,
                end_date: chrono::NaiveDate::parse_from_str(&end_date, "%Y-%m-%d").map_err(
                    |_| StoreError::Corrupt(format!("stored date {end_date} is not valid")),
                )?,
                principal: row.try_get::<Option<f64>, _>("principal")?.unwrap_or(0.0),
                interest: row.try_get::<Option<f64>, _>("interest")?.unwrap_or(0.0),
                label: row.try_get("label")?,
                received: received_at.is_some(),
                bank: row.try_get("bank")?,
            })
        })
        .collect()
}

/// Next deposit sort order, inside the caller's transaction (the 轉為定期
/// conversion).
pub async fn next_sort_order_tx(conn: &mut sqlx::SqliteConnection) -> Result<i64, StoreError> {
    Ok(
        sqlx::query_scalar("SELECT COALESCE(MAX(sort_order), 0) + 1 FROM deposits")
            .fetch_one(&mut *conn)
            .await?,
    )
}
