//! IBKR 轉入: a bank→broker cash movement recorded atomically — the log
//! row, the two balance updates, and the month's `ibkr:<id>` item — with an
//! exact undo through `delete_transfer`.

use sqlx::SqlitePool;

use crate::calc::FieldError;
use crate::models::{CashRole, Currency, IbkrTransfer, NewIbkrTransfer};
use crate::service::ServiceError;
use crate::store::{StoreError, cash_accounts, ibkr as ibkr_store, months, now_timestamp, today};

/// `POST /api/ibkr/transfers`: in one transaction, append the log row,
/// debit the source HKD bank account (the `HS` row unless `from_account_id`
/// names another), credit the IBKR HKD broker account, and record the
/// month's `ibkr:<id>` `IBKR 轉入` adjustment of −amount (skipped while the
/// month row is missing — the suggestion fallback covers it).
pub async fn create_transfer(
    pool: &SqlitePool,
    body: &NewIbkrTransfer,
) -> Result<IbkrTransfer, ServiceError> {
    let mut errors = Vec::new();
    if !body.amount_hkd.is_finite() || body.amount_hkd <= 0.0 {
        errors.push(FieldError::new(
            "amount_hkd",
            "amount must be a positive number",
        ));
    }
    let transfer_date = match &body.transfer_date {
        Some(date) => match chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d") {
            Ok(date) if date > today() => {
                errors.push(FieldError::new(
                    "transfer_date",
                    "date must not be after today",
                ));
                today()
            }
            Ok(date) => date,
            Err(_) => {
                errors.push(FieldError::new("transfer_date", "date must be YYYY-MM-DD"));
                today()
            }
        },
        None => today(),
    };
    if !errors.is_empty() {
        return Err(ServiceError::Validation(errors));
    }

    let now = now_timestamp();
    let mut tx = pool.begin().await.map_err(StoreError::from)?;
    let source = match body.from_account_id {
        Some(id) => cash_accounts::load_one(&mut *tx, id).await?,
        None => cash_accounts::bank_by_label(&mut *tx, "HS")
            .await?
            .ok_or_else(|| {
                ServiceError::Validation(vec![FieldError::new(
                    "from_account_id",
                    "no HS bank account",
                )])
            })?,
    };
    if source.role != CashRole::Bank || source.currency != Currency::Hkd {
        return Err(ServiceError::Validation(vec![FieldError::new(
            "from_account_id",
            "source must be an HKD bank account",
        )]));
    }
    // The overdraft check reads the source inside the transaction so the
    // check and the debit see the same balance.
    if source.amount < body.amount_hkd {
        return Err(ServiceError::Validation(vec![FieldError::new(
            "amount_hkd",
            format!("{} holds only {}", source.label, source.amount),
        )]));
    }
    let broker = cash_accounts::broker(&mut *tx, Currency::Hkd)
        .await?
        .ok_or_else(|| ServiceError::Conflict("no IBKR HKD account".to_string()))?;

    let id = ibkr_store::insert_transfer(
        &mut *tx,
        &transfer_date.to_string(),
        body.amount_hkd,
        Some(source.id),
        Some(broker.id),
    )
    .await?;
    cash_accounts::add_amount(&mut tx, source.id, -body.amount_hkd, &now).await?;
    cash_accounts::add_amount(&mut tx, broker.id, body.amount_hkd, &now).await?;
    months::record_receipt_item(
        &mut tx,
        &format!("{}-01", &transfer_date.to_string()[..7]),
        &format!("ibkr:{id}"),
        "IBKR 轉入",
        -body.amount_hkd,
    )
    .await?;
    tx.commit().await?;
    ibkr_store::load_transfer(pool, id)
        .await
        .map_err(ServiceError::from)
}

/// `DELETE /api/ibkr/transfers/{id}`: in one transaction, reverse the
/// recorded balance moves (a NULL or deleted account skips its side — the
/// same rule as receipt reversals), drop the `ibkr:<id>` month item and its
/// dismissals in any month, then delete the row. `Err(NotFound)` while `id`
/// names no transfer.
pub async fn delete_transfer(pool: &SqlitePool, id: i64) -> Result<(), ServiceError> {
    let transfer = ibkr_store::load_transfer(pool, id).await?;
    let now = now_timestamp();
    let mut tx = pool.begin().await.map_err(StoreError::from)?;
    for (account_id, delta) in [
        (transfer.from_account_id, transfer.amount_hkd),
        (transfer.broker_account_id, -transfer.amount_hkd),
    ] {
        if let Some(account_id) = account_id {
            match cash_accounts::load_one(&mut *tx, account_id).await {
                Ok(_) => cash_accounts::add_amount(&mut tx, account_id, delta, &now).await?,
                Err(StoreError::NotFound(_)) => {}
                Err(error) => return Err(error.into()),
            }
        }
    }
    months::delete_auto_items_by_key(&mut tx, &format!("ibkr:{id}")).await?;
    ibkr_store::delete_transfer(&mut tx, id).await?;
    tx.commit().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calc::build_suggestions;
    use crate::db::connect_memory;
    use std::collections::HashSet;

    async fn seed_bank(pool: &SqlitePool, label: &str, amount: f64) -> i64 {
        let sort: i64 =
            sqlx::query_scalar("SELECT COALESCE(MAX(sort_order), 0) + 1 FROM cash_accounts")
                .fetch_one(pool)
                .await
                .unwrap();
        sqlx::query_scalar(
            "INSERT INTO cash_accounts (label, currency, role, amount, is_default_receipt, \
             sort_order, updated_at) VALUES (?, 'HKD', 'bank', ?, 0, ?, \
             '2026-01-01T00:00:00') RETURNING id",
        )
        .bind(label)
        .bind(amount)
        .bind(sort)
        .fetch_one(pool)
        .await
        .unwrap()
    }

    async fn seed_month(pool: &SqlitePool, month: &str) {
        sqlx::query("INSERT INTO month_stats (month, created_at, updated_at) VALUES (?, ?, ?)")
            .bind(month)
            .bind("2026-01-01T00:00:00")
            .bind("2026-01-01T00:00:00")
            .execute(pool)
            .await
            .unwrap();
    }

    async fn amount(pool: &SqlitePool, id: i64) -> f64 {
        sqlx::query_scalar("SELECT amount FROM cash_accounts WHERE id = ?")
            .bind(id)
            .fetch_one(pool)
            .await
            .unwrap()
    }

    async fn broker_id(pool: &SqlitePool, currency: Currency) -> i64 {
        cash_accounts::broker(pool, currency)
            .await
            .unwrap()
            .expect("broker account")
            .id
    }

    async fn item_count(pool: &SqlitePool, auto_key: &str) -> i64 {
        sqlx::query_scalar("SELECT COUNT(*) FROM month_items WHERE auto_key = ?")
            .bind(auto_key)
            .fetch_one(pool)
            .await
            .unwrap()
    }

    fn body(amount: f64) -> NewIbkrTransfer {
        NewIbkrTransfer {
            amount_hkd: amount,
            transfer_date: None,
            from_account_id: None,
        }
    }

    /// The default source is `HS`: debit it, credit the IBKR HKD broker,
    /// and record the current month's `ibkr:<id>` item.
    #[tokio::test]
    async fn create_debits_hs_credits_the_broker_and_records_the_item() {
        let pool = connect_memory().await.expect("memory db");
        let hs = seed_bank(&pool, "HS", 65608.15).await;
        let sc = seed_bank(&pool, "渣打", 500.0).await;
        let broker = broker_id(&pool, Currency::Hkd).await;
        let month = format!("{}-01", &today().to_string()[..7]);
        seed_month(&pool, &month).await;

        let transfer = create_transfer(
            &pool,
            &NewIbkrTransfer {
                amount_hkd: 50000.0,
                transfer_date: None,
                from_account_id: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(transfer.transfer_date, today().to_string());
        assert_eq!(transfer.from_account_id, Some(hs));
        assert_eq!(transfer.broker_account_id, Some(broker));
        assert_eq!(transfer.from_account_label.as_deref(), Some("HS"));
        assert!(crate::calc::approx_eq(amount(&pool, hs).await, 15608.15));
        assert_eq!(amount(&pool, broker).await, 50000.0);
        assert_eq!(amount(&pool, sc).await, 500.0);

        let (label, item_amount): (String, f64) = sqlx::query_as(
            "SELECT label, amount FROM month_items WHERE month = ? AND auto_key = ?",
        )
        .bind(&month)
        .bind(format!("ibkr:{}", transfer.id))
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!((label.as_str(), item_amount), ("IBKR 轉入", -50000.0));
    }

    /// An explicit `from_account_id` debits that account instead of HS.
    #[tokio::test]
    async fn create_honours_an_explicit_source() {
        let pool = connect_memory().await.expect("memory db");
        let hs = seed_bank(&pool, "HS", 65608.15).await;
        let sc = seed_bank(&pool, "渣打", 800.0).await;

        let transfer = create_transfer(
            &pool,
            &NewIbkrTransfer {
                amount_hkd: 300.0,
                transfer_date: None,
                from_account_id: Some(sc),
            },
        )
        .await
        .unwrap();
        assert_eq!(transfer.from_account_label.as_deref(), Some("渣打"));
        assert_eq!(amount(&pool, sc).await, 500.0);
        assert_eq!(amount(&pool, hs).await, 65608.15);
    }

    /// No HS account while `from_account_id` is omitted → 400.
    #[tokio::test]
    async fn create_without_hs_or_an_explicit_source_is_a_validation_error() {
        let pool = connect_memory().await.expect("memory db");
        seed_bank(&pool, "渣打", 800.0).await;
        let err = create_transfer(&pool, &body(100.0))
            .await
            .expect_err("no HS");
        assert!(matches!(err, ServiceError::Validation(_)));
    }

    /// A broker (or otherwise non-HKD-bank) source is rejected.
    #[tokio::test]
    async fn create_rejects_a_broker_source() {
        let pool = connect_memory().await.expect("memory db");
        let broker = broker_id(&pool, Currency::Hkd).await;
        let err = create_transfer(
            &pool,
            &NewIbkrTransfer {
                amount_hkd: 100.0,
                transfer_date: None,
                from_account_id: Some(broker),
            },
        )
        .await
        .expect_err("broker source");
        assert!(matches!(err, ServiceError::Validation(_)));
        assert_eq!(amount(&pool, broker).await, 0.0);
    }

    /// An unknown `from_account_id` maps to NotFound.
    #[tokio::test]
    async fn create_with_an_unknown_source_is_not_found() {
        let pool = connect_memory().await.expect("memory db");
        seed_bank(&pool, "HS", 1000.0).await;
        let err = create_transfer(
            &pool,
            &NewIbkrTransfer {
                amount_hkd: 100.0,
                transfer_date: None,
                from_account_id: Some(9999),
            },
        )
        .await
        .expect_err("unknown source");
        assert!(matches!(err, ServiceError::Store(StoreError::NotFound(_))));
    }

    /// Zero, negative, and non-finite amounts are rejected before anything
    /// is written.
    #[tokio::test]
    async fn create_rejects_non_positive_amounts() {
        let pool = connect_memory().await.expect("memory db");
        seed_bank(&pool, "HS", 1000.0).await;
        for amount in [0.0, -5.0, f64::NAN, f64::INFINITY] {
            let err = create_transfer(&pool, &body(amount))
                .await
                .expect_err("amount rejected");
            assert!(matches!(err, ServiceError::Validation(_)), "{amount}");
        }
        let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM ibkr_transfers")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(rows, 0);
    }

    /// Bad dates and future dates are rejected.
    #[tokio::test]
    async fn create_rejects_bad_and_future_dates() {
        let pool = connect_memory().await.expect("memory db");
        seed_bank(&pool, "HS", 1000.0).await;
        let future = (today() + chrono::Days::new(1)).to_string();
        for date in ["not-a-date", "2026-13-40", future.as_str()] {
            let err = create_transfer(
                &pool,
                &NewIbkrTransfer {
                    amount_hkd: 100.0,
                    transfer_date: Some(date.to_string()),
                    from_account_id: None,
                },
            )
            .await
            .expect_err("date rejected");
            assert!(matches!(err, ServiceError::Validation(_)), "{date}");
        }
    }

    /// A transfer larger than the source balance is rejected and moves
    /// nothing — no row, no balance change, no month item.
    #[tokio::test]
    async fn create_rejects_an_overdraft_and_changes_nothing() {
        let pool = connect_memory().await.expect("memory db");
        let hs = seed_bank(&pool, "HS", 1000.0).await;
        let broker = broker_id(&pool, Currency::Hkd).await;
        let month = format!("{}-01", &today().to_string()[..7]);
        seed_month(&pool, &month).await;

        let err = create_transfer(&pool, &body(5000.0))
            .await
            .expect_err("overdraft");
        let ServiceError::Validation(fields) = err else {
            panic!("expected validation");
        };
        assert_eq!(fields[0].field, "amount_hkd");
        assert!(fields[0].message.contains("HS"), "{}", fields[0].message);
        assert_eq!(amount(&pool, hs).await, 1000.0);
        assert_eq!(amount(&pool, broker).await, 0.0);
        let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM ibkr_transfers")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(rows, 0);
        let items: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM month_items")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(items, 0);
    }

    /// A transfer dated in a month with no row records no item; once the
    /// row exists the `ibkr:<id>` candidate suggests instead. A legacy
    /// log-only row never suggests.
    #[tokio::test]
    async fn missing_month_row_falls_back_to_a_suggestion() {
        let pool = connect_memory().await.expect("memory db");
        seed_bank(&pool, "HS", 200000.0).await;
        let first = format!("{}-01", &today().to_string()[..7]);

        let created = create_transfer(&pool, &body(50000.0)).await.unwrap();
        assert_eq!(item_count(&pool, &format!("ibkr:{}", created.id)).await, 0);
        // A log-only row in the same month.
        ibkr_store::insert_transfer(&pool, &today().to_string(), 90000.0, None, None)
            .await
            .unwrap();

        seed_month(&pool, &first).await;
        let month = chrono::NaiveDate::parse_from_str(&first, "%Y-%m-%d").unwrap();
        let events = crate::service::months::load_events(&pool, month, 0.0)
            .await
            .unwrap();
        let suggestions = build_suggestions(&first, &events, &HashSet::new(), &HashSet::new());
        let keys: HashSet<&str> = suggestions
            .iter()
            .map(|suggestion| suggestion.auto_key.as_str())
            .collect();
        assert!(keys.contains(format!("ibkr:{}", created.id).as_str()));
        // Only the cash-moving row suggests — the log-only row never does.
        assert_eq!(
            suggestions
                .iter()
                .filter(|suggestion| suggestion.auto_key.starts_with("ibkr:"))
                .count(),
            1
        );
        let suggestion = suggestions
            .iter()
            .find(|suggestion| suggestion.auto_key == format!("ibkr:{}", created.id))
            .unwrap();
        assert_eq!(suggestion.label.as_deref(), Some("IBKR 轉入"));
        assert_eq!(suggestion.amount, -50000.0);
    }

    /// Delete restores both balances exactly and removes the item.
    #[tokio::test]
    async fn delete_reverses_the_movement_and_removes_the_item() {
        let pool = connect_memory().await.expect("memory db");
        let hs = seed_bank(&pool, "HS", 65608.15).await;
        let broker = broker_id(&pool, Currency::Hkd).await;
        let month = format!("{}-01", &today().to_string()[..7]);
        seed_month(&pool, &month).await;

        let transfer = create_transfer(&pool, &body(50000.0)).await.unwrap();
        delete_transfer(&pool, transfer.id).await.unwrap();
        assert!(crate::calc::approx_eq(amount(&pool, hs).await, 65608.15));
        assert_eq!(amount(&pool, broker).await, 0.0);
        assert_eq!(item_count(&pool, &format!("ibkr:{}", transfer.id)).await, 0);
        assert!(matches!(
            ibkr_store::load_transfer(&pool, transfer.id).await,
            Err(StoreError::NotFound(_))
        ));
    }

    /// A log-only row (no recorded accounts) deletes moving nothing.
    #[tokio::test]
    async fn delete_a_log_only_row_moves_no_cash() {
        let pool = connect_memory().await.expect("memory db");
        let hs = seed_bank(&pool, "HS", 1000.0).await;
        let broker = broker_id(&pool, Currency::Hkd).await;
        let id = ibkr_store::insert_transfer(&pool, "2026-06-02", 131000.0, None, None)
            .await
            .unwrap();

        delete_transfer(&pool, id).await.unwrap();
        assert_eq!(amount(&pool, hs).await, 1000.0);
        assert_eq!(amount(&pool, broker).await, 0.0);
    }

    /// With the source account gone, only the broker side reverses.
    #[tokio::test]
    async fn delete_with_a_deleted_source_reverses_only_the_broker() {
        let pool = connect_memory().await.expect("memory db");
        let hs = seed_bank(&pool, "HS", 1000.0).await;
        let broker = broker_id(&pool, Currency::Hkd).await;
        let transfer = create_transfer(
            &pool,
            &NewIbkrTransfer {
                amount_hkd: 400.0,
                transfer_date: None,
                from_account_id: Some(hs),
            },
        )
        .await
        .unwrap();
        cash_accounts::delete(&pool, hs).await.unwrap();

        delete_transfer(&pool, transfer.id).await.unwrap();
        assert_eq!(amount(&pool, broker).await, 0.0);
        assert!(matches!(
            ibkr_store::load_transfer(&pool, transfer.id).await,
            Err(StoreError::NotFound(_))
        ));
    }

    #[tokio::test]
    async fn delete_an_unknown_transfer_is_not_found() {
        let pool = connect_memory().await.expect("memory db");
        let err = delete_transfer(&pool, 9999).await.expect_err("unknown id");
        assert!(matches!(err, ServiceError::Store(StoreError::NotFound(_))));
    }
}
