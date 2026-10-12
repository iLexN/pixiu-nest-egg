use chrono::Datelike;
use serde::Serialize;
use sqlx::SqlitePool;

use crate::calc::{YearRow, yearly_rows};
use crate::models::Market;
use crate::store::{StoreError, today};

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct YearlyResponse {
    pub market: Market,
    /// The date the response was generated.
    pub today: String,
    pub years: Vec<YearRow>,
}

/// Every figure here is derived from stored trades, dividends and snapshots
/// on each read.
pub async fn build(pool: &SqlitePool, market: Market) -> Result<YearlyResponse, StoreError> {
    let trades = crate::store::trades::market_dated_trade_facts(pool, market).await?;
    let dividends = crate::store::dividends::market_dividend_facts(pool, market).await?;
    let snapshots = crate::store::yearly::load_snapshots(pool, market).await?;
    let live_market_value = super::summary::build(pool, market)
        .await?
        .totals
        .market_value;

    Ok(YearlyResponse {
        market,
        today: today().to_string(),
        years: yearly_rows(
            &trades,
            &dividends,
            &snapshots,
            live_market_value,
            today().year(),
        ),
    })
}
