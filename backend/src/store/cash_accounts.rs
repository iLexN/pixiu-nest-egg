//! The `cash_accounts` rows: 活期 bank accounts plus the IBKR broker cash per
//! currency.

use sqlx::Row;
use sqlx::SqlitePool;
use sqlx::sqlite::SqliteRow;

use crate::models::{CashAccount, CashRole, Currency};
use crate::store::{StoreError, now_timestamp};

fn row_to_account(row: &SqliteRow) -> Result<CashAccount, StoreError> {
    let currency: String = row.try_get("currency")?;
    let role: String = row.try_get("role")?;
    Ok(CashAccount {
        id: row.try_get("id")?,
        label: row.try_get("label")?,
        currency: Currency::parse(&currency).ok_or_else(|| {
            StoreError::Corrupt(format!("stored currency {currency} is not valid"))
        })?,
        role: CashRole::parse(&role)
            .ok_or_else(|| StoreError::Corrupt(format!("stored cash role {role} is not valid")))?,
        amount: row.try_get("amount")?,
        is_default_receipt: row.try_get::<i64, _>("is_default_receipt")? != 0,
        sort_order: row.try_get("sort_order")?,
        updated_at: row.try_get("updated_at")?,
    })
}

const COLUMNS: &str =
    "id, label, currency, role, amount, is_default_receipt, sort_order, updated_at";

pub async fn load_all(pool: &SqlitePool) -> Result<Vec<CashAccount>, StoreError> {
    let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
        "SELECT {COLUMNS} FROM cash_accounts ORDER BY sort_order, id"
    )))
    .fetch_all(pool)
    .await?;
    rows.iter().map(row_to_account).collect()
}

/// `Err(NotFound)` while the id names no account.
pub async fn load_one<'e, E>(exec: E, id: i64) -> Result<CashAccount, StoreError>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    let row = sqlx::query(sqlx::AssertSqlSafe(format!(
        "SELECT {COLUMNS} FROM cash_accounts WHERE id = ?"
    )))
    .bind(id)
    .fetch_optional(exec)
    .await?
    .ok_or_else(|| StoreError::NotFound(format!("cash account {id} not found")))?;
    row_to_account(&row)
}

/// The account flagged `is_default_receipt` for `currency`, if any.
pub async fn default_for<'e, E>(
    exec: E,
    currency: Currency,
) -> Result<Option<CashAccount>, StoreError>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    let row = sqlx::query(sqlx::AssertSqlSafe(format!(
        "SELECT {COLUMNS} FROM cash_accounts WHERE currency = ? AND is_default_receipt = 1"
    )))
    .bind(currency.as_str())
    .fetch_optional(exec)
    .await?;
    row.as_ref().map(row_to_account).transpose()
}

/// The broker account for `currency` — `None` while the row is absent.
pub async fn broker<'e, E>(exec: E, currency: Currency) -> Result<Option<CashAccount>, StoreError>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    let row = sqlx::query(sqlx::AssertSqlSafe(format!(
        "SELECT {COLUMNS} FROM cash_accounts WHERE role = 'broker' AND currency = ?"
    )))
    .bind(currency.as_str())
    .fetch_optional(exec)
    .await?;
    row.as_ref().map(row_to_account).transpose()
}

/// Σ amount over the `bank` accounts — the live 活期 sum.
pub async fn bank_sum<'e, E>(exec: E) -> Result<f64, StoreError>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    Ok(sqlx::query_scalar(
        "SELECT COALESCE(SUM(amount), 0.0) FROM cash_accounts WHERE role = 'bank'",
    )
    .fetch_one(exec)
    .await?)
}

pub async fn next_sort_order(pool: &SqlitePool) -> Result<i64, StoreError> {
    Ok(
        sqlx::query_scalar("SELECT COALESCE(MAX(sort_order), 0) + 1 FROM cash_accounts")
            .fetch_one(pool)
            .await?,
    )
}

/// Insert an account. When `is_default_receipt`, the other accounts of the
/// currency lose the flag first — the partial unique index allows only one.
#[allow(clippy::too_many_arguments)]
pub async fn insert(
    pool: &SqlitePool,
    label: &str,
    currency: Currency,
    role: CashRole,
    amount: f64,
    is_default_receipt: bool,
    sort_order: i64,
) -> Result<i64, StoreError> {
    let mut tx = pool.begin().await?;
    if is_default_receipt {
        sqlx::query("UPDATE cash_accounts SET is_default_receipt = 0 WHERE currency = ?")
            .bind(currency.as_str())
            .execute(&mut *tx)
            .await?;
    }
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO cash_accounts (label, currency, role, amount, is_default_receipt, \
         sort_order, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(label)
    .bind(currency.as_str())
    .bind(role.as_str())
    .bind(amount)
    .bind(is_default_receipt as i64)
    .bind(sort_order)
    .bind(now_timestamp())
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(id)
}

/// Update an account's fields. The default-flag rule of `insert` applies.
#[allow(clippy::too_many_arguments)]
pub async fn update(
    pool: &SqlitePool,
    id: i64,
    label: &str,
    currency: Currency,
    role: CashRole,
    amount: f64,
    is_default_receipt: bool,
    sort_order: i64,
) -> Result<(), StoreError> {
    let mut tx = pool.begin().await?;
    if is_default_receipt {
        sqlx::query(
            "UPDATE cash_accounts SET is_default_receipt = 0 \
             WHERE currency = ? AND id != ?",
        )
        .bind(currency.as_str())
        .bind(id)
        .execute(&mut *tx)
        .await?;
    }
    sqlx::query(
        "UPDATE cash_accounts SET label = ?, currency = ?, role = ?, amount = ?, \
         is_default_receipt = ?, sort_order = ?, updated_at = ? WHERE id = ?",
    )
    .bind(label)
    .bind(currency.as_str())
    .bind(role.as_str())
    .bind(amount)
    .bind(is_default_receipt as i64)
    .bind(sort_order)
    .bind(now_timestamp())
    .bind(id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

/// `amount += delta` on one account, inside the caller's transaction. Used by
/// the receipt flows; a reversal may drive a balance negative — the
/// non-negative check lives in API validation, not here.
pub async fn add_amount(
    conn: &mut sqlx::SqliteConnection,
    id: i64,
    delta: f64,
    now: &str,
) -> Result<(), StoreError> {
    sqlx::query("UPDATE cash_accounts SET amount = amount + ?, updated_at = ? WHERE id = ?")
        .bind(delta)
        .bind(now)
        .bind(id)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

/// `amount = value` on one account, inside the caller's transaction.
pub async fn set_amount(
    conn: &mut sqlx::SqliteConnection,
    id: i64,
    amount: f64,
    now: &str,
) -> Result<(), StoreError> {
    sqlx::query("UPDATE cash_accounts SET amount = ?, updated_at = ? WHERE id = ?")
        .bind(amount)
        .bind(now)
        .bind(id)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

/// Delete a bank account. Broker rows are protected — the IBKR block depends
/// on them. Past receipts stay readable: deposits/bonds lose their
/// `credited_account_id`, and the bond_coupons/dividends FKs clear themselves
/// via `ON DELETE SET NULL`.
pub async fn delete(pool: &SqlitePool, id: i64) -> Result<(), StoreError> {
    let mut tx = pool.begin().await?;
    let account = load_one(&mut *tx, id).await?;
    if account.role == CashRole::Broker {
        return Err(StoreError::Conflict(format!(
            "cash account {id} is a broker account and cannot be deleted"
        )));
    }
    sqlx::query("UPDATE deposits SET credited_account_id = NULL WHERE credited_account_id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE bonds SET credited_account_id = NULL WHERE credited_account_id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM cash_accounts WHERE id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connect_memory;

    async fn seed_bank(pool: &SqlitePool, label: &str, default: bool) -> i64 {
        insert(
            pool,
            label,
            Currency::Hkd,
            CashRole::Bank,
            100.0,
            default,
            next_sort_order(pool).await.unwrap(),
        )
        .await
        .unwrap()
    }

    /// The partial unique index allows one default per currency: marking a
    /// second account default clears the first in the same transaction.
    #[tokio::test]
    async fn setting_a_new_default_clears_the_old_one() {
        let pool = connect_memory().await.expect("memory db");
        let hs = seed_bank(&pool, "HS", true).await;
        let sc = seed_bank(&pool, "渣打", false).await;

        update(
            &pool,
            sc,
            "渣打2",
            Currency::Hkd,
            CashRole::Bank,
            0.0,
            true,
            9,
        )
        .await
        .unwrap();
        let default = default_for(&pool, Currency::Hkd).await.unwrap().unwrap();
        assert_eq!(default.id, sc);
        assert!(!load_one(&pool, hs).await.unwrap().is_default_receipt);
    }

    /// Broker rows are protected; deleting a bank account clears the
    /// deposits/bonds that recorded it as their receipt target.
    #[tokio::test]
    async fn delete_refuses_broker_rows_and_nulls_credited_ids() {
        let pool = connect_memory().await.expect("memory db");
        let broker = broker(&pool, Currency::Usd).await.unwrap().expect("broker");
        let err = delete(&pool, broker.id).await.expect_err("broker refused");
        assert!(matches!(err, StoreError::Conflict(_)));

        let hs = seed_bank(&pool, "HS", true).await;
        sqlx::query(
            "INSERT INTO deposits (label, bank, principal, interest, end_date, \
             received_at, credited_account_id, credited_amount, sort_order, \
             created_at, updated_at) VALUES ('d', 'SC', 1, 0, '2026-01-01', \
             '2026-01-01', ?, 1, 1, '2026-01-01T00:00:00', '2026-01-01T00:00:00')",
        )
        .bind(hs)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO bonds (label, principal, maturity_date, received_at, \
             credited_account_id, credited_amount, sort_order, created_at, updated_at) \
             VALUES ('b', 1, '2026-01-01', '2026-01-01', ?, 1, 1, \
             '2026-01-01T00:00:00', '2026-01-01T00:00:00')",
        )
        .bind(hs)
        .execute(&pool)
        .await
        .unwrap();

        delete(&pool, hs).await.unwrap();
        for table in ["deposits", "bonds"] {
            let n: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
                "SELECT COUNT(*) FROM {table} WHERE credited_account_id IS NOT NULL"
            )))
            .fetch_one(&pool)
            .await
            .unwrap();
            assert_eq!(n, 0, "{table}.credited_account_id");
        }
    }
}
