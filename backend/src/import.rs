//! One-off import of the spreadsheet's trade history into the database.
//!
//! Idempotent: a trade already stored with the same stock, 日期, 類別, 股數 and
//! buy total is skipped, while genuinely duplicated source rows are preserved.

use std::collections::HashMap;

use anyhow::{anyhow, Context};
use sqlx::{Row, SqlitePool};

use crate::calc::{validate_trade, TradeInput};
use crate::models::Market;
use crate::xlsx::{MarketSheets, SheetStock, WorkbookData};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct MarketReport {
    pub stocks_created: usize,
    pub stocks_updated: usize,
    pub trades_imported: usize,
    pub trades_skipped: usize,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ImportReport {
    pub hk: MarketReport,
    pub us: MarketReport,
}

impl ImportReport {
    pub fn market(&self, market: Market) -> &MarketReport {
        match market {
            Market::Hk => &self.hk,
            Market::Us => &self.us,
        }
    }
}

pub async fn import(pool: &SqlitePool, data: &WorkbookData) -> anyhow::Result<ImportReport> {
    Ok(ImportReport {
        hk: import_market(pool, &data.hk).await?,
        us: import_market(pool, &data.us).await?,
    })
}

async fn import_market(pool: &SqlitePool, sheets: &MarketSheets) -> anyhow::Result<MarketReport> {
    let market = sheets.market;
    let mut report = MarketReport::default();

    for stock in &sheets.stocks {
        match upsert_stock(pool, market, stock).await? {
            Upsert::Created => report.stocks_created += 1,
            Upsert::Updated => report.stocks_updated += 1,
            Upsert::Unchanged => {}
        }
    }

    // Codes that only appear in the trade sheet still need a stock row, placed
    // after the stocks listed on the summary sheet.
    let mut next_sort_order = next_stock_sort_order(pool, market).await?;
    for trade in &sheets.trades {
        if stock_id(pool, market, &trade.code).await?.is_none() {
            upsert_stock(
                pool,
                market,
                &SheetStock {
                    code: trade.code.clone(),
                    ticker: None,
                    exchange: None,
                    sector: None,
                    sort_order: next_sort_order,
                },
            )
            .await?;
            next_sort_order += 1;
            report.stocks_created += 1;
        }
    }

    // Snapshot the trades already stored, so inserts made by this run cannot
    // make a genuinely duplicated source row look like an existing one.
    let mut existing = existing_trade_keys(pool, market).await?;

    for trade in &sheets.trades {
        let stock_id = stock_id(pool, market, &trade.code)
            .await?
            .ok_or_else(|| anyhow!("stock {} was not created", trade.code))?;

        let validated = validate_trade(TradeInput {
            trade_type: &trade.trade_type,
            trade_date: &trade.trade_date,
            shares: trade.shares,
            unit_price: trade.unit_price,
            total: trade.total,
            fee: trade.fee,
            input_mode: trade.input_mode,
            note: trade.note.as_deref(),
        })
        .map_err(|errors| {
            anyhow!(
                "row {} of the {} trade sheet is not valid: {}",
                trade.source_row,
                market.as_str(),
                errors
                    .iter()
                    .map(|e| format!("{}: {}", e.field, e.message))
                    .collect::<Vec<_>>()
                    .join("; ")
            )
        })?;

        let key = TradeKey::new(
            stock_id,
            &validated.trade_date,
            validated.trade_type.as_str(),
            validated.shares,
            validated.total,
        );
        if let Some(remaining) = existing.get_mut(&key) {
            if *remaining > 0 {
                *remaining -= 1;
                report.trades_skipped += 1;
                continue;
            }
        }

        let now = crate::routes::now_timestamp();
        sqlx::query(
            "INSERT INTO trades (stock_id, trade_type, trade_date, shares, unit_price, fee, \
             total, input_mode, note, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(stock_id)
        .bind(validated.trade_type.as_str())
        .bind(&validated.trade_date)
        .bind(validated.shares)
        .bind(validated.unit_price)
        .bind(validated.fee)
        .bind(validated.total)
        .bind(validated.input_mode.as_str())
        .bind(trade.note.as_deref())
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await
        .with_context(|| format!("inserting row {} of {}", trade.source_row, market.as_str()))?;
        report.trades_imported += 1;
    }

    Ok(report)
}

enum Upsert {
    Created,
    Updated,
    Unchanged,
}

async fn upsert_stock(
    pool: &SqlitePool,
    market: Market,
    stock: &SheetStock,
) -> anyhow::Result<Upsert> {
    let existing = sqlx::query(
        "SELECT id, ticker, exchange, sector FROM stocks WHERE market = ? AND code = ?",
    )
    .bind(market.as_str())
    .bind(&stock.code)
    .fetch_optional(pool)
    .await?;

    match existing {
        None => {
            sqlx::query(
                "INSERT INTO stocks (market, code, ticker, exchange, sector, is_active, sort_order) \
                 VALUES (?, ?, ?, ?, ?, 1, ?)",
            )
            .bind(market.as_str())
            .bind(&stock.code)
            .bind(stock.ticker.as_deref())
            .bind(stock.exchange.as_deref())
            .bind(stock.sector.as_deref())
            .bind(stock.sort_order)
            .execute(pool)
            .await?;
            Ok(Upsert::Created)
        }
        Some(row) => {
            let id: i64 = row.try_get("id")?;
            let ticker: Option<String> = row.try_get("ticker")?;
            let exchange: Option<String> = row.try_get("exchange")?;
            let sector: Option<String> = row.try_get("sector")?;

            // Fill in what the sheet knows without overwriting manual edits.
            let new_ticker = ticker.clone().or_else(|| stock.ticker.clone());
            let new_exchange = exchange.clone().or_else(|| stock.exchange.clone());
            let new_sector = sector.clone().or_else(|| stock.sector.clone());
            if new_ticker == ticker && new_exchange == exchange && new_sector == sector {
                return Ok(Upsert::Unchanged);
            }

            sqlx::query("UPDATE stocks SET ticker = ?, exchange = ?, sector = ? WHERE id = ?")
                .bind(new_ticker)
                .bind(new_exchange)
                .bind(new_sector)
                .bind(id)
                .execute(pool)
                .await?;
            Ok(Upsert::Updated)
        }
    }
}

async fn next_stock_sort_order(pool: &SqlitePool, market: Market) -> anyhow::Result<i64> {
    Ok(
        sqlx::query_scalar("SELECT COALESCE(MAX(sort_order), 0) + 1 FROM stocks WHERE market = ?")
            .bind(market.as_str())
            .fetch_one(pool)
            .await?,
    )
}

async fn stock_id(pool: &SqlitePool, market: Market, code: &str) -> anyhow::Result<Option<i64>> {
    Ok(
        sqlx::query_scalar("SELECT id FROM stocks WHERE market = ? AND code = ?")
            .bind(market.as_str())
            .bind(code)
            .fetch_optional(pool)
            .await?,
    )
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct TradeKey {
    stock_id: i64,
    trade_date: String,
    trade_type: String,
    shares: u64,
    total: u64,
}

impl TradeKey {
    fn new(stock_id: i64, trade_date: &str, trade_type: &str, shares: f64, total: f64) -> Self {
        Self {
            stock_id,
            trade_date: trade_date.to_string(),
            trade_type: trade_type.to_string(),
            shares: shares.to_bits(),
            total: total.to_bits(),
        }
    }
}

async fn existing_trade_keys(
    pool: &SqlitePool,
    market: Market,
) -> anyhow::Result<HashMap<TradeKey, usize>> {
    let rows = sqlx::query(
        "SELECT t.stock_id, t.trade_date, t.trade_type, t.shares, t.total FROM trades t \
         JOIN stocks s ON s.id = t.stock_id WHERE s.market = ?",
    )
    .bind(market.as_str())
    .fetch_all(pool)
    .await?;

    let mut keys: HashMap<TradeKey, usize> = HashMap::new();
    for row in &rows {
        let trade_type: String = row.try_get("trade_type")?;
        let trade_date: String = row.try_get("trade_date")?;
        let key = TradeKey::new(
            row.try_get("stock_id")?,
            &trade_date,
            &trade_type,
            row.try_get("shares")?,
            row.try_get("total")?,
        );
        *keys.entry(key).or_insert(0) += 1;
    }
    Ok(keys)
}
