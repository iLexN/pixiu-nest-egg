use sqlx::Row;
use sqlx::SqlitePool;
use sqlx::sqlite::SqliteRow;

use crate::calc::{SuggestionTrade, TradeFacts, ValidatedTrade, unit_price_incl_fee};
use crate::models::{InputMode, Market, Trade, TradeType};
use crate::store::{StoreError, now_timestamp};

pub const TRADE_SELECT: &str = "SELECT t.id, t.stock_id, s.market, s.code, t.trade_type, \
     t.trade_date, t.shares, t.unit_price, t.fee, t.total, t.input_mode, t.note \
     FROM trades t JOIN stocks s ON s.id = t.stock_id";

pub fn row_to_trade(row: &SqliteRow) -> Result<Trade, StoreError> {
    let market: String = row.try_get("market")?;
    let trade_type: String = row.try_get("trade_type")?;
    let input_mode: String = row.try_get("input_mode")?;
    let shares: f64 = row.try_get("shares")?;
    let total: f64 = row.try_get("total")?;
    Ok(Trade {
        id: row.try_get("id")?,
        stock_id: row.try_get("stock_id")?,
        market: Market::parse(&market)
            .ok_or_else(|| StoreError::Corrupt(format!("stored market {market} is not valid")))?,
        code: row.try_get("code")?,
        trade_type: TradeType::parse(&trade_type).ok_or_else(|| {
            StoreError::Corrupt(format!("stored trade type {trade_type} is not valid"))
        })?,
        trade_date: row.try_get("trade_date")?,
        shares,
        unit_price: row.try_get("unit_price")?,
        fee: row.try_get("fee")?,
        total,
        input_mode: InputMode::parse(&input_mode).ok_or_else(|| {
            StoreError::Corrupt(format!("stored input mode {input_mode} is not valid"))
        })?,
        note: row.try_get("note")?,
        unit_price_incl_fee: unit_price_incl_fee(total, shares),
    })
}

pub async fn list(
    pool: &SqlitePool,
    market: Option<Market>,
    stock_id: Option<i64>,
    code: Option<&str>,
    from: Option<&str>,
    to: Option<&str>,
    descending: bool,
) -> Result<Vec<Trade>, StoreError> {
    let mut sql = TRADE_SELECT.to_string();
    let mut conditions: Vec<&str> = Vec::new();
    if market.is_some() {
        conditions.push("s.market = ?");
    }
    if stock_id.is_some() {
        conditions.push("t.stock_id = ?");
    }
    if code.is_some() {
        conditions.push("s.code = ?");
    }
    if from.is_some() {
        conditions.push("t.trade_date >= ?");
    }
    if to.is_some() {
        conditions.push("t.trade_date <= ?");
    }
    if !conditions.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&conditions.join(" AND "));
    }
    sql.push_str(if descending {
        " ORDER BY t.trade_date DESC, t.id DESC"
    } else {
        " ORDER BY t.trade_date ASC, t.id ASC"
    });

    let mut statement = sqlx::query(sqlx::AssertSqlSafe(sql));
    if let Some(market) = market {
        statement = statement.bind(market.as_str());
    }
    if let Some(stock_id) = stock_id {
        statement = statement.bind(stock_id);
    }
    if let Some(code) = code {
        statement = statement.bind(code.to_string());
    }
    if let Some(from) = from {
        statement = statement.bind(from.to_string());
    }
    if let Some(to) = to {
        statement = statement.bind(to.to_string());
    }

    statement
        .fetch_all(pool)
        .await?
        .iter()
        .map(row_to_trade)
        .collect::<Result<Vec<_>, _>>()
}

pub async fn insert_trade(
    pool: &SqlitePool,
    stock_id: i64,
    trade: &ValidatedTrade,
    note: Option<&str>,
) -> Result<i64, StoreError> {
    let now = now_timestamp();
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO trades (stock_id, trade_type, trade_date, shares, unit_price, fee, total, \
         input_mode, note, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(stock_id)
    .bind(trade.trade_type.as_str())
    .bind(&trade.trade_date)
    .bind(trade.shares)
    .bind(trade.unit_price)
    .bind(trade.fee)
    .bind(trade.total)
    .bind(trade.input_mode.as_str())
    .bind(note)
    .bind(&now)
    .bind(&now)
    .fetch_one(pool)
    .await?;
    Ok(id)
}

pub async fn update_trade(
    pool: &SqlitePool,
    id: i64,
    stock_id: i64,
    validated: &ValidatedTrade,
    note: Option<String>,
) -> Result<(), StoreError> {
    sqlx::query(
        "UPDATE trades SET stock_id = ?, trade_type = ?, trade_date = ?, shares = ?, \
         unit_price = ?, fee = ?, total = ?, input_mode = ?, note = ?, updated_at = ? WHERE id = ?",
    )
    .bind(stock_id)
    .bind(validated.trade_type.as_str())
    .bind(&validated.trade_date)
    .bind(validated.shares)
    .bind(validated.unit_price)
    .bind(validated.fee)
    .bind(validated.total)
    .bind(validated.input_mode.as_str())
    .bind(note)
    .bind(now_timestamp())
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Rows deleted, so the handler can tell "not found" apart from success.
pub async fn delete_trade(pool: &SqlitePool, id: i64) -> Result<u64, StoreError> {
    Ok(sqlx::query("DELETE FROM trades WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?
        .rows_affected())
}

pub async fn load_one(pool: &SqlitePool, id: i64) -> Result<Trade, StoreError> {
    let row = sqlx::query(sqlx::AssertSqlSafe(format!(
        "{TRADE_SELECT} WHERE t.id = ?"
    )))
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| StoreError::NotFound(format!("trade {id} not found")))?;
    row_to_trade(&row)
}

/// The market's trades reduced to what the summary/yearly rollups need.
pub async fn market_trade_facts(
    pool: &SqlitePool,
    market: Market,
) -> Result<Vec<(i64, TradeFacts)>, StoreError> {
    let rows = sqlx::query(
        "SELECT t.stock_id, t.trade_type, t.shares, t.total FROM trades t \
         JOIN stocks s ON s.id = t.stock_id WHERE s.market = ? \
         ORDER BY t.trade_date, t.id",
    )
    .bind(market.as_str())
    .fetch_all(pool)
    .await?;

    let mut facts = Vec::with_capacity(rows.len());
    for row in &rows {
        let trade_type: String = row.try_get("trade_type")?;
        facts.push((
            row.try_get("stock_id")?,
            TradeFacts {
                trade_type: TradeType::parse(&trade_type).ok_or_else(|| {
                    StoreError::Corrupt(format!("stored trade type {trade_type} is not valid"))
                })?,
                shares: row.try_get("shares")?,
                total: row.try_get("total")?,
            },
        ));
    }
    Ok(facts)
}

/// The market's dated trades, for the yearly rollup.
pub async fn market_dated_trade_facts(
    pool: &SqlitePool,
    market: Market,
) -> Result<Vec<(chrono::NaiveDate, TradeFacts)>, StoreError> {
    let rows = sqlx::query(
        "SELECT t.trade_date, t.trade_type, t.shares, t.total FROM trades t \
         JOIN stocks s ON s.id = t.stock_id WHERE s.market = ? \
         ORDER BY t.trade_date, t.id",
    )
    .bind(market.as_str())
    .fetch_all(pool)
    .await?;

    let mut facts = Vec::with_capacity(rows.len());
    for row in &rows {
        let trade_date: String = row.try_get("trade_date")?;
        let parsed = chrono::NaiveDate::parse_from_str(&trade_date, "%Y-%m-%d").map_err(|_| {
            StoreError::Corrupt(format!("stored trade_date {trade_date} is not valid"))
        })?;
        let trade_type: String = row.try_get("trade_type")?;
        facts.push((
            parsed,
            TradeFacts {
                trade_type: TradeType::parse(&trade_type).ok_or_else(|| {
                    StoreError::Corrupt(format!("stored trade type {trade_type} is not valid"))
                })?,
                shares: row.try_get("shares")?,
                total: row.try_get("total")?,
            },
        ));
    }
    Ok(facts)
}

/// HK trades in `[first, next)` as suggestion sources (US trades settle inside
/// IBKR and stay manual items).
pub async fn hk_suggestion_trades(
    pool: &SqlitePool,
    first: &str,
    next: &str,
) -> Result<Vec<SuggestionTrade>, StoreError> {
    let rows = sqlx::query(
        "SELECT t.id, t.trade_date, t.trade_type, t.total, s.code FROM trades t \
         JOIN stocks s ON s.id = t.stock_id \
         WHERE s.market = 'HK' AND t.trade_date >= ? AND t.trade_date < ?",
    )
    .bind(first)
    .bind(next)
    .fetch_all(pool)
    .await?;
    let mut trades = Vec::with_capacity(rows.len());
    for row in &rows {
        let trade_type: String = row.try_get("trade_type")?;
        let trade_date: String = row.try_get("trade_date")?;
        trades.push(SuggestionTrade {
            id: row.try_get("id")?,
            date: chrono::NaiveDate::parse_from_str(&trade_date, "%Y-%m-%d").map_err(|_| {
                StoreError::Corrupt(format!("stored date {trade_date} is not valid"))
            })?,
            kind: TradeType::parse(&trade_type).ok_or_else(|| {
                StoreError::Corrupt(format!("stored trade type {trade_type} is not valid"))
            })?,
            total: row.try_get("total")?,
            code: row.try_get("code")?,
        });
    }
    Ok(trades)
}
