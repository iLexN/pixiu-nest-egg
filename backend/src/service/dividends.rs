//! The dividend PATCH transition — the route keeps merge+validate; this owns
//! the row update, the receipt credit/reversal, and the `div:<id>` month
//! item.

use chrono::Datelike;
use serde::Serialize;
use sqlx::SqlitePool;

use crate::calc::{
    CandidateKey, DividendFacts, DividendYearRollup, ValidatedDividend, dividend,
    dividend_year_rollups, history_years,
};
use crate::models::{Currency, Dividend, Market};
use crate::service::{ServiceError, receipts};
use crate::store::{StoreError, dividends, months, now_timestamp, today};

/// The credit currency a dividend banks in: HK stocks credit HKD, US stocks
/// land in the USD (IBKR) account.
fn credit_currency(market: Market) -> Currency {
    match market {
        Market::Hk => Currency::Hkd,
        Market::Us => Currency::Usd,
    }
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct DividendSummaryResponse {
    /// The date the response was generated.
    pub today: String,
    /// Pending dividends (no received amount), earliest pay date first.
    pub pending: Vec<Dividend>,
    /// Per-year totals of received amounts, with a per-stock breakdown.
    pub years: Vec<DividendYearRollup>,
    /// Distinct pay-date years in the data plus the current year.
    pub history_years: Vec<i32>,
}

/// The 派息 page read model — every figure derived from the stored dividends
/// on each read. `market` restricts to one market; `None` covers both.
pub async fn summary(
    pool: &SqlitePool,
    market: Option<Market>,
) -> Result<DividendSummaryResponse, StoreError> {
    let today = today();
    let all = dividends::list_for_market(pool, market).await?;
    let facts: Vec<DividendFacts<'_>> = all
        .iter()
        .map(dividends::facts_of)
        .collect::<Result<_, _>>()?;

    let pending: Vec<Dividend> = all
        .iter()
        .filter(|dividend| dividend.received_amount.is_none())
        .cloned()
        .collect();

    Ok(DividendSummaryResponse {
        today: today.to_string(),
        pending,
        years: dividend_year_rollups(&facts),
        history_years: history_years(facts.iter().map(|fact| fact.pay_date.year()), today),
    })
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
    // default HKD account; US → the chosen or default USD/IBKR account) and
    // records the div:<id> month item — the HK-only rule lives in the
    // loader, which returns no event for US. Clearing received_amount
    // reverses the credit and retires the item in whichever month it sits.
    let credited_before = dividends::credit(&mut tx, id).await?;
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
            if let Some(event) = dividends::suggestion_receipt(&mut tx, id).await?
                && let Some(candidate) = dividend(&event)
            {
                months::record_candidate(&mut tx, &candidate).await?;
            }
        }
        (Some(_), None) => {
            receipts::reverse(&mut tx, credited_before, &now).await?;
            dividends::set_credit(&mut tx, id, None).await?;
            months::retire_candidate(&mut tx, &CandidateKey::Div(id)).await?;
        }
        _ => {}
    }
    tx.commit().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calc::ValidatedDividend;
    use crate::db::connect_memory;
    use crate::models::MonthItemCategory;
    use std::collections::HashSet;

    fn this_month() -> chrono::NaiveDate {
        let today = today();
        chrono::NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap()
    }

    async fn seed_month(pool: &SqlitePool, month: chrono::NaiveDate) {
        sqlx::query("INSERT INTO month_stats (month, created_at, updated_at) VALUES (?, ?, ?)")
            .bind(month.to_string())
            .bind("2026-01-01T00:00:00")
            .bind("2026-01-01T00:00:00")
            .execute(pool)
            .await
            .unwrap();
    }

    async fn seed_stock(pool: &SqlitePool, market: &str, code: &str) -> i64 {
        sqlx::query_scalar("INSERT INTO stocks (market, code) VALUES (?, ?) RETURNING id")
            .bind(market)
            .bind(code)
            .fetch_one(pool)
            .await
            .unwrap()
    }

    async fn seed_dividend(
        pool: &SqlitePool,
        stock_id: i64,
        pay_date: &str,
        estimated: Option<f64>,
    ) -> i64 {
        sqlx::query_scalar(
            "INSERT INTO dividends (stock_id, pay_date, estimated_amount, created_at, updated_at) \
             VALUES (?, ?, ?, '2026-01-01T00:00:00', '2026-01-01T00:00:00') RETURNING id",
        )
        .bind(stock_id)
        .bind(pay_date)
        .bind(estimated)
        .fetch_one(pool)
        .await
        .unwrap()
    }

    fn validated(
        pay_date: &str,
        estimated: Option<f64>,
        received: Option<f64>,
    ) -> ValidatedDividend {
        ValidatedDividend {
            pay_date: pay_date.to_string(),
            per_share: None,
            shares_held: None,
            buy_cost: None,
            estimated_amount: estimated,
            received_amount: received,
            received_price: None,
            withholding_tax: None,
        }
    }

    /// The suggestion seen before 收訖 and the item the 收訖 stores agree on
    /// key, month, category and label, with the amount at the 收訖 figure.
    #[tokio::test]
    async fn parity_div() {
        let pool = connect_memory().await.expect("memory db");
        let month = this_month();
        seed_month(&pool, month).await;
        let stock_id = seed_stock(&pool, "HK", "中國銀行").await;
        let pay_date = format!("{}-05", &month.to_string()[..7]);
        let id = seed_dividend(&pool, stock_id, &pay_date, Some(500.0)).await;

        let events = crate::service::months::load_events(&pool, month)
            .await
            .unwrap();
        let suggestions = crate::calc::build_suggestions(
            &month.to_string(),
            &events,
            &HashSet::new(),
            &HashSet::new(),
        );
        let suggestion = suggestions
            .iter()
            .find(|suggestion| suggestion.auto_key == format!("div:{id}"))
            .expect("div suggestion");
        assert_eq!(suggestion.category, MonthItemCategory::Adjustment);
        assert_eq!(suggestion.label.as_deref(), Some("中國銀行"));
        assert_eq!(suggestion.amount, 500.0);

        let received = validated(&pay_date, Some(500.0), Some(500.0));
        update(
            &pool,
            id,
            &DividendUpdate {
                stock_id,
                market: Market::Hk,
                validated: &received,
                note: None,
                existing_received: None,
                bank_in: false,
                account_id: None,
            },
        )
        .await
        .unwrap();

        let (item_month, category, label, amount): (String, String, String, f64) = sqlx::query_as(
            "SELECT month, category, label, amount FROM month_items WHERE auto_key = ?",
        )
        .bind(format!("div:{id}"))
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(
            (
                item_month.as_str(),
                category.as_str(),
                label.as_str(),
                amount
            ),
            (month.to_string().as_str(), "adjustment", "中國銀行", 500.0)
        );
    }

    /// One PATCH moving `pay_date` and clearing `received_amount` retires the
    /// `div:<id>` item in whichever month it sits; a US dividend never
    /// records one.
    #[tokio::test]
    async fn clearing_after_the_pay_date_moved_leaves_no_item() {
        let pool = connect_memory().await.expect("memory db");
        let month = this_month();
        let next = month + chrono::Months::new(1);
        seed_month(&pool, month).await;
        seed_month(&pool, next).await;
        let stock_id = seed_stock(&pool, "HK", "中國銀行").await;
        let pay_date = format!("{}-05", &month.to_string()[..7]);
        let id = seed_dividend(&pool, stock_id, &pay_date, Some(500.0)).await;

        let received = validated(&pay_date, Some(500.0), Some(500.0));
        update(
            &pool,
            id,
            &DividendUpdate {
                stock_id,
                market: Market::Hk,
                validated: &received,
                note: None,
                existing_received: None,
                bank_in: false,
                account_id: None,
            },
        )
        .await
        .unwrap();
        let item_month: String =
            sqlx::query_scalar("SELECT month FROM month_items WHERE auto_key = ?")
                .bind(format!("div:{id}"))
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(item_month, month.to_string());

        let moved = validated(&format!("{}-20", &next.to_string()[..7]), Some(500.0), None);
        update(
            &pool,
            id,
            &DividendUpdate {
                stock_id,
                market: Market::Hk,
                validated: &moved,
                note: None,
                existing_received: Some(500.0),
                bank_in: false,
                account_id: None,
            },
        )
        .await
        .unwrap();
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM month_items WHERE auto_key = ?")
            .bind(format!("div:{id}"))
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 0);

        // A US dividend 收訖 still records no item.
        let us_stock = seed_stock(&pool, "US", "AAPL").await;
        let us_id = seed_dividend(&pool, us_stock, &pay_date, Some(200.0)).await;
        let received = validated(&pay_date, Some(200.0), Some(200.0));
        update(
            &pool,
            us_id,
            &DividendUpdate {
                stock_id: us_stock,
                market: Market::Us,
                validated: &received,
                note: None,
                existing_received: None,
                bank_in: false,
                account_id: None,
            },
        )
        .await
        .unwrap();
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM month_items WHERE auto_key = ?")
            .bind(format!("div:{us_id}"))
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 0);
    }
}
