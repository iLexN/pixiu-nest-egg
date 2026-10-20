//! The deposit 收訖 flow: marking a deposit received is one transaction —
//! the row update, the cash-account credit, and the `dep-end` month item.

use sqlx::SqlitePool;

use crate::models::Currency;
use crate::service::{ServiceError, receipts};
use crate::store::{StoreError, deposits, months, now_timestamp};

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
