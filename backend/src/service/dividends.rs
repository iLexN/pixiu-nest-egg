//! The dividend PATCH transition — the route keeps merge+validate; this owns
//! the row update, the receipt credit/reversal, and the `div:<id>` month
//! item.

use sqlx::SqlitePool;

use crate::calc::ValidatedDividend;
use crate::models::{Currency, Market};
use crate::service::{ServiceError, receipts};
use crate::store::{StoreError, dividends, months, now_timestamp};

/// The credit currency a dividend banks in: HK stocks credit HKD, US stocks
/// land in the USD (IBKR) account.
fn credit_currency(market: Market) -> Currency {
    match market {
        Market::Hk => Currency::Hkd,
        Market::Us => Currency::Usd,
    }
}

/// The dividend PATCH transition inputs — the route keeps merge+validate and
/// passes the resolved values here.
pub struct DividendUpdate<'a> {
    pub stock_id: i64,
    pub market: Market,
    pub validated: &'a ValidatedDividend,
    pub note: Option<String>,
    /// The dividend's `received_amount` before the update.
    pub existing_received: Option<f64>,
    /// The stock's 股票代碼, for the `div:<id>` month item's label.
    pub code: &'a str,
    pub bank_in: bool,
    /// The chosen receipt account; `None` falls back to the currency default.
    pub account_id: Option<i64>,
}

/// The (received_amount before, after) pair drives credit vs reversal.
pub async fn update(
    pool: &SqlitePool,
    id: i64,
    input: &DividendUpdate<'_>,
) -> Result<(), ServiceError> {
    let now = now_timestamp();
    let mut tx = pool.begin().await.map_err(StoreError::from)?;
    dividends::update_dividend(
        &mut tx,
        id,
        input.stock_id,
        input.validated,
        input.note.clone(),
        &now,
    )
    .await?;

    // 收訖: received_amount NULL → set banks the amount (HK → the chosen or
    // default HKD account; US → the chosen or default USD/IBKR account) and,
    // for HK, records the div:<id> month item. Clearing received_amount
    // reverses through the recorded credit.
    let credited_before = dividends::credit(&mut tx, id).await?;
    let auto_key = format!("div:{id}");
    let month = format!("{}-01", &input.validated.pay_date[..7]);
    match (input.existing_received, input.validated.received_amount) {
        (None, Some(amount)) => {
            let credit = receipts::credit(
                &mut tx,
                credit_currency(input.market),
                amount,
                input.bank_in,
                input.account_id,
                &now,
            )
            .await?;
            dividends::set_credit(&mut tx, id, credit).await?;
            // US dividends land in IBKR, not 活期 — no adjustment item.
            if input.market == Market::Hk {
                months::record_receipt_item(&mut tx, &month, &auto_key, input.code, amount).await?;
            }
        }
        (Some(_), None) => {
            receipts::reverse(&mut tx, credited_before, &now).await?;
            dividends::set_credit(&mut tx, id, None).await?;
            months::delete_auto_item(&mut tx, &month, &auto_key).await?;
        }
        _ => {}
    }
    tx.commit().await?;
    Ok(())
}
