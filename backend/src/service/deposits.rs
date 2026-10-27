//! Deposit flows that span tables: create records the start month's
//! `dep-start` item, 收訖 is one transaction — the row update, the
//! cash-account credit, and the `dep-end` month item — and delete removes
//! the `dep-start` item and its dismissals together with the row.

use sqlx::SqlitePool;

use crate::calc::ValidatedDeposit;
use crate::models::{Currency, NewDeposit};
use crate::service::{ServiceError, receipts};
use crate::store::{StoreError, deposits, months, now_timestamp};

/// Create a deposit: one transaction — the row insert plus the start month's
/// `dep-start` item (−principal), as the dep-start suggestion would have. A
/// deposit without `start_date`, or whose start month has no row, records no
/// item; the suggestion still surfaces for it later.
pub async fn create(
    pool: &SqlitePool,
    deposit: &ValidatedDeposit,
    body: &NewDeposit,
) -> Result<i64, ServiceError> {
    let sort_order = deposits::next_sort_order(pool).await?;
    let mut tx = pool.begin().await.map_err(StoreError::from)?;
    let id = deposits::insert_deposit(&mut *tx, deposit, sort_order, body).await?;
    if let (Some(start_date), Some(principal)) = (&deposit.start_date, deposit.principal)
        && principal != 0.0
    {
        let month = format!("{}-01", &start_date[..7]);
        let label = deposit.label.as_deref().unwrap_or("定期");
        months::record_receipt_item(
            &mut tx,
            &month,
            &format!("dep-start:{id}"),
            label,
            -principal,
        )
        .await?;
    }
    tx.commit().await?;
    Ok(id)
}

/// What a `POST /api/deposits/{id}/receive` body resolves to once the route
/// maps it.
pub struct ReceiveInput {
    pub received_at: String,
    pub interest: Option<f64>,
    /// Whether the total banks into a cash account.
    pub bank_in: bool,
    /// The chosen account; `None` falls back to the currency default.
    pub account_id: Option<i64>,
}

/// `Err(NotFound)` while `id` names no deposit, `Err(Conflict)` when the
/// deposit was already received.
pub async fn receive(pool: &SqlitePool, id: i64, input: &ReceiveInput) -> Result<(), ServiceError> {
    let deposit = deposits::load_one(pool, id).await?;
    if deposit.received_at.is_some() {
        return Err(ServiceError::Conflict(format!(
            "deposit {id} is already received"
        )));
    }
    // The credit and the dep-end item are always principal + (corrected)
    // interest — not user-entered.
    let amount = deposit.principal.unwrap_or(0.0) + input.interest.unwrap_or(0.0);
    // The dep-end item lands in the end month, as the suggestion would have.
    let month = format!("{}-01", &deposit.end_date[..7]);
    let now = now_timestamp();
    let mut tx = pool.begin().await.map_err(StoreError::from)?;
    let credit = receipts::credit(
        &mut tx,
        Currency::Hkd,
        amount,
        input.bank_in,
        input.account_id,
        &now,
    )
    .await?;
    deposits::mark_received(
        &mut tx,
        id,
        &input.received_at,
        input.interest,
        credit,
        &now,
    )
    .await?;
    let label = deposit.label.as_deref().unwrap_or("定期");
    months::record_receipt_item(&mut tx, &month, &format!("dep-end:{id}"), label, amount).await?;
    tx.commit().await?;
    Ok(())
}

/// Delete an unreceived deposit: one transaction — every `dep-start:<id>`
/// month item and dismissal for that key (in whichever month they sit — an
/// edited start_date does not move the stored item), then the deposit row.
/// `Err(NotFound)` while `id` names no deposit, `Err(Conflict)` when the
/// deposit was already received.
pub async fn delete(pool: &SqlitePool, id: i64) -> Result<(), ServiceError> {
    let deposit = deposits::load_one(pool, id).await?;
    if deposit.received_at.is_some() {
        return Err(ServiceError::Conflict(format!(
            "deposit {id} is already received"
        )));
    }
    let mut tx = pool.begin().await.map_err(StoreError::from)?;
    months::delete_auto_items_by_key(&mut tx, &format!("dep-start:{id}")).await?;
    deposits::delete_deposit(&mut tx, id).await?;
    tx.commit().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calc::{DepositInput, validate_deposit};

    fn validated(input: DepositInput<'_>) -> ValidatedDeposit {
        validate_deposit(input).expect("valid deposit")
    }

    fn body() -> NewDeposit {
        NewDeposit {
            label: Some("SC-1".to_string()),
            bank: Some("SC".to_string()),
            principal: Some(140000.0),
            rate: None,
            interest: Some(1279.0),
            start_date: Some("2099-10-13".to_string()),
            end_date: "2100-02-02".to_string(),
            note1: None,
            note2: None,
        }
    }

    /// The same deposit as `body()`, as calc input — start month 2099-10,
    /// end month 2100-02.
    fn input() -> DepositInput<'static> {
        DepositInput {
            label: Some("SC-1"),
            bank: Some("SC"),
            principal: Some(140000.0),
            rate: None,
            interest: Some(1279.0),
            start_date: Some("2099-10-13"),
            end_date: "2100-02-02",
        }
    }

    async fn seed_months(pool: &SqlitePool, months: &[&str]) {
        for month in months {
            sqlx::query("INSERT INTO month_stats (month, created_at, updated_at) VALUES (?, ?, ?)")
                .bind(month)
                .bind("2026-01-01T00:00:00")
                .bind("2026-01-01T00:00:00")
                .execute(pool)
                .await
                .unwrap();
        }
    }

    async fn item_count(pool: &SqlitePool, auto_key: &str) -> i64 {
        sqlx::query_scalar("SELECT COUNT(*) FROM month_items WHERE auto_key = ?")
            .bind(auto_key)
            .fetch_one(pool)
            .await
            .unwrap()
    }

    async fn dep_start_amount(pool: &SqlitePool, id: i64) -> Option<f64> {
        sqlx::query_scalar(
            "SELECT amount FROM month_items WHERE month = '2099-10-01' AND auto_key = ?",
        )
        .bind(format!("dep-start:{id}"))
        .fetch_optional(pool)
        .await
        .unwrap()
    }

    /// Create records the start month's dep-start item of −principal; a
    /// monthless start month or a missing start_date records none.
    #[tokio::test]
    async fn create_records_the_dep_start_item() {
        let pool = crate::db::connect_memory().await.expect("memory db");
        sqlx::query(
            "INSERT INTO month_stats (month, created_at, updated_at) \
             VALUES ('2099-10-01', '2026-01-01T00:00:00', '2026-01-01T00:00:00')",
        )
        .execute(&pool)
        .await
        .unwrap();

        let id = create(
            &pool,
            &validated(DepositInput {
                label: Some("SC-1"),
                bank: Some("SC"),
                principal: Some(140000.0),
                rate: None,
                interest: Some(1279.0),
                start_date: Some("2099-10-13"),
                end_date: "2100-02-02",
            }),
            &body(),
        )
        .await
        .unwrap();
        assert_eq!(dep_start_amount(&pool, id).await, Some(-140000.0));

        // Monthless start month: the item is skipped, the suggestion remains.
        let mut monthless = body();
        monthless.start_date = Some("2099-11-13".to_string());
        let id = create(
            &pool,
            &validated(DepositInput {
                label: Some("SC-1"),
                bank: Some("SC"),
                principal: Some(140000.0),
                rate: None,
                interest: Some(1279.0),
                start_date: Some("2099-11-13"),
                end_date: "2100-02-02",
            }),
            &monthless,
        )
        .await
        .unwrap();
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM month_items WHERE auto_key = ?")
            .bind(format!("dep-start:{id}"))
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 0);

        // No start_date: no item at all.
        let mut no_start = body();
        no_start.start_date = None;
        let id = create(
            &pool,
            &validated(DepositInput {
                label: Some("SC-1"),
                bank: Some("SC"),
                principal: Some(140000.0),
                rate: None,
                interest: Some(1279.0),
                start_date: None,
                end_date: "2100-02-02",
            }),
            &no_start,
        )
        .await
        .unwrap();
        assert_eq!(dep_start_amount(&pool, id).await, None);
    }

    /// A deposit receive with `bank_in` and no explicit account credits the
    /// currency's default-receipt account.
    #[tokio::test]
    async fn receive_credits_the_default_account() {
        let pool = crate::db::connect_memory().await.expect("memory db");
        let hs: i64 = sqlx::query_scalar(
            "INSERT INTO cash_accounts (label, currency, role, amount, is_default_receipt, \
             sort_order, updated_at) VALUES ('HS', 'HKD', 'bank', 100.0, 1, 1, \
             '2026-01-01T00:00:00') RETURNING id",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        let deposit_id: i64 = sqlx::query_scalar(
            "INSERT INTO deposits (label, bank, principal, interest, end_date, sort_order, \
             created_at, updated_at) VALUES ('SC-1', 'SC', 110000, 993, '2099-10-12', 1, \
             '2026-01-01T00:00:00', '2026-01-01T00:00:00') RETURNING id",
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        receive(
            &pool,
            deposit_id,
            &ReceiveInput {
                received_at: "2099-10-12".to_string(),
                interest: Some(993.0),
                bank_in: true,
                account_id: None,
            },
        )
        .await
        .unwrap();

        let stored = deposits::load_one(&pool, deposit_id).await.unwrap();
        assert_eq!(stored.received_at.as_deref(), Some("2099-10-12"));
        let amount: f64 = sqlx::query_scalar("SELECT amount FROM cash_accounts WHERE id = ?")
            .bind(hs)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(amount, 100.0 + 110993.0);
        // Both halves of the credit are recorded.
        let (account, credited): (i64, f64) = sqlx::query_as(
            "SELECT credited_account_id, credited_amount FROM deposits WHERE id = ?",
        )
        .bind(deposit_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!((account, credited), (hs, 110993.0));
    }

    /// Deleting an unreceived deposit removes its dep-start item and any
    /// dismissal for the key — in whichever month they sit — while the
    /// month's other items stay.
    #[tokio::test]
    async fn delete_removes_the_dep_start_item_and_dismissals() {
        let pool = crate::db::connect_memory().await.expect("memory db");
        seed_months(&pool, &["2099-10-01"]).await;
        let id = create(&pool, &validated(input()), &body()).await.unwrap();
        assert_eq!(dep_start_amount(&pool, id).await, Some(-140000.0));

        sqlx::query(
            "INSERT INTO month_items (month, category, label, amount, created_at) \
             VALUES ('2099-10-01', 'adjustment', 'unrelated', 5.0, '2026-01-01T00:00:00')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO month_item_dismissals (month, auto_key, created_at) \
             VALUES ('2099-11-01', ?, '2026-01-01T00:00:00')",
        )
        .bind(format!("dep-start:{id}"))
        .execute(&pool)
        .await
        .unwrap();

        delete(&pool, id).await.unwrap();

        assert!(matches!(
            deposits::load_one(&pool, id).await,
            Err(StoreError::NotFound(_))
        ));
        assert_eq!(dep_start_amount(&pool, id).await, None);
        assert_eq!(item_count(&pool, &format!("dep-start:{id}")).await, 0);
        let dismissals: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM month_item_dismissals WHERE auto_key = ?")
                .bind(format!("dep-start:{id}"))
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(dismissals, 0);
        let remaining: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM month_items WHERE month = '2099-10-01'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(remaining, 1);
    }

    /// The stored dep-start item stays in the month it was recorded — an
    /// edited start_date does not move it — so delete matches by key alone.
    #[tokio::test]
    async fn delete_removes_the_item_after_the_start_date_moved() {
        let pool = crate::db::connect_memory().await.expect("memory db");
        seed_months(&pool, &["2099-10-01"]).await;
        let id = create(&pool, &validated(input()), &body()).await.unwrap();
        sqlx::query("UPDATE deposits SET start_date = '2099-12-01' WHERE id = ?")
            .bind(id)
            .execute(&pool)
            .await
            .unwrap();

        delete(&pool, id).await.unwrap();

        assert_eq!(item_count(&pool, &format!("dep-start:{id}")).await, 0);
        assert!(matches!(
            deposits::load_one(&pool, id).await,
            Err(StoreError::NotFound(_))
        ));
    }

    /// A 收訖 deposit refuses deletion: the row, its dep-end item and the
    /// cash credit stay as they were.
    #[tokio::test]
    async fn delete_a_received_deposit_conflicts() {
        let pool = crate::db::connect_memory().await.expect("memory db");
        seed_months(&pool, &["2099-10-01", "2100-02-01"]).await;
        let id = create(&pool, &validated(input()), &body()).await.unwrap();
        receive(
            &pool,
            id,
            &ReceiveInput {
                received_at: "2100-02-02".to_string(),
                interest: Some(1279.0),
                bank_in: false,
                account_id: None,
            },
        )
        .await
        .unwrap();

        let err = delete(&pool, id).await.expect_err("received deposit");
        assert!(matches!(err, ServiceError::Conflict(_)));

        assert!(deposits::load_one(&pool, id).await.is_ok());
        let dep_end: Option<f64> = sqlx::query_scalar(
            "SELECT amount FROM month_items WHERE month = '2100-02-01' AND auto_key = ?",
        )
        .bind(format!("dep-end:{id}"))
        .fetch_optional(&pool)
        .await
        .unwrap();
        assert_eq!(dep_end, Some(141279.0));
    }

    #[tokio::test]
    async fn delete_an_unknown_deposit_is_not_found() {
        let pool = crate::db::connect_memory().await.expect("memory db");
        let err = delete(&pool, 9999).await.expect_err("unknown id");
        assert!(matches!(err, ServiceError::Store(StoreError::NotFound(_))));
    }
}
