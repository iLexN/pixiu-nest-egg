//! Cash-account CRUD validation: every field's rules live here; the store
//! enforces the unique constraints (label, one broker per currency, one
//! default per currency) that surface as 409s.

use sqlx::SqlitePool;

use crate::calc::FieldError;
use crate::models::{
    CashAccount, CashAccountPatch, CashRole, Currency, NewBankMove, NewCashAccount,
};
use crate::service::{ServiceError, cash_movement};
use crate::store::{StoreError, cash_accounts, now_timestamp};

fn validate(
    label: &str,
    currency: Currency,
    role: CashRole,
    amount: f64,
) -> Result<String, ServiceError> {
    let mut errors = Vec::new();
    let label = label.trim();
    if label.is_empty() {
        errors.push(FieldError::new("label", "label is required"));
    }
    if !amount.is_finite() || amount < 0.0 {
        errors.push(FieldError::new(
            "amount",
            "amount must be a non-negative number",
        ));
    }
    if role == CashRole::Bank && currency != Currency::Hkd {
        errors.push(FieldError::new("currency", "bank accounts must be HKD"));
    }
    if errors.is_empty() {
        Ok(label.to_string())
    } else {
        Err(ServiceError::Validation(errors))
    }
}

pub async fn create(pool: &SqlitePool, body: &NewCashAccount) -> Result<CashAccount, ServiceError> {
    let amount = body.amount.unwrap_or(0.0);
    let label = validate(&body.label, body.currency, body.role, amount)?;
    let sort_order = cash_accounts::next_sort_order(pool).await?;
    let id = cash_accounts::insert(
        pool,
        &label,
        body.currency,
        body.role,
        amount,
        body.is_default_receipt.unwrap_or(false),
        sort_order,
    )
    .await?;
    Ok(cash_accounts::load_one(pool, id).await?)
}

/// `Err(NotFound)` while `id` names no account.
pub async fn update(
    pool: &SqlitePool,
    id: i64,
    patch: &CashAccountPatch,
) -> Result<CashAccount, ServiceError> {
    let existing = cash_accounts::load_one(pool, id).await?;
    let currency = patch.currency.unwrap_or(existing.currency);
    let role = patch.role.unwrap_or(existing.role);
    let amount = patch.amount.unwrap_or(existing.amount);
    let label = validate(
        patch.label.as_deref().unwrap_or(&existing.label),
        currency,
        role,
        amount,
    )?;
    cash_accounts::update(
        pool,
        id,
        &label,
        currency,
        role,
        amount,
        patch
            .is_default_receipt
            .unwrap_or(existing.is_default_receipt),
        existing.sort_order,
    )
    .await?;
    Ok(cash_accounts::load_one(pool, id).await?)
}

/// `Err(NotFound)` for a missing id; `Err(Conflict)` for a broker account.
pub async fn remove(pool: &SqlitePool, id: i64) -> Result<(), ServiceError> {
    cash_accounts::delete(pool, id).await?;
    Ok(())
}

/// 活期調撥: move `amount` from one HKD bank account to another in one
/// transaction and return both, source first. Records nothing — the 活期
/// total is unchanged, and the undo is a move the other way.
pub async fn bank_move(
    pool: &SqlitePool,
    body: &NewBankMove,
) -> Result<Vec<CashAccount>, ServiceError> {
    let mut errors = Vec::new();
    if !body.amount.is_finite() || body.amount <= 0.0 {
        errors.push(FieldError::new(
            "amount",
            "amount must be a number greater than 0",
        ));
    }
    if body.from_account_id == body.to_account_id {
        errors.push(FieldError::new(
            "to_account_id",
            "destination must differ from the source",
        ));
    }
    if !errors.is_empty() {
        return Err(ServiceError::Validation(errors));
    }
    let now = now_timestamp();
    let mut tx = pool.begin().await.map_err(StoreError::from)?;
    let from = cash_movement::debit_bank(
        &mut tx,
        body.from_account_id,
        body.amount,
        "from_account_id",
        "amount",
        &now,
    )
    .await?;
    let to = cash_accounts::load_one(&mut *tx, body.to_account_id).await?;
    cash_movement::require_hkd_bank(&to, "to_account_id")?;
    cash_accounts::add_amount(&mut tx, to.id, body.amount, &now).await?;
    let accounts = vec![
        cash_accounts::load_one(&mut *tx, from.id).await?,
        cash_accounts::load_one(&mut *tx, to.id).await?,
    ];
    tx.commit().await.map_err(StoreError::from)?;
    Ok(accounts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calc::approx_eq;
    use crate::db::connect_memory;

    async fn bank(pool: &SqlitePool, label: &str, amount: f64) -> i64 {
        create(
            pool,
            &NewCashAccount {
                label: label.to_string(),
                currency: Currency::Hkd,
                role: CashRole::Bank,
                amount: Some(amount),
                is_default_receipt: None,
            },
        )
        .await
        .expect("seed bank")
        .id
    }

    async fn balance(pool: &SqlitePool, id: i64) -> f64 {
        cash_accounts::load_one(pool, id).await.unwrap().amount
    }

    fn mv(from: i64, to: i64, amount: f64) -> NewBankMove {
        NewBankMove {
            from_account_id: from,
            to_account_id: to,
            amount,
        }
    }

    /// The field names of a `Validation` error; panics on any other error.
    fn fields(err: ServiceError) -> Vec<String> {
        match err {
            ServiceError::Validation(errors) => errors.into_iter().map(|e| e.field).collect(),
            other => panic!("expected Validation, got {other:?}"),
        }
    }

    async fn hkd_broker(pool: &SqlitePool) -> i64 {
        cash_accounts::broker(pool, Currency::Hkd)
            .await
            .unwrap()
            .expect("migrated IBKR HKD row")
            .id
    }

    #[tokio::test]
    async fn move_hs_to_sc_shifts_the_amount_and_keeps_the_sum() {
        let pool = connect_memory().await.unwrap();
        let hs = bank(&pool, "HS", 30000.0).await;
        let sc = bank(&pool, "渣打", 1549.01).await;

        let accounts = bank_move(&pool, &mv(hs, sc, 10000.0)).await.unwrap();

        let ids: Vec<i64> = accounts.iter().map(|a| a.id).collect();
        assert_eq!(ids, [hs, sc]);
        assert_eq!(accounts[0].amount, 20000.0);
        assert!(approx_eq(accounts[1].amount, 11549.01));
        assert_eq!(balance(&pool, hs).await, 20000.0);
        assert!(approx_eq(balance(&pool, sc).await, 11549.01));
    }

    #[tokio::test]
    async fn moving_back_restores_both_accounts() {
        let pool = connect_memory().await.unwrap();
        let hs = bank(&pool, "HS", 30000.0).await;
        let sc = bank(&pool, "渣打", 1549.01).await;

        bank_move(&pool, &mv(hs, sc, 10000.0)).await.unwrap();
        bank_move(&pool, &mv(sc, hs, 10000.0)).await.unwrap();

        // Stored REAL is never rounded, so 1549.01 + 10000 − 10000 may
        // differ in the last bit.
        assert!(approx_eq(balance(&pool, hs).await, 30000.0));
        assert!(approx_eq(balance(&pool, sc).await, 1549.01));
    }

    #[tokio::test]
    async fn overdraft_is_rejected_on_amount_and_changes_nothing() {
        let pool = connect_memory().await.unwrap();
        let hs = bank(&pool, "HS", 5000.0).await;
        let sc = bank(&pool, "渣打", 100.0).await;

        let err = bank_move(&pool, &mv(hs, sc, 8000.0)).await.unwrap_err();

        assert_eq!(fields(err), ["amount"]);
        assert_eq!(balance(&pool, hs).await, 5000.0);
        assert_eq!(balance(&pool, sc).await, 100.0);
    }

    #[tokio::test]
    async fn broker_destination_is_rejected_and_the_source_is_unchanged() {
        let pool = connect_memory().await.unwrap();
        let hs = bank(&pool, "HS", 5000.0).await;
        let broker = hkd_broker(&pool).await;
        let broker_before = balance(&pool, broker).await;

        let err = bank_move(&pool, &mv(hs, broker, 1000.0)).await.unwrap_err();

        assert_eq!(fields(err), ["to_account_id"]);
        assert_eq!(balance(&pool, hs).await, 5000.0);
        assert_eq!(balance(&pool, broker).await, broker_before);
    }

    #[tokio::test]
    async fn broker_source_is_rejected_on_from_account_id() {
        let pool = connect_memory().await.unwrap();
        let hs = bank(&pool, "HS", 5000.0).await;
        let broker = hkd_broker(&pool).await;

        let err = bank_move(&pool, &mv(broker, hs, 1000.0)).await.unwrap_err();

        assert_eq!(fields(err), ["from_account_id"]);
        assert_eq!(balance(&pool, hs).await, 5000.0);
    }

    #[tokio::test]
    async fn same_account_is_rejected_on_to_account_id() {
        let pool = connect_memory().await.unwrap();
        let hs = bank(&pool, "HS", 5000.0).await;

        let err = bank_move(&pool, &mv(hs, hs, 1000.0)).await.unwrap_err();

        assert_eq!(fields(err), ["to_account_id"]);
        assert_eq!(balance(&pool, hs).await, 5000.0);
    }

    #[tokio::test]
    async fn zero_negative_and_nan_amounts_are_rejected_on_amount() {
        let pool = connect_memory().await.unwrap();
        let hs = bank(&pool, "HS", 5000.0).await;
        let sc = bank(&pool, "渣打", 100.0).await;

        for amount in [0.0, -1.0, f64::NAN] {
            let err = bank_move(&pool, &mv(hs, sc, amount)).await.unwrap_err();
            assert_eq!(fields(err), ["amount"], "amount = {amount}");
        }
        assert_eq!(balance(&pool, hs).await, 5000.0);
        assert_eq!(balance(&pool, sc).await, 100.0);
    }

    #[tokio::test]
    async fn unknown_destination_is_not_found_and_the_source_is_unchanged() {
        let pool = connect_memory().await.unwrap();
        let hs = bank(&pool, "HS", 5000.0).await;

        let err = bank_move(&pool, &mv(hs, 9999, 1000.0)).await.unwrap_err();

        assert!(
            matches!(err, ServiceError::Store(StoreError::NotFound(_))),
            "got {err:?}"
        );
        assert_eq!(balance(&pool, hs).await, 5000.0);
    }

    /// Candidates are `month_items` rows too, so one count covers both.
    #[tokio::test]
    async fn a_move_writes_no_month_item() {
        let pool = connect_memory().await.unwrap();
        let hs = bank(&pool, "HS", 30000.0).await;
        let sc = bank(&pool, "渣打", 1549.01).await;
        let now = now_timestamp();
        sqlx::query(
            "INSERT INTO month_stats (month, created_at, updated_at) \
             VALUES (date('now', 'localtime', 'start of month'), ?, ?)",
        )
        .bind(&now)
        .bind(&now)
        .execute(&pool)
        .await
        .unwrap();
        let count = || async {
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM month_items")
                .fetch_one(&pool)
                .await
                .unwrap()
        };
        let before = count().await;

        bank_move(&pool, &mv(hs, sc, 10000.0)).await.unwrap();

        assert_eq!(count().await, before);
    }
}
