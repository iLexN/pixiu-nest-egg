use sqlx::Row;
use sqlx::SqlitePool;
use sqlx::sqlite::SqliteRow;

use crate::calc::{
    DividendFacts, SuggestionReceipt, TradeFacts, ValidatedDividend, dividend_amount,
    dividend_variance, yield_on_cost, yield_on_price,
};
use crate::models::{Dividend, DividendStatus, Market, TradeType};
use crate::store::{StoreError, now_timestamp};

pub const DIVIDEND_SELECT: &str = "SELECT d.id, d.stock_id, s.market, s.code, d.pay_date, \
     d.per_share, d.shares_held, d.buy_cost, d.estimated_amount, d.received_amount, \
     d.received_price, d.withholding_tax, d.note \
     FROM dividends d JOIN stocks s ON s.id = d.stock_id";

pub fn row_to_dividend(row: &SqliteRow) -> Result<Dividend, StoreError> {
    let market: String = row.try_get("market")?;
    let shares_held: Option<f64> = row.try_get("shares_held")?;
    let buy_cost: Option<f64> = row.try_get("buy_cost")?;
    let estimated_amount: Option<f64> = row.try_get("estimated_amount")?;
    let received_amount: Option<f64> = row.try_get("received_amount")?;
    let received_price: Option<f64> = row.try_get("received_price")?;
    let withholding_tax: Option<f64> = row.try_get("withholding_tax")?;
    let amount = dividend_amount(received_amount, estimated_amount);
    let gross_amount = match (received_amount, withholding_tax) {
        (Some(received), Some(tax)) => Some(received + tax),
        _ => None,
    };
    Ok(Dividend {
        id: row.try_get("id")?,
        stock_id: row.try_get("stock_id")?,
        market: Market::parse(&market)
            .ok_or_else(|| StoreError::Corrupt(format!("stored market {market} is not valid")))?,
        code: row.try_get("code")?,
        pay_date: row.try_get("pay_date")?,
        per_share: row.try_get("per_share")?,
        shares_held,
        buy_cost,
        estimated_amount,
        received_amount,
        received_price,
        withholding_tax,
        gross_amount,
        note: row.try_get("note")?,
        status: if received_amount.is_some() {
            DividendStatus::Received
        } else {
            DividendStatus::Pending
        },
        amount,
        yield_on_cost: yield_on_cost(amount, buy_cost),
        yield_on_price: yield_on_price(amount, received_price, shares_held),
        variance: dividend_variance(received_amount, estimated_amount),
    })
}

/// `received`: Some(false) lists only pending rows, Some(true) only received.
pub async fn list(
    pool: &SqlitePool,
    market: Option<Market>,
    stock_id: Option<i64>,
    received: Option<bool>,
    year: Option<i32>,
    descending: bool,
) -> Result<Vec<Dividend>, StoreError> {
    let mut sql = DIVIDEND_SELECT.to_string();
    let mut conditions: Vec<&str> = Vec::new();
    if market.is_some() {
        conditions.push("s.market = ?");
    }
    if stock_id.is_some() {
        conditions.push("d.stock_id = ?");
    }
    if let Some(received) = received {
        if received {
            conditions.push("d.received_amount IS NOT NULL");
        } else {
            conditions.push("d.received_amount IS NULL");
        }
    }
    if year.is_some() {
        conditions.push("d.pay_date >= ? AND d.pay_date <= ?");
    }
    if !conditions.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&conditions.join(" AND "));
    }
    sql.push_str(if descending {
        " ORDER BY d.pay_date DESC, d.id DESC"
    } else {
        " ORDER BY d.pay_date ASC, d.id ASC"
    });

    let mut statement = sqlx::query(sqlx::AssertSqlSafe(sql));
    if let Some(market) = market {
        statement = statement.bind(market.as_str());
    }
    if let Some(stock_id) = stock_id {
        statement = statement.bind(stock_id);
    }
    if let Some(year) = year {
        statement = statement
            .bind(format!("{year:04}-01-01"))
            .bind(format!("{year:04}-12-31"));
    }

    statement
        .fetch_all(pool)
        .await?
        .iter()
        .map(row_to_dividend)
        .collect::<Result<Vec<_>, _>>()
}

/// The summary list: every dividend, optionally restricted to one market.
pub async fn list_for_market(
    pool: &SqlitePool,
    market: Option<Market>,
) -> Result<Vec<Dividend>, StoreError> {
    let mut sql = DIVIDEND_SELECT.to_string();
    if market.is_some() {
        sql.push_str(" WHERE s.market = ?");
    }
    sql.push_str(" ORDER BY d.pay_date ASC, d.id ASC");
    let mut statement = sqlx::query(sqlx::AssertSqlSafe(sql));
    if let Some(market) = market {
        statement = statement.bind(market.as_str());
    }
    statement
        .fetch_all(pool)
        .await?
        .iter()
        .map(row_to_dividend)
        .collect::<Result<Vec<_>, _>>()
}

/// The stock's trades reduced to what the snapshot derivation needs.
pub async fn dated_trade_facts(
    pool: &SqlitePool,
    stock_id: i64,
) -> Result<Vec<(chrono::NaiveDate, TradeFacts)>, StoreError> {
    let rows = sqlx::query(
        "SELECT trade_type, trade_date, shares, total FROM trades \
         WHERE stock_id = ? ORDER BY trade_date, id",
    )
    .bind(stock_id)
    .fetch_all(pool)
    .await?;
    let mut facts = Vec::with_capacity(rows.len());
    for row in &rows {
        let trade_type: String = row.try_get("trade_type")?;
        let trade_date: String = row.try_get("trade_date")?;
        let parsed = chrono::NaiveDate::parse_from_str(&trade_date, "%Y-%m-%d").map_err(|_| {
            StoreError::Corrupt(format!("stored trade_date {trade_date} is not valid"))
        })?;
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

pub async fn insert_dividend(
    pool: &SqlitePool,
    stock_id: i64,
    dividend: &ValidatedDividend,
    note: Option<&str>,
) -> Result<i64, StoreError> {
    let now = now_timestamp();
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO dividends (stock_id, pay_date, per_share, shares_held, buy_cost, \
         estimated_amount, received_amount, received_price, note, \
         created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(stock_id)
    .bind(&dividend.pay_date)
    .bind(dividend.per_share)
    .bind(dividend.shares_held)
    .bind(dividend.buy_cost)
    .bind(dividend.estimated_amount)
    .bind(dividend.received_amount)
    .bind(dividend.received_price)
    .bind(note)
    .bind(&now)
    .bind(&now)
    .fetch_one(pool)
    .await?;
    Ok(id)
}

pub async fn update_dividend(
    conn: &mut sqlx::SqliteConnection,
    id: i64,
    stock_id: i64,
    validated: &ValidatedDividend,
    note: Option<String>,
    now: &str,
) -> Result<(), StoreError> {
    sqlx::query(
        "UPDATE dividends SET stock_id = ?, pay_date = ?, per_share = ?, shares_held = ?, \
         buy_cost = ?, estimated_amount = ?, received_amount = ?, \
         received_price = ?, withholding_tax = ?, note = ?, updated_at = ? WHERE id = ?",
    )
    .bind(stock_id)
    .bind(&validated.pay_date)
    .bind(validated.per_share)
    .bind(validated.shares_held)
    .bind(validated.buy_cost)
    .bind(validated.estimated_amount)
    .bind(validated.received_amount)
    .bind(validated.received_price)
    .bind(validated.withholding_tax)
    .bind(note)
    .bind(now)
    .bind(id)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// The stored credited_amount of a dividend, inside the caller's transaction.
pub async fn credited_amount(
    conn: &mut sqlx::SqliteConnection,
    id: i64,
) -> Result<Option<f64>, StoreError> {
    Ok(
        sqlx::query_scalar("SELECT credited_amount FROM dividends WHERE id = ?")
            .bind(id)
            .fetch_one(&mut *conn)
            .await?,
    )
}

pub async fn set_credited_amount(
    conn: &mut sqlx::SqliteConnection,
    id: i64,
    amount: Option<f64>,
) -> Result<(), StoreError> {
    match amount {
        Some(_) => {
            sqlx::query("UPDATE dividends SET credited_amount = ? WHERE id = ?")
                .bind(amount)
                .bind(id)
                .execute(&mut *conn)
                .await?;
        }
        None => {
            sqlx::query("UPDATE dividends SET credited_amount = NULL WHERE id = ?")
                .bind(id)
                .execute(&mut *conn)
                .await?;
        }
    }
    Ok(())
}

/// Rows deleted, so the handler can tell "not found" apart from success.
pub async fn delete_dividend(pool: &SqlitePool, id: i64) -> Result<u64, StoreError> {
    Ok(sqlx::query("DELETE FROM dividends WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?
        .rows_affected())
}

pub async fn load_one(pool: &SqlitePool, id: i64) -> Result<Dividend, StoreError> {
    let row = sqlx::query(sqlx::AssertSqlSafe(format!(
        "{DIVIDEND_SELECT} WHERE d.id = ?"
    )))
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| StoreError::NotFound(format!("dividend {id} not found")))?;
    row_to_dividend(&row)
}

/// Σ received amount per stock over one market (累計派息 counts only received
/// amounts; pending estimates are excluded).
pub async fn received_totals(
    pool: &SqlitePool,
    market: Market,
) -> Result<std::collections::HashMap<i64, f64>, StoreError> {
    let rows = sqlx::query(
        "SELECT d.stock_id, SUM(d.received_amount) AS received FROM dividends d \
         JOIN stocks s ON s.id = d.stock_id WHERE s.market = ? \
         AND d.received_amount IS NOT NULL GROUP BY d.stock_id",
    )
    .bind(market.as_str())
    .fetch_all(pool)
    .await?;
    let mut dividends: std::collections::HashMap<i64, f64> =
        std::collections::HashMap::with_capacity(rows.len());
    for row in &rows {
        dividends.insert(row.try_get("stock_id")?, row.try_get("received")?);
    }
    Ok(dividends)
}

/// Only received amounts count toward a year's 派息; pending estimates are
/// filtered out here so the rollup sees nothing but real payouts.
pub async fn market_dividend_facts(
    pool: &SqlitePool,
    market: Market,
) -> Result<Vec<DividendFacts<'static>>, StoreError> {
    let rows = sqlx::query(
        "SELECT d.pay_date, d.received_amount FROM dividends d \
         JOIN stocks s ON s.id = d.stock_id WHERE s.market = ? \
         AND d.received_amount IS NOT NULL",
    )
    .bind(market.as_str())
    .fetch_all(pool)
    .await?;

    let mut facts = Vec::with_capacity(rows.len());
    for row in &rows {
        let pay_date: String = row.try_get("pay_date")?;
        let parsed = chrono::NaiveDate::parse_from_str(&pay_date, "%Y-%m-%d")
            .map_err(|_| StoreError::Corrupt(format!("stored pay_date {pay_date} is not valid")))?;
        facts.push(DividendFacts {
            code: "",
            pay_date: parsed,
            received_amount: row.try_get("received_amount")?,
        });
    }
    Ok(facts)
}

/// Received HK dividends paid in `range`, or all of them when `None`.
pub async fn suggestion_receipts(
    pool: &SqlitePool,
    range: Option<(String, String)>,
) -> Result<Vec<SuggestionReceipt>, StoreError> {
    let mut sql = "SELECT d.id, d.pay_date, d.received_amount, d.estimated_amount, s.code \
         FROM dividends d JOIN stocks s ON s.id = d.stock_id WHERE s.market = 'HK'"
        .to_string();
    if range.is_some() {
        sql.push_str(" AND d.pay_date >= ? AND d.pay_date < ?");
    }
    let mut query = sqlx::query(sqlx::AssertSqlSafe(sql));
    if let Some((first, next)) = &range {
        query = query.bind(first).bind(next);
    }
    let rows = query.fetch_all(pool).await?;
    rows.iter()
        .map(|row| {
            let received_amount: Option<f64> = row.try_get("received_amount")?;
            let estimated_amount: Option<f64> = row.try_get("estimated_amount")?;
            let pay_date: String = row.try_get("pay_date")?;
            Ok(SuggestionReceipt {
                id: row.try_get("id")?,
                pay_date: chrono::NaiveDate::parse_from_str(&pay_date, "%Y-%m-%d").map_err(
                    |_| StoreError::Corrupt(format!("stored date {pay_date} is not valid")),
                )?,
                amount: received_amount.or(estimated_amount),
                label: row.try_get("code")?,
                received: received_amount.is_some(),
            })
        })
        .collect()
}
