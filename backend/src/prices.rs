//! Bulk 現價 import from a `current-price.json` style file.
//!
//! Shared by `POST /api/stocks/prices` and the `import_prices` bin so both
//! entry points resolve symbols and report results identically. The file is
//! user-supplied input: no external market-data service is involved.

use std::collections::HashSet;

use serde::Deserialize;
use sqlx::SqlitePool;

use crate::error::ApiError;
use crate::models::{InvalidPriceEntry, Market, PriceReport, PriceUpdate, RawPriceEntry, Stock};
use crate::routes::now_timestamp;
use crate::routes::stocks::load_stocks;

/// Parsed `{"stocks": [...]}` body shared by the route and the CLI.
#[derive(Debug, Deserialize)]
pub struct PriceUpload {
    pub stocks: Vec<RawPriceEntry>,
}

pub fn parse(body: &str) -> Result<PriceUpload, ApiError> {
    serde_json::from_str(body)
        .map_err(|err| ApiError::field("file", format!("not a valid price file: {err}")))
}

/// Resolve every entry, then apply all matched prices in one transaction.
/// Entries that fail to resolve or are malformed are reported, not fatal.
pub async fn apply(pool: &SqlitePool, upload: PriceUpload) -> Result<PriceReport, ApiError> {
    let hk = load_stocks(pool, Some(Market::Hk)).await?;
    let us = load_stocks(pool, Some(Market::Us)).await?;

    struct Resolved {
        id: i64,
        market: Market,
        code: String,
        symbol: String,
        price: f64,
    }

    let mut resolved = Vec::new();
    let mut unmatched = Vec::new();
    let mut invalid = Vec::new();
    for entry in &upload.stocks {
        let symbol = entry.symbol.as_deref().map(str::trim).unwrap_or_default();
        let price = entry.price;
        let reason = if symbol.is_empty() {
            Some("symbol is required".to_string())
        } else if price.is_none_or(|p| !p.is_finite() || p <= 0.0) {
            Some("price must be a positive number".to_string())
        } else {
            None
        };
        if let Some(reason) = reason {
            invalid.push(InvalidPriceEntry {
                symbol: entry.symbol.clone(),
                price: entry.price,
                reason,
            });
            continue;
        }
        match resolve(&hk, &us, symbol) {
            Some(stock) => resolved.push(Resolved {
                id: stock.id,
                market: stock.market,
                code: stock.code.clone(),
                symbol: symbol.to_string(),
                price: price.unwrap_or_default(),
            }),
            None => unmatched.push(symbol.to_string()),
        }
    }

    let now = now_timestamp();
    let mut updated_ids = HashSet::new();
    let mut updated = Vec::new();
    let mut transaction = pool.begin().await?;
    for entry in &resolved {
        sqlx::query("UPDATE stocks SET manual_price = ?, price_updated_at = ? WHERE id = ?")
            .bind(entry.price)
            .bind(&now)
            .bind(entry.id)
            .execute(&mut *transaction)
            .await?;
        updated_ids.insert(entry.id);
        updated.push(PriceUpdate {
            market: entry.market,
            code: entry.code.clone(),
            symbol: entry.symbol.clone(),
            price: entry.price,
        });
    }
    transaction.commit().await?;

    // Record today's totals for every touched market: a bulk import can
    // change both markets without any summary view following it (the CLI
    // `import_prices` bin shares this path).
    for market in [Market::Hk, Market::Us] {
        if resolved.iter().any(|entry| entry.market == market) {
            crate::routes::summary::build(pool, market).await?;
        }
    }

    let not_updated = hk
        .iter()
        .chain(us.iter())
        .filter(|stock| !updated_ids.contains(&stock.id))
        .map(|stock| format!("{} {}", stock.market.as_str(), stock.code))
        .collect();

    Ok(PriceReport {
        updated,
        unmatched,
        invalid,
        not_updated,
    })
}

/// `NNNN.HK` resolves to an HK stock by ticker; anything else resolves to a
/// US stock by code or ticker, with `-` treated as `.` (Yahoo's `BRK-B`).
fn resolve<'a>(hk: &'a [Stock], us: &'a [Stock], symbol: &str) -> Option<&'a Stock> {
    if symbol.len() > 3 && symbol.to_ascii_uppercase().ends_with(".HK") {
        let ticker = &symbol[..symbol.len() - 3];
        return hk.iter().find(|s| s.ticker.as_deref() == Some(ticker));
    }
    let normalized = symbol.replace('-', ".");
    us.iter()
        .find(|s| s.code == normalized || s.ticker.as_deref() == Some(normalized.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    async fn add_stock(pool: &SqlitePool, market: &str, code: &str, ticker: Option<&str>) -> i64 {
        sqlx::query_scalar(
            "INSERT INTO stocks (market, code, ticker, is_active, sort_order) \
             VALUES (?, ?, ?, 1, 1) RETURNING id",
        )
        .bind(market)
        .bind(code)
        .bind(ticker)
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

    #[test]
    fn parse_rejects_malformed_bodies() {
        assert!(parse("not json").is_err());
        assert!(parse("{}").is_err());
        assert!(parse(r#"{"stocks": "nope"}"#).is_err());
        assert!(parse(r#"{"stocks": []}"#).is_ok());
    }

    #[tokio::test]
    async fn apply_resolves_reports_and_updates() {
        let pool = db::connect_memory().await.expect("memory db");
        add_stock(&pool, "HK", "中國銀行", Some("3988")).await;
        add_stock(&pool, "HK", "ＦＧ恆生紅利", Some("3031")).await;
        add_stock(&pool, "US", "VOO", Some("VOO")).await;
        add_stock(&pool, "US", "BRK.B", Some("BRK.B")).await;

        let upload = parse(
            r#"{"stocks": [
                {"symbol": "3988.HK", "price": 5.975},
                {"symbol": "VOO", "price": 699.3},
                {"symbol": "BRK-B", "price": 514.95},
                {"symbol": "1310.HK", "price": 5.47},
                {"symbol": "", "price": 1.0},
                {"price": 1.0},
                {"symbol": "BE", "price": -1}
            ]}"#,
        )
        .expect("parse");
        let report = apply(&pool, upload).await.expect("apply");

        assert_eq!(report.updated.len(), 3);
        assert_eq!(report.unmatched, vec!["1310.HK"]);
        assert_eq!(report.invalid.len(), 3);
        assert_eq!(report.not_updated, vec!["HK ＦＧ恆生紅利"]);

        let (price, stamped): (f64, Option<String>) = sqlx::query_as(
            "SELECT manual_price, price_updated_at FROM stocks WHERE market = 'US' AND code = 'BRK.B'",
        )
        .fetch_one(&pool)
        .await
        .expect("BRK.B row");
        assert_eq!(price, 514.95);
        assert!(stamped.is_some());
    }

    #[tokio::test]
    async fn apply_records_history_for_every_updated_market() {
        let pool = db::connect_memory().await.expect("memory db");
        let hk_id = add_stock(&pool, "HK", "中國銀行", Some("3988")).await;
        let us_id = add_stock(&pool, "US", "VOO", Some("VOO")).await;
        add_buy(&pool, hk_id, 100.0, 500.0).await;
        add_buy(&pool, us_id, 10.0, 5000.0).await;

        let upload = parse(
            r#"{"stocks": [
                {"symbol": "3988.HK", "price": 6.0},
                {"symbol": "VOO", "price": 700.0}
            ]}"#,
        )
        .expect("parse");
        apply(&pool, upload).await.expect("apply");

        let rows: Vec<(String, f64, f64)> = sqlx::query_as(
            "SELECT market, buy_cost_priced, market_value FROM market_history \
             ORDER BY market",
        )
        .fetch_all(&pool)
        .await
        .expect("history");
        assert_eq!(
            rows,
            vec![
                ("HK".to_string(), 500.0, 600.0),
                ("US".to_string(), 5000.0, 7000.0),
            ]
        );
    }
}
