//! The shared receipt-credit rule used by every source: pick the cash account
//! a received amount banks into, apply it, and reverse it exactly.

use crate::calc::FieldError;
use crate::models::Currency;
use crate::service::ServiceError;
use crate::store::cash_accounts;
use crate::store::{Credit, StoreError};

/// Pick the account a receipt banks into: the caller's explicit choice first,
/// else the currency's default-receipt account. `bank_in == false` and no
/// default both mean no credit.
async fn choose(
    conn: &mut sqlx::SqliteConnection,
    currency: Currency,
    amount: f64,
    bank_in: bool,
    account_id: Option<i64>,
) -> Result<Option<Credit>, ServiceError> {
    if !bank_in {
        return Ok(None);
    }
    let account = match account_id {
        Some(id) => {
            let account = cash_accounts::load_one(&mut *conn, id).await?;
            if account.currency != currency {
                return Err(ServiceError::Validation(vec![FieldError::new(
                    "account_id",
                    format!("account currency must be {}", currency.as_str()),
                )]));
            }
            account
        }
        None => match cash_accounts::default_for(&mut *conn, currency).await? {
            Some(account) => account,
            None => return Ok(None),
        },
    };
    Ok(Some(Credit {
        account_id: account.id,
        amount,
    }))
}

/// Apply a receipt's credit: choose the account per [`choose`], add the
/// amount there, and return the `Credit` for the caller to record.
pub async fn credit(
    conn: &mut sqlx::SqliteConnection,
    currency: Currency,
    amount: f64,
    bank_in: bool,
    account_id: Option<i64>,
    now: &str,
) -> Result<Option<Credit>, ServiceError> {
    let Some(credit) = choose(conn, currency, amount, bank_in, account_id).await? else {
        return Ok(None);
    };
    cash_accounts::add_amount(conn, credit.account_id, credit.amount, now).await?;
    Ok(Some(credit))
}

/// Undo a recorded credit: subtract the stored amount from the stored
/// account. A deleted account or no credit is a no-op.
pub async fn reverse(
    conn: &mut sqlx::SqliteConnection,
    credit: Option<Credit>,
    now: &str,
) -> Result<(), StoreError> {
    let Some(credit) = credit else {
        return Ok(());
    };
    match cash_accounts::load_one(&mut *conn, credit.account_id).await {
        Ok(_) => cash_accounts::add_amount(conn, credit.account_id, -credit.amount, now).await,
        Err(StoreError::NotFound(_)) => Ok(()),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connect_memory;
    use sqlx::SqlitePool;

    const NOW: &str = "2026-10-20T00:00:00";

    /// A bank account; `default` marks it the receipt default for its
    /// currency. Returns its id.
    async fn seed_account(
        pool: &SqlitePool,
        label: &str,
        currency: Currency,
        amount: f64,
        default: bool,
    ) -> i64 {
        let sort: i64 =
            sqlx::query_scalar("SELECT COALESCE(MAX(sort_order), 0) + 1 FROM cash_accounts")
                .fetch_one(pool)
                .await
                .unwrap();
        sqlx::query_scalar(
            "INSERT INTO cash_accounts (label, currency, role, amount, is_default_receipt, \
             sort_order, updated_at) VALUES (?, ?, 'bank', ?, ?, ?, ?) RETURNING id",
        )
        .bind(label)
        .bind(currency.as_str())
        .bind(amount)
        .bind(default as i64)
        .bind(sort)
        .bind(NOW)
        .fetch_one(pool)
        .await
        .unwrap()
    }

    async fn amount(pool: &SqlitePool, id: i64) -> f64 {
        sqlx::query_scalar("SELECT amount FROM cash_accounts WHERE id = ?")
            .bind(id)
            .fetch_one(pool)
            .await
            .unwrap()
    }

    /// 1. bank_in with no explicit account banks into the currency default.
    #[tokio::test]
    async fn credit_uses_the_default_account() {
        let pool = connect_memory().await.expect("memory db");
        let hs = seed_account(&pool, "HS", Currency::Hkd, 100.0, true).await;
        let sc = seed_account(&pool, "渣打", Currency::Hkd, 0.0, false).await;
        let mut conn = pool.acquire().await.unwrap();

        let credit = credit(&mut conn, Currency::Hkd, 50.0, true, None, NOW)
            .await
            .unwrap();
        assert_eq!(credit.map(|c| (c.account_id, c.amount)), Some((hs, 50.0)));
        drop(conn);
        assert_eq!(amount(&pool, hs).await, 150.0);
        assert_eq!(amount(&pool, sc).await, 0.0);
    }

    /// 2. An explicit account wins over the default.
    #[tokio::test]
    async fn credit_honours_an_explicit_account() {
        let pool = connect_memory().await.expect("memory db");
        let hs = seed_account(&pool, "HS", Currency::Hkd, 100.0, true).await;
        let sc = seed_account(&pool, "渣打", Currency::Hkd, 0.0, false).await;
        let mut conn = pool.acquire().await.unwrap();

        let credit = credit(&mut conn, Currency::Hkd, 50.0, true, Some(sc), NOW)
            .await
            .unwrap();
        assert_eq!(credit.unwrap().account_id, sc);
        drop(conn);
        assert_eq!(amount(&pool, hs).await, 100.0);
        assert_eq!(amount(&pool, sc).await, 50.0);
    }

    /// 3. An explicit account in the wrong currency is a validation error.
    #[tokio::test]
    async fn credit_rejects_a_currency_mismatch() {
        let pool = connect_memory().await.expect("memory db");
        // Bank accounts are HKD-only by schema; the migrated IBKR USD broker
        // row supplies a USD account.
        let usd: i64 = sqlx::query_scalar(
            "SELECT id FROM cash_accounts WHERE role = 'broker' AND currency = 'USD'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        let mut conn = pool.acquire().await.unwrap();

        let err = credit(&mut conn, Currency::Hkd, 50.0, true, Some(usd), NOW)
            .await
            .expect_err("mismatch");
        assert!(matches!(err, ServiceError::Validation(_)));
    }

    /// 4. With no default and no explicit account, nothing is credited.
    #[tokio::test]
    async fn credit_is_a_no_op_without_a_default() {
        let pool = connect_memory().await.expect("memory db");
        let sc = seed_account(&pool, "渣打", Currency::Hkd, 0.0, false).await;
        let mut conn = pool.acquire().await.unwrap();

        let credit = credit(&mut conn, Currency::Hkd, 50.0, true, None, NOW)
            .await
            .unwrap();
        assert!(credit.is_none());
        drop(conn);
        assert_eq!(amount(&pool, sc).await, 0.0);
    }

    /// 5. Reversal subtracts from the recorded account even after the
    /// default moved elsewhere.
    #[tokio::test]
    async fn reversal_follows_the_recorded_account() {
        let pool = connect_memory().await.expect("memory db");
        let hs = seed_account(&pool, "HS", Currency::Hkd, 100.0, true).await;
        let sc = seed_account(&pool, "渣打", Currency::Hkd, 0.0, false).await;
        let mut conn = pool.acquire().await.unwrap();

        let credit = credit(&mut conn, Currency::Hkd, 50.0, true, None, NOW)
            .await
            .unwrap()
            .unwrap();
        // Move the default to 渣打 — the recorded credit still points at HS.
        sqlx::query(
            "UPDATE cash_accounts SET is_default_receipt = 0 WHERE id = ?; \
             UPDATE cash_accounts SET is_default_receipt = 1 WHERE id = ?",
        )
        .bind(hs)
        .bind(sc)
        .execute(&mut *conn)
        .await
        .unwrap();
        reverse(&mut conn, Some(credit), NOW).await.unwrap();
        drop(conn);
        assert_eq!(amount(&pool, hs).await, 100.0);
        assert_eq!(amount(&pool, sc).await, 0.0);
    }

    /// 6. Reversal after the recorded account was deleted changes nothing.
    #[tokio::test]
    async fn reversal_after_account_deletion_is_a_no_op() {
        let pool = connect_memory().await.expect("memory db");
        let hs = seed_account(&pool, "HS", Currency::Hkd, 100.0, true).await;
        let sc = seed_account(&pool, "渣打", Currency::Hkd, 0.0, false).await;
        let mut conn = pool.acquire().await.unwrap();
        let credit = credit(&mut conn, Currency::Hkd, 50.0, true, Some(hs), NOW)
            .await
            .unwrap();
        drop(conn);
        cash_accounts::delete(&pool, hs).await.unwrap();
        let mut conn = pool.acquire().await.unwrap();
        reverse(&mut conn, credit, NOW).await.unwrap();
        drop(conn);
        assert_eq!(amount(&pool, sc).await, 0.0);
    }

    /// `bank_in = false` never credits.
    #[tokio::test]
    async fn bank_in_false_credits_nothing() {
        let pool = connect_memory().await.expect("memory db");
        let hs = seed_account(&pool, "HS", Currency::Hkd, 100.0, true).await;
        let mut conn = pool.acquire().await.unwrap();
        let credit = credit(&mut conn, Currency::Hkd, 50.0, false, Some(hs), NOW)
            .await
            .unwrap();
        assert!(credit.is_none());
        drop(conn);
        assert_eq!(amount(&pool, hs).await, 100.0);
    }
}
