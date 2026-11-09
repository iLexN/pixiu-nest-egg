//! Deposit flows that span tables: placement (create and the 轉為定期
//! conversion) validates the body, inserts the row, optionally debits the principal from a
//! chosen 活期 account, and records the start month's `dep-start` item;
//! 收訖 is one transaction — the row update, the cash-account credit, and
//! the `dep-end` month item — and delete reverses the debit and removes
//! the `dep-start` item and its dismissals together with the row.

use sqlx::SqlitePool;

use crate::calc::{DepositInput, FieldError, ValidatedDeposit, validate_deposit};
use crate::models::{CashRole, Currency, NewDeposit};
use crate::service::{ServiceError, receipts};
use crate::store::{StoreError, cash_accounts, deposits, forecast, months, now_timestamp};

/// The shared placement write, inside the caller's transaction: the row
/// insert, the optional 活期 debit of the principal from
/// `debit_account_id`, then the start month's `dep-start:<id>` item
/// (−principal), as the dep-start suggestion would have. A deposit without
/// `start_date`, or whose start month has no row, records no item; the
/// suggestion still surfaces for it later.
///
/// The debit names an HKD `bank` account holding at least the principal —
/// `Err(Validation)` on `debit_account_id` otherwise, unknown id →
/// `Err(NotFound)` — and applies only to a positive principal on an
/// unreceived insert: a row `insert_deposit` auto-received (past
/// `end_date`) is a historic entry and never debits.
async fn place(
    conn: &mut sqlx::SqliteConnection,
    deposit: &ValidatedDeposit,
    body: &NewDeposit,
) -> Result<i64, ServiceError> {
    let sort_order = deposits::next_sort_order_tx(&mut *conn).await?;
    let id = deposits::insert_deposit(&mut *conn, deposit, sort_order, body).await?;
    if let Some(account_id) = body.debit_account_id
        && let Some(principal) = deposit.principal
        && principal > 0.0
        && !deposits::is_received(&mut *conn, id).await?
    {
        let account = cash_accounts::load_one(&mut *conn, account_id).await?;
        if account.role != CashRole::Bank || account.currency != Currency::Hkd {
            return Err(ServiceError::Validation(vec![FieldError::new(
                "debit_account_id",
                "must be an HKD bank account",
            )]));
        }
        // The overdraft check reads the account inside the transaction so
        // the check and the debit see the same balance.
        if account.amount < principal {
            return Err(ServiceError::Validation(vec![FieldError::new(
                "debit_account_id",
                format!("{} holds only {}", account.label, account.amount),
            )]));
        }
        let now = now_timestamp();
        cash_accounts::add_amount(&mut *conn, account.id, -principal, &now).await?;
        deposits::set_debit(&mut *conn, id, account.id, principal, &now).await?;
    }
    if let (Some(start_date), Some(principal)) = (&deposit.start_date, deposit.principal)
        && principal != 0.0
    {
        let month = format!("{}-01", &start_date[..7]);
        let label = deposit.label.as_deref().unwrap_or("定期");
        months::record_receipt_item(
            &mut *conn,
            &month,
            &format!("dep-start:{id}"),
            label,
            -principal,
        )
        .await?;
    }
    Ok(id)
}

/// The placement's domain validation — `Err(Validation)` with the field
/// errors, before any transaction opens.
fn validate(body: &NewDeposit) -> Result<ValidatedDeposit, ServiceError> {
    validate_deposit(DepositInput {
        label: body.label.as_deref(),
        bank: body.bank.as_deref(),
        principal: body.principal,
        rate: body.rate,
        interest: body.interest,
        start_date: body.start_date.as_deref(),
        end_date: &body.end_date,
    })
    .map_err(ServiceError::Validation)
}

/// Create a deposit: validate, then one transaction — [`place`].
pub async fn create(pool: &SqlitePool, body: &NewDeposit) -> Result<i64, ServiceError> {
    let deposit = validate(body)?;
    let mut tx = pool.begin().await.map_err(StoreError::from)?;
    let id = place(&mut tx, &deposit, body).await?;
    tx.commit().await?;
    Ok(id)
}

/// 轉為定期: load the plan (`Err(NotFound)` while missing; `Err(Conflict)`
/// unless it is a deposit plan), default the unset `bank` from its kind and
/// `principal` from −amount, validate, then [`place`] the deposit and delete
/// the plan in one transaction — a failed placement leaves the plan in
/// place, and a failed delete can never leave both (which would
/// double-count in the forecast).
pub async fn convert_plan(
    pool: &SqlitePool,
    item_id: i64,
    mut body: NewDeposit,
) -> Result<i64, ServiceError> {
    let item = forecast::load_item(pool, item_id).await?;
    if item.kind.deposit_lag().is_none() {
        return Err(ServiceError::Conflict(format!(
            "forecast item {item_id} is not a deposit plan"
        )));
    }
    if body.bank.is_none() {
        body.bank = item.kind.default_bank().map(str::to_string);
    }
    if body.principal.is_none() {
        body.principal = Some(-item.amount);
    }
    let deposit = validate(&body)?;
    let mut tx = pool.begin().await.map_err(StoreError::from)?;
    let id = place(&mut tx, &deposit, &body).await?;
    forecast::delete_item_in_tx(&mut tx, item_id).await?;
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

/// Delete an unreceived deposit: one transaction — the reversal of the
/// recorded placement debit (the stored amount added back to the stored
/// account; skipped when the account is gone), every `dep-start:<id>`
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
    let now = now_timestamp();
    let mut tx = pool.begin().await.map_err(StoreError::from)?;
    let debit = deposits::load_debit(&mut tx, id).await?;
    receipts::refund(&mut tx, debit, &now).await?;
    months::delete_auto_items_by_key(&mut tx, &format!("dep-start:{id}")).await?;
    deposits::delete_deposit(&mut tx, id).await?;
    tx.commit().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
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
            debit_account_id: None,
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

        let id = create(&pool, &body()).await.unwrap();
        assert_eq!(dep_start_amount(&pool, id).await, Some(-140000.0));

        // Monthless start month: the item is skipped, the suggestion remains.
        let mut monthless = body();
        monthless.start_date = Some("2099-11-13".to_string());
        let id = create(&pool, &monthless).await.unwrap();
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM month_items WHERE auto_key = ?")
            .bind(format!("dep-start:{id}"))
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 0);

        // No start_date: no item at all.
        let mut no_start = body();
        no_start.start_date = None;
        let id = create(&pool, &no_start).await.unwrap();
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
        let id = create(&pool, &body()).await.unwrap();
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
        let id = create(&pool, &body()).await.unwrap();
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
        let id = create(&pool, &body()).await.unwrap();
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

    async fn seed_bank(pool: &SqlitePool, label: &str, amount: f64, default: bool) -> i64 {
        let sort: i64 =
            sqlx::query_scalar("SELECT COALESCE(MAX(sort_order), 0) + 1 FROM cash_accounts")
                .fetch_one(pool)
                .await
                .unwrap();
        sqlx::query_scalar(
            "INSERT INTO cash_accounts (label, currency, role, amount, is_default_receipt, \
             sort_order, updated_at) VALUES (?, 'HKD', 'bank', ?, ?, ?, \
             '2026-01-01T00:00:00') RETURNING id",
        )
        .bind(label)
        .bind(amount)
        .bind(default as i64)
        .bind(sort)
        .fetch_one(pool)
        .await
        .unwrap()
    }

    async fn account_amount(pool: &SqlitePool, id: i64) -> f64 {
        sqlx::query_scalar("SELECT amount FROM cash_accounts WHERE id = ?")
            .bind(id)
            .fetch_one(pool)
            .await
            .unwrap()
    }

    async fn stored_debit(pool: &SqlitePool, id: i64) -> (Option<i64>, Option<f64>) {
        sqlx::query_as("SELECT debited_account_id, debited_amount FROM deposits WHERE id = ?")
            .bind(id)
            .fetch_one(pool)
            .await
            .unwrap()
    }

    /// `debit_account_id` debits the principal and stores the debit on the
    /// row, alongside the dep-start item; without it nothing moves and the
    /// debit fields stay empty.
    #[tokio::test]
    async fn create_debits_the_chosen_account_and_records_it() {
        let pool = crate::db::connect_memory().await.expect("memory db");
        seed_months(&pool, &["2099-10-01"]).await;
        let hs = seed_bank(&pool, "HS", 200000.0, true).await;
        let sc = seed_bank(&pool, "渣打", 800.0, false).await;

        let mut debit_body = body();
        debit_body.debit_account_id = Some(hs);
        let id = create(&pool, &debit_body).await.unwrap();
        assert_eq!(account_amount(&pool, hs).await, 60000.0);
        assert_eq!(account_amount(&pool, sc).await, 800.0);
        assert_eq!(stored_debit(&pool, id).await, (Some(hs), Some(140000.0)));
        assert_eq!(dep_start_amount(&pool, id).await, Some(-140000.0));

        let id = create(&pool, &body()).await.unwrap();
        assert_eq!(stored_debit(&pool, id).await, (None, None));
        assert_eq!(account_amount(&pool, hs).await, 60000.0);
    }

    /// A debit naming an account below the principal fails before commit:
    /// no deposit, no item, no balance change. An unknown id is a
    /// `StoreError::NotFound`.
    #[tokio::test]
    async fn create_rejects_an_overdraft_and_writes_nothing() {
        let pool = crate::db::connect_memory().await.expect("memory db");
        seed_months(&pool, &["2099-10-01"]).await;
        let hs = seed_bank(&pool, "HS", 50000.0, true).await;

        let mut body = body();
        body.debit_account_id = Some(hs);
        let err = create(&pool, &body).await.expect_err("overdraft");
        let ServiceError::Validation(fields) = err else {
            panic!("expected validation");
        };
        assert_eq!(fields[0].field, "debit_account_id");
        assert!(fields[0].message.contains("HS"), "{}", fields[0].message);

        assert_eq!(account_amount(&pool, hs).await, 50000.0);
        let deposits: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM deposits")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(deposits, 0);
        let items: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM month_items")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(items, 0);

        body.debit_account_id = Some(9999);
        let err = create(&pool, &body).await.expect_err("unknown account");
        assert!(matches!(err, ServiceError::Store(StoreError::NotFound(_))));
    }

    /// A broker or USD `debit_account_id` is a validation error on the
    /// field, and nothing is stored. (The schema makes a non-HKD bank row
    /// impossible, so the USD account here is the seeded USD broker.)
    #[tokio::test]
    async fn create_rejects_a_non_hkd_bank_debit() {
        let pool = crate::db::connect_memory().await.expect("memory db");
        let broker: i64 = sqlx::query_scalar(
            "SELECT id FROM cash_accounts WHERE role = 'broker' AND currency = 'HKD'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        let usd: i64 = sqlx::query_scalar(
            "SELECT id FROM cash_accounts WHERE role = 'broker' AND currency = 'USD'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        for account in [broker, usd] {
            let mut body = body();
            body.debit_account_id = Some(account);
            let err = create(&pool, &body).await.expect_err("non-HKD-bank debit");
            let ServiceError::Validation(fields) = err else {
                panic!("expected validation");
            };
            assert_eq!(fields[0].field, "debit_account_id");
        }
        let deposits: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM deposits")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(deposits, 0);
    }

    /// An insert auto-received by a past `end_date` is a historic entry —
    /// it never debits even when `debit_account_id` is given.
    #[tokio::test]
    async fn create_an_auto_received_insert_never_debits() {
        let pool = crate::db::connect_memory().await.expect("memory db");
        let hs = seed_bank(&pool, "HS", 200000.0, true).await;

        let mut body = body();
        body.start_date = Some("2019-10-13".to_string());
        body.end_date = "2020-02-02".to_string();
        body.debit_account_id = Some(hs);
        let id = create(&pool, &body).await.unwrap();

        let stored = deposits::load_one(&pool, id).await.unwrap();
        assert!(stored.received_at.is_some());
        assert_eq!(stored_debit(&pool, id).await, (None, None));
        assert_eq!(account_amount(&pool, hs).await, 200000.0);
    }

    /// Delete adds the recorded debit back to the recorded account — the
    /// receipt default moving to 渣打 in between changes nothing.
    #[tokio::test]
    async fn delete_restores_the_debited_account() {
        let pool = crate::db::connect_memory().await.expect("memory db");
        seed_months(&pool, &["2099-10-01"]).await;
        let hs = seed_bank(&pool, "HS", 200000.0, true).await;
        let sc = seed_bank(&pool, "渣打", 800.0, false).await;

        let mut body = body();
        body.debit_account_id = Some(hs);
        let id = create(&pool, &body).await.unwrap();
        assert_eq!(account_amount(&pool, hs).await, 60000.0);

        sqlx::query("UPDATE cash_accounts SET is_default_receipt = 0 WHERE id = ?")
            .bind(hs)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("UPDATE cash_accounts SET is_default_receipt = 1 WHERE id = ?")
            .bind(sc)
            .execute(&pool)
            .await
            .unwrap();

        delete(&pool, id).await.unwrap();
        assert_eq!(account_amount(&pool, hs).await, 200000.0);
        assert_eq!(account_amount(&pool, sc).await, 800.0);
    }

    async fn deposit_count(pool: &SqlitePool) -> i64 {
        sqlx::query_scalar("SELECT COUNT(*) FROM deposits")
            .fetch_one(pool)
            .await
            .unwrap()
    }

    async fn seed_plan(pool: &SqlitePool, kind: &str, amount: f64) -> i64 {
        forecast::insert_item(pool, "2099-10-01", kind, amount, &None, &None, 1)
            .await
            .unwrap()
    }

    /// A body failing domain validation is rejected before anything is
    /// written.
    #[tokio::test]
    async fn create_rejects_an_invalid_body_and_writes_nothing() {
        let pool = crate::db::connect_memory().await.expect("memory db");
        let mut bad = body();
        bad.end_date = "2100-13-40".to_string();

        let err = create(&pool, &bad).await.expect_err("invalid end_date");
        let ServiceError::Validation(fields) = err else {
            panic!("expected validation");
        };
        assert_eq!(fields[0].field, "end_date");
        assert_eq!(deposit_count(&pool).await, 0);
    }

    /// An unset bank/principal defaults from the plan: `sc_deposit` → `SC`,
    /// principal = −amount; the plan is consumed.
    #[tokio::test]
    async fn convert_plan_defaults_bank_and_principal_from_the_plan() {
        let pool = crate::db::connect_memory().await.expect("memory db");
        let plan = seed_plan(&pool, "sc_deposit", -180000.0).await;
        let mut bare = body();
        bare.bank = None;
        bare.principal = None;

        let id = convert_plan(&pool, plan, bare).await.unwrap();

        let stored = deposits::load_one(&pool, id).await.unwrap();
        assert_eq!(stored.bank.as_deref(), Some("SC"));
        assert_eq!(stored.principal, Some(180000.0));
        assert!(matches!(
            forecast::load_item(&pool, plan).await,
            Err(StoreError::NotFound(_))
        ));
    }

    /// A non-deposit plan is a conflict and stays; a missing plan is not
    /// found; an invalid body is a validation error that keeps the plan.
    #[tokio::test]
    async fn convert_plan_error_precedence_keeps_the_plan() {
        let pool = crate::db::connect_memory().await.expect("memory db");
        let tax = seed_plan(&pool, "tax", -3000.0).await;
        let mut bad = body();
        bad.end_date = "not-a-date".to_string();

        // The kind check wins over the invalid body.
        let err = convert_plan(&pool, tax, bad.clone())
            .await
            .expect_err("tax");
        assert!(matches!(err, ServiceError::Conflict(_)));
        assert!(forecast::load_item(&pool, tax).await.is_ok());

        let err = convert_plan(&pool, 999_999, body())
            .await
            .expect_err("missing");
        assert!(matches!(err, ServiceError::Store(StoreError::NotFound(_))));

        let hs = seed_plan(&pool, "hs_deposit", -130000.0).await;
        let err = convert_plan(&pool, hs, bad).await.expect_err("invalid");
        let ServiceError::Validation(fields) = err else {
            panic!("expected validation");
        };
        assert_eq!(fields[0].field, "end_date");
        assert!(forecast::load_item(&pool, hs).await.is_ok());
        assert_eq!(deposit_count(&pool).await, 0);
    }

    /// The debited account deleted in between: the deposit still deletes
    /// and no remaining account changes.
    #[tokio::test]
    async fn delete_with_the_debited_account_gone_moves_nothing() {
        let pool = crate::db::connect_memory().await.expect("memory db");
        seed_months(&pool, &["2099-10-01"]).await;
        let hs = seed_bank(&pool, "HS", 50.0, true).await;
        let sc = seed_bank(&pool, "渣打", 200000.0, false).await;

        let mut body = body();
        body.debit_account_id = Some(sc);
        let id = create(&pool, &body).await.unwrap();
        cash_accounts::delete(&pool, sc).await.unwrap();

        delete(&pool, id).await.unwrap();
        assert_eq!(account_amount(&pool, hs).await, 50.0);
        assert!(matches!(
            deposits::load_one(&pool, id).await,
            Err(StoreError::NotFound(_))
        ));
    }
}
