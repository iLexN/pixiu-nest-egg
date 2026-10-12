use sqlx::SqlitePool;

use crate::models::Market;
use crate::store::StoreError;

/// Resolve the stock a trade or dividend refers to — by `stock_id`, or by
/// market + 股票代碼 when the caller only has the code.
pub async fn resolve_stock(
    pool: &SqlitePool,
    stock_id: Option<i64>,
    market: Option<Market>,
    code: Option<&str>,
) -> Result<(i64, Market), StoreError> {
    if let Some(stock_id) = stock_id {
        let stock = crate::store::stocks::load_one(pool, stock_id)
            .await
            .map_err(|err| match err {
                StoreError::NotFound(_) => {
                    StoreError::NotFound(format!("stock {stock_id} is not registered"))
                }
                other => other,
            })?;
        return Ok((stock.id, stock.market));
    }

    match (market, code.map(str::trim).filter(|c| !c.is_empty())) {
        (Some(market), Some(code)) => {
            let stock = crate::store::stocks::find_by_code(pool, market, code)
                .await?
                .ok_or_else(|| {
                    StoreError::NotFound(format!(
                        "{code} is not registered in market {}",
                        market.as_str()
                    ))
                })?;
            Ok((stock.id, stock.market))
        }
        _ => Err(StoreError::NotFound(
            "either stock_id, or market and 股票代碼, is required".to_string(),
        )),
    }
}
