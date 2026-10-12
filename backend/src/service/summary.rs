use serde::Serialize;
use sqlx::SqlitePool;

use crate::calc::{
    MarketTotals, MpfPoint, RollupInput, SectorRollup, StockSummary, TradeFacts, market_totals,
    mpf_last_month, mpf_max, sector_rollup, summarize,
};
use crate::models::{Market, MarketFigures, Stock};
use crate::store::StoreError;

/// Shown next to 加權平均買入單價 so the definition travels with the number.
pub const AVERAGE_PRICE_DEFINITION: &str =
    "加權平均買入單價 = 總買入成本 ÷ Σ BUY 股數 (shares bought, not shares held)";

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct StockRow {
    #[serde(flatten)]
    pub stock: Stock,
    #[serde(flatten)]
    pub summary: StockSummary,
    pub trade_count: usize,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct SummaryResponse {
    pub market: Market,
    pub average_price_definition: &'static str,
    pub stocks: Vec<StockRow>,
    pub sectors: Vec<SectorRollup>,
    pub totals: MarketTotals,
    /// Latest recorded figures in the previous calendar month, if any.
    pub last_month: Option<MarketFigures>,
    /// All-time maxima over the imported seed marks, every history row, plus
    /// the current values. Percent and amount are independent and may come
    /// from different moments.
    pub max: Option<MarketFigures>,
}

/// Every figure here is derived from the stored trades on each read.
pub async fn build(pool: &SqlitePool, market: Market) -> Result<SummaryResponse, StoreError> {
    let (stock_rows, sectors, totals) = summarize_market(pool, market).await?;

    // Record today's totals, then derive 上月/最高 from the market's history.
    // An unpriced market records nothing: its rows would have no market value.
    let today = crate::store::today();
    if let Some(market_value) = totals.market_value {
        crate::store::market_history::record(
            pool,
            market,
            (totals.buy_cost_priced, market_value),
            today,
        )
        .await?;
    }
    let history = crate::store::market_history::load(pool, market).await?;
    let figures = |figures: crate::models::MpfFigures| MarketFigures {
        percent: figures.rate,
        amount: figures.gain,
    };
    let last_month = mpf_last_month(&history, today).map(figures);
    // The workbook's cached max cells seed `app_meta` marks that floor the
    // derived 最高 — real records can still exceed them. 最高 reports
    // whenever marks or history exist, even while the market is unpriced.
    let seed_percent = crate::store::meta::meta_f64(
        pool,
        &crate::store::market_history::seed_max_percent_key(market),
    )
    .await?;
    let seed_amount = crate::store::meta::meta_f64(
        pool,
        &crate::store::market_history::seed_max_amount_key(market),
    )
    .await?;
    let current = totals.market_value.map(|market_value| MpfPoint {
        recorded_on: today,
        contributions: totals.buy_cost_priced,
        balance: market_value,
    });
    let max = if current.is_some()
        || !history.is_empty()
        || seed_percent.is_some()
        || seed_amount.is_some()
    {
        Some(figures(mpf_max(
            &history,
            current,
            seed_percent,
            seed_amount,
        )))
    } else {
        None
    };

    Ok(SummaryResponse {
        market,
        average_price_definition: AVERAGE_PRICE_DEFINITION,
        stocks: stock_rows,
        sectors,
        totals,
        last_month,
        max,
    })
}

/// The stock rows, sector rollup and market totals, without the
/// market-history upsert — the part of `build` that is free of side effects.
async fn summarize_market(
    pool: &SqlitePool,
    market: Market,
) -> Result<(Vec<StockRow>, Vec<SectorRollup>, MarketTotals), StoreError> {
    let stocks = crate::store::stocks::load_stocks(pool, Some(market)).await?;

    let facts = crate::store::trades::market_trade_facts(pool, market).await?;
    // 累計派息 counts only received amounts; pending estimates are excluded.
    let dividends = crate::store::dividends::received_totals(pool, market).await?;

    let stock_rows: Vec<StockRow> = stocks
        .into_iter()
        .map(|stock| {
            let own: Vec<TradeFacts> = facts
                .iter()
                .filter(|(stock_id, _)| *stock_id == stock.id)
                .map(|(_, fact)| *fact)
                .collect();
            let dividend_total = dividends.get(&stock.id).copied().unwrap_or(0.0);
            let summary = summarize(&own, stock.manual_price, dividend_total);
            StockRow {
                stock,
                summary,
                trade_count: own.len(),
            }
        })
        .collect();

    let rollup_inputs: Vec<RollupInput<'_>> = stock_rows
        .iter()
        .map(|row| RollupInput {
            code: &row.stock.code,
            sector: row.stock.sector.as_deref(),
            total_buy_cost: row.summary.total_buy_cost,
            market_value: row.summary.market_value,
            dividends_received: row.summary.dividends_received,
        })
        .collect();

    let sectors = sector_rollup(&rollup_inputs);
    let totals = market_totals(&rollup_inputs);
    Ok((stock_rows, sectors, totals))
}

/// A market's live 總市值 with no market-history side effect — what the
/// month-stat live totals need.
pub async fn market_value_only(
    pool: &SqlitePool,
    market: Market,
) -> Result<Option<f64>, StoreError> {
    Ok(summarize_market(pool, market).await?.2.market_value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connect_memory;
    use chrono::Datelike;

    async fn add_stock(pool: &SqlitePool, market: &str, code: &str, price: Option<f64>) -> i64 {
        sqlx::query_scalar(
            "INSERT INTO stocks (market, code, manual_price, is_active, sort_order) \
             VALUES (?, ?, ?, 1, 1) RETURNING id",
        )
        .bind(market)
        .bind(code)
        .bind(price)
        .fetch_one(pool)
        .await
        .expect("insert stock")
    }

    async fn add_buy(pool: &SqlitePool, stock_id: i64, shares: f64, total: f64) {
        sqlx::query(
            "INSERT INTO trades (stock_id, trade_type, trade_date, shares, unit_price, fee, \
             total, input_mode, created_at, updated_at) \
             VALUES (?, 'BUY', '2026-01-01', ?, ?, 0.0, ?, 'US_FEE', 'x', 'x')",
        )
        .bind(stock_id)
        .bind(shares)
        .bind(total / shares)
        .bind(total)
        .execute(pool)
        .await
        .expect("insert trade");
    }

    async fn seed_history(pool: &SqlitePool, recorded_on: &str, cost: f64, value: f64) {
        sqlx::query(
            "INSERT INTO market_history \
             (market, recorded_on, buy_cost_priced, market_value, synthetic) \
             VALUES ('HK', ?, ?, ?, 0)",
        )
        .bind(recorded_on)
        .bind(cost)
        .bind(value)
        .execute(pool)
        .await
        .expect("seed history");
    }

    /// The last day of the calendar month before today.
    fn previous_month_end() -> String {
        let today = crate::store::today();
        let first = chrono::NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap();
        (first - chrono::Days::new(1)).to_string()
    }

    fn near(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    async fn history_rows(pool: &SqlitePool) -> Vec<(String, f64, f64)> {
        sqlx::query_as(
            "SELECT recorded_on, buy_cost_priced, market_value FROM market_history \
             WHERE market = 'HK' ORDER BY recorded_on",
        )
        .fetch_all(pool)
        .await
        .expect("history")
    }

    #[tokio::test]
    async fn build_records_todays_totals() {
        let pool = connect_memory().await.unwrap();
        let id = add_stock(&pool, "HK", "中國銀行", Some(10.0)).await;
        add_buy(&pool, id, 100.0, 900.0).await;

        build(&pool, Market::Hk).await.expect("build");

        let today = crate::store::today().to_string();
        let rows = history_rows(&pool).await;
        assert_eq!(rows, vec![(today, 900.0, 1000.0)]);
    }

    #[tokio::test]
    async fn previous_month_row_yields_last_month() {
        let pool = connect_memory().await.unwrap();
        let id = add_stock(&pool, "HK", "中國銀行", Some(10.0)).await;
        add_buy(&pool, id, 100.0, 900.0).await;
        seed_history(&pool, &previous_month_end(), 500.0, 600.0).await;

        let response = build(&pool, Market::Hk).await.expect("build");

        let last_month = response.last_month.expect("last month");
        assert!(near(last_month.percent.unwrap(), 0.2));
        assert!(near(last_month.amount, 100.0));
    }

    #[tokio::test]
    async fn max_rises_when_current_sets_a_new_high() {
        let pool = connect_memory().await.unwrap();
        let id = add_stock(&pool, "HK", "中國銀行", Some(10.0)).await;
        add_buy(&pool, id, 100.0, 900.0).await;
        // History peaked at 10%; current is ~11.1% — max must track the live value.
        seed_history(&pool, &previous_month_end(), 500.0, 550.0).await;

        let response = build(&pool, Market::Hk).await.expect("build");

        let max = response.max.expect("max");
        assert!(near(max.percent.unwrap(), 100.0 / 900.0));
        assert!(near(max.amount, 100.0));
    }

    #[tokio::test]
    async fn seeded_max_marks_floor_the_reported_max() {
        let pool = connect_memory().await.unwrap();
        let id = add_stock(&pool, "HK", "中國銀行", Some(10.0)).await;
        add_buy(&pool, id, 100.0, 900.0).await;
        // Marks above anything history or the current values could produce.
        sqlx::query(
            "INSERT INTO app_meta (key, value) VALUES \
             ('market.HK.seed_max_percent', '0.5'), \
             ('market.HK.seed_max_amount', '500000')",
        )
        .execute(&pool)
        .await
        .expect("seed marks");

        let response = build(&pool, Market::Hk).await.expect("build");

        let max = response.max.expect("max");
        assert!(near(max.percent.unwrap(), 0.5));
        assert!(near(max.amount, 500000.0));
    }

    #[tokio::test]
    async fn unpriced_market_records_nothing() {
        let pool = connect_memory().await.unwrap();
        let id = add_stock(&pool, "HK", "中國銀行", None).await;
        add_buy(&pool, id, 100.0, 900.0).await;

        let response = build(&pool, Market::Hk).await.expect("build");

        assert!(history_rows(&pool).await.is_empty());
        assert!(response.last_month.is_none());
        assert!(response.max.is_none());
    }
}
