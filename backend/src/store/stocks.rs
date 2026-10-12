use sqlx::Row;
use sqlx::SqlitePool;
use sqlx::sqlite::SqliteRow;

use crate::models::{Market, Stock};
use crate::store::StoreError;

pub const STOCK_COLUMNS: &str = "id, market, code, ticker, exchange, sector, manual_price, \
     price_updated_at, pe, eps, high52, low52, note, is_active, sort_order";

pub fn row_to_stock(row: &SqliteRow) -> Result<Stock, StoreError> {
    let market: String = row.try_get("market")?;
    Ok(Stock {
        id: row.try_get("id")?,
        market: Market::parse(&market)
            .ok_or_else(|| StoreError::Corrupt(format!("stored market {market} is not valid")))?,
        code: row.try_get("code")?,
        ticker: row.try_get("ticker")?,
        exchange: row.try_get("exchange")?,
        sector: row.try_get("sector")?,
        manual_price: row.try_get("manual_price")?,
        price_updated_at: row.try_get("price_updated_at")?,
        pe: row.try_get("pe")?,
        eps: row.try_get("eps")?,
        high52: row.try_get("high52")?,
        low52: row.try_get("low52")?,
        note: row.try_get("note")?,
        is_active: row.try_get::<i64, _>("is_active")? != 0,
        sort_order: row.try_get("sort_order")?,
    })
}

pub async fn load_stocks(
    pool: &SqlitePool,
    market: Option<Market>,
) -> Result<Vec<Stock>, StoreError> {
    let sql = format!(
        "SELECT {STOCK_COLUMNS} FROM stocks {} ORDER BY market, sort_order, code",
        if market.is_some() {
            "WHERE market = ?"
        } else {
            ""
        }
    );
    let mut statement = sqlx::query(sqlx::AssertSqlSafe(sql));
    if let Some(market) = market {
        statement = statement.bind(market.as_str());
    }
    statement
        .fetch_all(pool)
        .await?
        .iter()
        .map(row_to_stock)
        .collect()
}

pub async fn load_one(pool: &SqlitePool, id: i64) -> Result<Stock, StoreError> {
    let row = sqlx::query(sqlx::AssertSqlSafe(format!(
        "SELECT {STOCK_COLUMNS} FROM stocks WHERE id = ?"
    )))
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| StoreError::NotFound(format!("stock {id} not found")))?;
    row_to_stock(&row)
}

pub async fn find_by_code(
    pool: &SqlitePool,
    market: Market,
    code: &str,
) -> Result<Option<Stock>, StoreError> {
    let row = sqlx::query(sqlx::AssertSqlSafe(format!(
        "SELECT {STOCK_COLUMNS} FROM stocks WHERE market = ? AND code = ?"
    )))
    .bind(market.as_str())
    .bind(code)
    .fetch_optional(pool)
    .await?;
    row.as_ref().map(row_to_stock).transpose()
}

pub async fn next_sort_order(pool: &SqlitePool, market: Market) -> Result<i64, StoreError> {
    Ok(
        sqlx::query_scalar("SELECT COALESCE(MAX(sort_order), 0) + 1 FROM stocks WHERE market = ?")
            .bind(market.as_str())
            .fetch_one(pool)
            .await?,
    )
}

#[allow(clippy::too_many_arguments)]
pub async fn insert_stock(
    pool: &SqlitePool,
    market: Market,
    code: &str,
    ticker: Option<String>,
    exchange: Option<String>,
    sector: Option<String>,
    manual_price: Option<f64>,
    price_updated_at: Option<String>,
    pe: Option<f64>,
    eps: Option<f64>,
    high52: Option<f64>,
    low52: Option<f64>,
    note: Option<String>,
    sort_order: i64,
) -> Result<i64, StoreError> {
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO stocks (market, code, ticker, exchange, sector, manual_price, \
         price_updated_at, pe, eps, high52, low52, note, is_active, sort_order) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 1, ?) RETURNING id",
    )
    .bind(market.as_str())
    .bind(code)
    .bind(ticker)
    .bind(exchange)
    .bind(sector)
    .bind(manual_price)
    .bind(price_updated_at)
    .bind(pe)
    .bind(eps)
    .bind(high52)
    .bind(low52)
    .bind(note)
    .bind(sort_order)
    .fetch_one(pool)
    .await?;
    Ok(id)
}

pub async fn set_sort_order(
    conn: &mut sqlx::SqliteConnection,
    sort_order: i64,
    stock_id: i64,
    market: Market,
) -> Result<(), StoreError> {
    sqlx::query("UPDATE stocks SET sort_order = ? WHERE id = ? AND market = ?")
        .bind(sort_order)
        .bind(stock_id)
        .bind(market.as_str())
        .execute(&mut *conn)
        .await?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn update_stock(
    pool: &SqlitePool,
    id: i64,
    code: String,
    ticker: Option<String>,
    exchange: Option<String>,
    sector: Option<String>,
    manual_price: Option<f64>,
    price_updated_at: Option<String>,
    pe: Option<f64>,
    eps: Option<f64>,
    high52: Option<f64>,
    low52: Option<f64>,
    note: Option<String>,
    is_active: i64,
) -> Result<(), StoreError> {
    sqlx::query(
        "UPDATE stocks SET code = ?, ticker = ?, exchange = ?, sector = ?, manual_price = ?, \
         price_updated_at = ?, pe = ?, eps = ?, high52 = ?, low52 = ?, note = ?, is_active = ? \
         WHERE id = ?",
    )
    .bind(code)
    .bind(ticker)
    .bind(exchange)
    .bind(sector)
    .bind(manual_price)
    .bind(price_updated_at)
    .bind(pe)
    .bind(eps)
    .bind(high52)
    .bind(low52)
    .bind(note)
    .bind(is_active)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn set_price(
    conn: &mut sqlx::SqliteConnection,
    price: f64,
    now: &str,
    id: i64,
) -> Result<(), StoreError> {
    sqlx::query("UPDATE stocks SET manual_price = ?, price_updated_at = ? WHERE id = ?")
        .bind(price)
        .bind(now)
        .bind(id)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

pub async fn count_trades(pool: &SqlitePool, stock_id: i64) -> Result<i64, StoreError> {
    Ok(
        sqlx::query_scalar("SELECT COUNT(*) FROM trades WHERE stock_id = ?")
            .bind(stock_id)
            .fetch_one(pool)
            .await?,
    )
}

pub async fn count_dividends(pool: &SqlitePool, stock_id: i64) -> Result<i64, StoreError> {
    Ok(
        sqlx::query_scalar("SELECT COUNT(*) FROM dividends WHERE stock_id = ?")
            .bind(stock_id)
            .fetch_one(pool)
            .await?,
    )
}

pub async fn delete_stock(pool: &SqlitePool, id: i64) -> Result<(), StoreError> {
    sqlx::query("DELETE FROM stocks WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn manual_price(pool: &SqlitePool, id: i64) -> Result<Option<f64>, StoreError> {
    Ok(
        sqlx::query_scalar("SELECT manual_price FROM stocks WHERE id = ?")
            .bind(id)
            .fetch_one(pool)
            .await?,
    )
}
