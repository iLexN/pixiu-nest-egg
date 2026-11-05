use sqlx::SqlitePool;

use crate::calc::ValidatedTrade;
use crate::models::{Currency, Market, TradeType};
use crate::service::ServiceError;
use crate::store::{StoreError, cash_accounts, now_timestamp, trades as store};

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

/// US trades settle inside IBKR; HK trades settle at the HS bank and keep
/// the 活期 suggestion model. The signed amount a trade applies to the USD
/// broker account: BUY subtracts the total, SELL adds it. Zero-share rows
/// are 加權平均買入單價 corrections, not cash events.
fn settlement(trade: &ValidatedTrade, market: Market) -> Option<f64> {
    if market != Market::Us || trade.shares <= 0.0 {
        return None;
    }
    Some(match trade.trade_type {
        TradeType::Buy => -trade.total,
        TradeType::Sell => trade.total,
    })
}

async fn usd_broker_id<'e, E>(exec: E) -> Result<i64, ServiceError>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    cash_accounts::broker(exec, Currency::Usd)
        .await?
        .map(|account| account.id)
        .ok_or_else(|| ServiceError::Conflict("no IBKR USD account".to_string()))
}

/// Create a trade and, in the same transaction, apply its settlement to the
/// USD broker account when the rule applies — recording the applied amount
/// on the row so a later edit or delete reverses exactly it.
pub async fn create(
    pool: &SqlitePool,
    stock_id: i64,
    market: Market,
    trade: &ValidatedTrade,
    note: Option<&str>,
) -> Result<i64, ServiceError> {
    let applied = settlement(trade, market);
    let now = now_timestamp();
    let mut tx = pool.begin().await.map_err(StoreError::from)?;
    let id = store::insert_trade(&mut *tx, stock_id, trade, note, applied).await?;
    if let Some(amount) = applied {
        let broker = usd_broker_id(&mut *tx).await?;
        cash_accounts::add_amount(&mut tx, broker, amount, &now).await?;
    }
    tx.commit().await?;
    Ok(id)
}

/// Update a trade, reversing its recorded settlement and applying the new
/// one in the same transaction. A trade with no recorded settlement moves
/// no cash — NULL stays NULL for every row written before settlement
/// existed. The caller guarantees the new stock is in the trade's market.
pub async fn update(
    pool: &SqlitePool,
    id: i64,
    stock_id: i64,
    market: Market,
    validated: &ValidatedTrade,
    note: Option<String>,
) -> Result<(), ServiceError> {
    let now = now_timestamp();
    let mut tx = pool.begin().await.map_err(StoreError::from)?;
    let recorded = store::settled_amount(&mut *tx, id).await?;
    let applied = recorded.and_then(|_| settlement(validated, market));
    if let Some(amount) = recorded {
        let broker = usd_broker_id(&mut *tx).await?;
        cash_accounts::add_amount(&mut tx, broker, -amount, &now).await?;
    }
    store::update_trade(&mut *tx, id, stock_id, validated, note, applied).await?;
    if let Some(amount) = applied {
        let broker = usd_broker_id(&mut *tx).await?;
        cash_accounts::add_amount(&mut tx, broker, amount, &now).await?;
    }
    tx.commit().await?;
    Ok(())
}

/// Delete a trade, reversing its recorded settlement in the same
/// transaction. `Ok(false)` when no trade `id` exists.
pub async fn remove(pool: &SqlitePool, id: i64) -> Result<bool, ServiceError> {
    let now = now_timestamp();
    let mut tx = pool.begin().await.map_err(StoreError::from)?;
    let recorded = store::settled_amount(&mut *tx, id).await?;
    if store::delete_trade(&mut *tx, id).await? == 0 {
        return Ok(false);
    }
    if let Some(amount) = recorded {
        let broker = usd_broker_id(&mut *tx).await?;
        cash_accounts::add_amount(&mut tx, broker, -amount, &now).await?;
    }
    tx.commit().await?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connect_memory;
    use crate::models::InputMode;

    async fn seed_stock(pool: &SqlitePool, market: Market, code: &str) -> i64 {
        sqlx::query_scalar("INSERT INTO stocks (market, code) VALUES (?, ?) RETURNING id")
            .bind(market.as_str())
            .bind(code)
            .fetch_one(pool)
            .await
            .unwrap()
    }

    async fn set_cash(pool: &SqlitePool, currency: Currency, amount: f64) {
        let account = cash_accounts::broker(pool, currency)
            .await
            .unwrap()
            .expect("broker account");
        let mut conn = pool.acquire().await.unwrap();
        cash_accounts::set_amount(&mut conn, account.id, amount, "2026-01-01T00:00:00")
            .await
            .unwrap();
    }

    async fn amount(pool: &SqlitePool, currency: Currency) -> f64 {
        cash_accounts::broker(pool, currency)
            .await
            .unwrap()
            .expect("broker account")
            .amount
    }

    fn validated(
        trade_type: TradeType,
        shares: f64,
        unit_price: f64,
        total: f64,
    ) -> ValidatedTrade {
        ValidatedTrade {
            trade_type,
            trade_date: "2026-11-05".to_string(),
            shares,
            unit_price,
            fee: 0.0,
            total,
            input_mode: InputMode::UsFee,
        }
    }

    /// A trade written before settlement existed: `settled_amount` NULL.
    async fn seed_legacy_trade(pool: &SqlitePool, stock_id: i64) -> i64 {
        sqlx::query_scalar(
            "INSERT INTO trades (stock_id, trade_type, trade_date, shares, unit_price, fee, \
             total, input_mode, created_at, updated_at) \
             VALUES (?, 'BUY', '2020-01-01', 1, 100, 0, 1000, 'US_FEE', \
             '2020-01-01T00:00:00', '2020-01-01T00:00:00') RETURNING id",
        )
        .bind(stock_id)
        .fetch_one(pool)
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn create_us_buy_debits_usd_cash() {
        let pool = connect_memory().await.expect("memory db");
        let voo = seed_stock(&pool, Market::Us, "VOO").await;
        set_cash(&pool, Currency::Usd, 5000.0).await;
        let trade = validated(TradeType::Buy, 3.0, 696.04, 2089.12);

        create(&pool, voo, Market::Us, &trade, None).await.unwrap();

        assert!((amount(&pool, Currency::Usd).await - 2910.88).abs() < 1e-6);
        assert_eq!(
            store::settled_amount(&pool, 1).await.unwrap(),
            Some(-2089.12)
        );
    }

    #[tokio::test]
    async fn create_us_sell_credits_usd_cash() {
        let pool = connect_memory().await.expect("memory db");
        let voo = seed_stock(&pool, Market::Us, "VOO").await;
        set_cash(&pool, Currency::Usd, 500.0).await;

        create(
            &pool,
            voo,
            Market::Us,
            &validated(TradeType::Sell, 1.0, 700.0, 1200.0),
            None,
        )
        .await
        .unwrap();

        assert_eq!(amount(&pool, Currency::Usd).await, 1700.0);
    }

    #[tokio::test]
    async fn create_allows_negative_usd_cash() {
        let pool = connect_memory().await.expect("memory db");
        let voo = seed_stock(&pool, Market::Us, "VOO").await;
        set_cash(&pool, Currency::Usd, 1200.0).await;

        create(
            &pool,
            voo,
            Market::Us,
            &validated(TradeType::Buy, 10.0, 500.0, 5000.0),
            None,
        )
        .await
        .unwrap();

        assert_eq!(amount(&pool, Currency::Usd).await, -3800.0);
    }

    #[tokio::test]
    async fn hk_trade_and_zero_share_move_no_cash() {
        let pool = connect_memory().await.expect("memory db");
        let hk = seed_stock(&pool, Market::Hk, "0005").await;
        let voo = seed_stock(&pool, Market::Us, "VOO").await;
        set_cash(&pool, Currency::Usd, 100.0).await;
        set_cash(&pool, Currency::Hkd, 765.42).await;

        create(
            &pool,
            hk,
            Market::Hk,
            &validated(TradeType::Buy, 30000.0, 3.2, 96523.15),
            None,
        )
        .await
        .unwrap();
        // A zero-share US row is a price correction, not a cash event.
        create(
            &pool,
            voo,
            Market::Us,
            &validated(TradeType::Buy, 0.0, 0.0, 50.0),
            Some("adj"),
        )
        .await
        .unwrap();

        assert_eq!(amount(&pool, Currency::Usd).await, 100.0);
        assert_eq!(amount(&pool, Currency::Hkd).await, 765.42);
        assert_eq!(store::settled_amount(&pool, 2).await.unwrap(), None);
    }

    #[tokio::test]
    async fn update_reverses_recorded_then_applies_new() {
        let pool = connect_memory().await.expect("memory db");
        let voo = seed_stock(&pool, Market::Us, "VOO").await;
        set_cash(&pool, Currency::Usd, 5000.0).await;
        let id = create(
            &pool,
            voo,
            Market::Us,
            &validated(TradeType::Buy, 3.0, 696.04, 2000.0),
            None,
        )
        .await
        .unwrap();

        update(
            &pool,
            id,
            voo,
            Market::Us,
            &validated(TradeType::Buy, 3.0, 696.04, 2100.0),
            None,
        )
        .await
        .unwrap();

        assert_eq!(amount(&pool, Currency::Usd).await, 2900.0);
        assert_eq!(
            store::settled_amount(&pool, id).await.unwrap(),
            Some(-2100.0)
        );
    }

    #[tokio::test]
    async fn update_to_zero_shares_unwinds_settlement() {
        let pool = connect_memory().await.expect("memory db");
        let voo = seed_stock(&pool, Market::Us, "VOO").await;
        set_cash(&pool, Currency::Usd, 5000.0).await;
        let id = create(
            &pool,
            voo,
            Market::Us,
            &validated(TradeType::Buy, 3.0, 696.04, 2000.0),
            None,
        )
        .await
        .unwrap();

        update(
            &pool,
            id,
            voo,
            Market::Us,
            &validated(TradeType::Buy, 0.0, 0.0, 2000.0),
            Some("price fix".to_string()),
        )
        .await
        .unwrap();

        assert_eq!(amount(&pool, Currency::Usd).await, 5000.0);
        assert_eq!(store::settled_amount(&pool, id).await.unwrap(), None);
    }

    #[tokio::test]
    async fn unsettled_trade_stays_inert_on_update_and_delete() {
        let pool = connect_memory().await.expect("memory db");
        let voo = seed_stock(&pool, Market::Us, "VOO").await;
        set_cash(&pool, Currency::Usd, 5000.0).await;
        let id = seed_legacy_trade(&pool, voo).await;

        update(
            &pool,
            id,
            voo,
            Market::Us,
            &validated(TradeType::Buy, 2.0, 500.0, 999.0),
            None,
        )
        .await
        .unwrap();
        assert_eq!(amount(&pool, Currency::Usd).await, 5000.0);
        assert_eq!(store::settled_amount(&pool, id).await.unwrap(), None);

        assert!(remove(&pool, id).await.unwrap());
        assert_eq!(amount(&pool, Currency::Usd).await, 5000.0);
        assert!(!remove(&pool, id).await.unwrap());
    }

    #[tokio::test]
    async fn delete_reverses_the_recorded_amount() {
        let pool = connect_memory().await.expect("memory db");
        let voo = seed_stock(&pool, Market::Us, "VOO").await;
        set_cash(&pool, Currency::Usd, 5000.0).await;
        let id = create(
            &pool,
            voo,
            Market::Us,
            &validated(TradeType::Buy, 3.0, 696.04, 2089.12),
            None,
        )
        .await
        .unwrap();

        assert!(remove(&pool, id).await.unwrap());

        assert!((amount(&pool, Currency::Usd).await - 5000.0).abs() < 1e-6);
    }
}
