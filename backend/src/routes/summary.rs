use axum::extract::{Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};

use super::{parse_market, AppState};
use crate::calc::{
    market_totals, sector_rollup, summarize, MarketTotals, RollupInput, SectorRollup, StockSummary,
    TradeFacts,
};
use crate::error::ApiError;
use crate::models::{Market, Stock, TradeType};

/// Shown next to 加權平均買入單價 so the definition travels with the number.
pub const AVERAGE_PRICE_DEFINITION: &str =
    "加權平均買入單價 = 總買入成本 ÷ Σ BUY 股數 (shares bought, not shares held)";

#[derive(Debug, Deserialize)]
pub struct SummaryQuery {
    pub market: String,
}

#[derive(Debug, Serialize)]
pub struct StockRow {
    #[serde(flatten)]
    pub stock: Stock,
    #[serde(flatten)]
    pub summary: StockSummary,
    pub trade_count: usize,
}

#[derive(Debug, Serialize)]
pub struct SummaryResponse {
    pub market: Market,
    pub average_price_definition: &'static str,
    pub stocks: Vec<StockRow>,
    pub sectors: Vec<SectorRollup>,
    pub totals: MarketTotals,
}

pub async fn show(
    State(state): State<AppState>,
    Query(query): Query<SummaryQuery>,
) -> Result<Json<SummaryResponse>, ApiError> {
    let market = parse_market(&query.market)?;
    Ok(Json(build(&state.pool, market).await?))
}

/// Every figure here is derived from the stored trades on each read.
pub async fn build(pool: &SqlitePool, market: Market) -> Result<SummaryResponse, ApiError> {
    let stocks = super::stocks::load_stocks(pool, Some(market)).await?;

    let rows = sqlx::query(
        "SELECT t.stock_id, t.trade_type, t.shares, t.total FROM trades t \
         JOIN stocks s ON s.id = t.stock_id WHERE s.market = ? \
         ORDER BY t.trade_date, t.id",
    )
    .bind(market.as_str())
    .fetch_all(pool)
    .await?;

    let mut facts: Vec<(i64, TradeFacts)> = Vec::with_capacity(rows.len());
    for row in &rows {
        let trade_type: String = row.try_get("trade_type")?;
        let trade_type = TradeType::parse(&trade_type).ok_or_else(|| {
            ApiError::Conflict(format!("stored trade type {trade_type} is not valid"))
        })?;
        facts.push((
            row.try_get("stock_id")?,
            TradeFacts {
                trade_type,
                shares: row.try_get("shares")?,
                total: row.try_get("total")?,
            },
        ));
    }

    let stock_rows: Vec<StockRow> = stocks
        .into_iter()
        .map(|stock| {
            let own: Vec<TradeFacts> = facts
                .iter()
                .filter(|(stock_id, _)| *stock_id == stock.id)
                .map(|(_, fact)| *fact)
                .collect();
            let summary = summarize(&own, stock.manual_price);
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
        })
        .collect();

    let sectors = sector_rollup(&rollup_inputs);
    let totals = market_totals(&rollup_inputs);
    drop(rollup_inputs);

    Ok(SummaryResponse {
        market,
        average_price_definition: AVERAGE_PRICE_DEFINITION,
        stocks: stock_rows,
        sectors,
        totals,
    })
}
