//! Bulk 現價 updates: from a `current-price.json` style file and from a
//! user-triggered Yahoo Finance fetch.
//!
//! Shared by `POST /api/stocks/prices`, `POST /api/stocks/prices/fetch`, and
//! the `import_prices` bin so every entry point resolves symbols and reports
//! results identically — a fetch builds the same `{"stocks": [...]}` upload
//! a file would carry, then runs the same `apply` pipeline.

use std::collections::HashSet;
use std::time::Duration;

use serde::Deserialize;
use sqlx::SqlitePool;

use crate::error::ApiError;
use crate::models::{
    Currency, InvalidPriceEntry, Market, PriceReport, PriceUpdate, RawPriceEntry, Stock,
};
use crate::store::now_timestamp;
use crate::store::stocks::load_stocks;

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
        crate::store::stocks::set_price(&mut transaction, entry.price, &now, entry.id).await?;
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
            crate::service::summary::record_market(pool, market).await?;
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
        failed: Vec::new(),
        fx: None,
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

/// Yahoo's chart endpoint; a plain browser `User-Agent` keeps it from
/// answering 429.
const CHART_URL: &str = "https://query1.finance.yahoo.com/v8/finance/chart";
const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 \
     (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";
const FX_SYMBOL: &str = "USDHKD=X";

/// Fetch the latest Yahoo price for every stock in `market` and run the
/// results through `apply`, so a fetch behaves exactly like a price file.
/// Per-symbol failures are reported, never fatal; a US fetch also refreshes
/// the stored USD rate from `USDHKD=X` (raw, unrounded).
pub async fn fetch_market(pool: &SqlitePool, market: Market) -> Result<PriceReport, ApiError> {
    match http_client() {
        // A client that cannot start fails every symbol the same way.
        Err(reason) => {
            fetch_market_inner(pool, market, move |_| {
                let reason = reason.clone();
                async move { Err(reason) }
            })
            .await
        }
        Ok(client) => {
            fetch_market_inner(pool, market, move |symbol| {
                let client = client.clone();
                async move { fetch_price(&client, &symbol).await }
            })
            .await
        }
    }
}

/// The fetch orchestration behind `fetch_market`, with the price source
/// injected so the failure paths are testable without HTTP.
async fn fetch_market_inner<F, Fut>(
    pool: &SqlitePool,
    market: Market,
    mut fetch: F,
) -> Result<PriceReport, ApiError>
where
    F: FnMut(String) -> Fut,
    Fut: Future<Output = Result<f64, String>>,
{
    let stocks = load_stocks(pool, Some(market)).await?;
    let mut entries = Vec::new();
    let mut failed = Vec::new();
    let mut fx = None;
    let mut first = true;

    for stock in &stocks {
        let Some(symbol) = yahoo_symbol(stock) else {
            failed.push(InvalidPriceEntry {
                symbol: Some(stock.code.clone()),
                price: None,
                reason: "no ticker to map to a Yahoo symbol".to_string(),
            });
            continue;
        };
        pace(&mut first).await;
        match fetch(symbol.clone()).await {
            Ok(price) => entries.push(RawPriceEntry {
                symbol: Some(symbol),
                price: Some(round_price(market, price)),
            }),
            Err(reason) => failed.push(InvalidPriceEntry {
                symbol: Some(symbol),
                price: None,
                reason,
            }),
        }
    }
    if market == Market::Us {
        pace(&mut first).await;
        match fetch(FX_SYMBOL.to_string()).await {
            Ok(rate) => {
                crate::store::fx::set(pool, Currency::Usd, rate, &now_timestamp()).await?;
                fx = Some(rate);
            }
            Err(reason) => failed.push(InvalidPriceEntry {
                symbol: Some(FX_SYMBOL.to_string()),
                price: None,
                reason,
            }),
        }
    }

    let mut report = apply(pool, PriceUpload { stocks: entries }).await?;
    report.failed = failed;
    report.fx = fx;
    // A market-scoped fetch shouldn't list the other market's stocks as
    // unupdated — the report describes only what the user asked to fetch.
    report
        .not_updated
        .retain(|entry| entry.starts_with(market.as_str()));
    Ok(report)
}

fn http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|err| format!("cannot start HTTP client: {err}"))
}

/// ~200ms between requests so the batch does not trip Yahoo's rate limit;
/// the first request goes out immediately.
async fn pace(first: &mut bool) {
    if *first {
        *first = false;
    } else {
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

/// GET one chart response and pull its price. Every failure mode folds
/// into a reason string so a bad symbol never kills the batch.
async fn fetch_price(client: &reqwest::Client, symbol: &str) -> Result<f64, String> {
    let response = client
        .get(format!("{CHART_URL}/{symbol}?interval=1d&range=1d"))
        .send()
        .await
        .map_err(|err| format!("request failed: {err}"))?;
    if !response.status().is_success() {
        return Err(format!("HTTP {}", response.status()));
    }
    let body = response
        .text()
        .await
        .map_err(|err| format!("read failed: {err}"))?;
    parse_chart_price(&body).ok_or_else(|| "no price in response".to_string())
}

/// The Yahoo query symbol for a stored stock: `{ticker}.HK` for HK (`None`
/// when the stock has no ticker), `code` with `.` → `-` for US — the exact
/// inverse of `resolve`'s normalization.
fn yahoo_symbol(stock: &Stock) -> Option<String> {
    match stock.market {
        Market::Hk => stock.ticker.as_deref().map(|ticker| format!("{ticker}.HK")),
        Market::Us => Some(stock.code.replace('.', "-")),
    }
}

/// `chart.result[0].meta.regularMarketPrice`, falling back to
/// `previousClose` when the live price is absent.
fn parse_chart_price(body: &str) -> Option<f64> {
    let json: serde_json::Value = serde_json::from_str(body).ok()?;
    let meta = json.pointer("/chart/result/0/meta")?;
    meta.get("regularMarketPrice")
        .and_then(serde_json::Value::as_f64)
        .or_else(|| {
            meta.get("previousClose")
                .and_then(serde_json::Value::as_f64)
        })
}

/// Display-grade rounding at the fetch boundary: HK 3dp, US 2dp. The FX
/// rate is never rounded (the fx-rates spec forbids it).
fn round_price(market: Market, price: f64) -> f64 {
    let digits = match market {
        Market::Hk => 3,
        Market::Us => 2,
    };
    let factor = 10f64.powi(digits);
    (price * factor).round() / factor
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

    fn stock(market: Market, code: &str, ticker: Option<&str>) -> Stock {
        Stock {
            id: 1,
            market,
            code: code.to_string(),
            ticker: ticker.map(str::to_string),
            exchange: None,
            sector: None,
            manual_price: None,
            price_updated_at: None,
            pe: None,
            eps: None,
            high52: None,
            low52: None,
            note: None,
            is_active: true,
            sort_order: 1,
        }
    }

    #[test]
    fn yahoo_symbol_maps_each_market() {
        assert_eq!(
            yahoo_symbol(&stock(Market::Hk, "中國銀行", Some("3988"))).as_deref(),
            Some("3988.HK")
        );
        assert_eq!(yahoo_symbol(&stock(Market::Hk, "無代碼", None)), None);
        assert_eq!(
            yahoo_symbol(&stock(Market::Us, "BRK.B", Some("BRK.B"))).as_deref(),
            Some("BRK-B")
        );
        assert_eq!(
            yahoo_symbol(&stock(Market::Us, "VOO", Some("VOO"))).as_deref(),
            Some("VOO")
        );
    }

    #[test]
    fn parse_chart_price_prefers_live_then_close() {
        let live =
            r#"{"chart":{"result":[{"meta":{"regularMarketPrice":5.975,"previousClose":5.9}}]}}"#;
        assert_eq!(parse_chart_price(live), Some(5.975));

        let closed =
            r#"{"chart":{"result":[{"meta":{"regularMarketPrice":null,"previousClose":5.9}}]}}"#;
        assert_eq!(parse_chart_price(closed), Some(5.9));

        assert_eq!(
            parse_chart_price(r#"{"chart":{"result":[{"meta":{}}]}}"#),
            None
        );
        assert_eq!(parse_chart_price(r#"{"chart":{"result":[]}}"#), None);
        assert_eq!(parse_chart_price(r#"{"chart":{"error":{}}}"#), None);
        assert_eq!(parse_chart_price("not json"), None);
    }

    #[test]
    fn round_price_uses_market_precision() {
        assert_eq!(round_price(Market::Hk, 5.9754), 5.975);
        assert_eq!(round_price(Market::Hk, 5.9756), 5.976);
        assert_eq!(round_price(Market::Us, 514.954), 514.95);
    }

    #[tokio::test]
    async fn fetch_inner_reports_failures_and_scopes_not_updated() {
        let pool = db::connect_memory().await.expect("memory db");
        add_stock(&pool, "HK", "中國銀行", Some("3988")).await;
        add_stock(&pool, "HK", "港燈", Some("2638")).await;
        add_stock(&pool, "HK", "無代碼", None).await;
        add_stock(&pool, "US", "VOO", Some("VOO")).await;

        let report = fetch_market_inner(&pool, Market::Hk, |symbol| async move {
            match symbol.as_str() {
                "3988.HK" => Ok(6.2914),
                _ => Err("boom".to_string()),
            }
        })
        .await
        .expect("fetch");

        assert_eq!(report.updated.len(), 1);
        assert_eq!(report.updated[0].symbol, "3988.HK");
        assert_eq!(report.updated[0].price, 6.291);
        // The fetch failure and the unmapped stock are reported, not fatal.
        assert_eq!(report.failed.len(), 2);
        assert!(report.fx.is_none());
        // The other market's stocks are out of scope for this fetch.
        assert_eq!(report.not_updated, vec!["HK 港燈", "HK 無代碼"]);
    }

    #[tokio::test]
    async fn fetch_inner_stores_the_usd_rate_unrounded() {
        let pool = db::connect_memory().await.expect("memory db");
        add_stock(&pool, "US", "VOO", Some("VOO")).await;

        let report = fetch_market_inner(&pool, Market::Us, |symbol| async move {
            match symbol.as_str() {
                "USDHKD=X" => Ok(7.84304567),
                _ => Ok(699.304),
            }
        })
        .await
        .expect("fetch");

        assert_eq!(report.updated[0].price, 699.3);
        assert_eq!(report.fx, Some(7.84304567));
        let rate = crate::store::fx::usd_rate(&pool).await.expect("rate");
        assert_eq!(rate, Some(7.84304567));
    }

    #[tokio::test]
    async fn fetch_inner_leaves_the_rate_when_fx_fetch_fails() {
        let pool = db::connect_memory().await.expect("memory db");
        add_stock(&pool, "US", "VOO", Some("VOO")).await;
        crate::store::fx::set(&pool, Currency::Usd, 7.8, "before")
            .await
            .expect("seed rate");

        let report = fetch_market_inner(&pool, Market::Us, |symbol| async move {
            match symbol.as_str() {
                "USDHKD=X" => Err("HTTP 429".to_string()),
                _ => Ok(700.0),
            }
        })
        .await
        .expect("fetch");

        // The stock price still applied; the rate failure is only reported.
        assert_eq!(report.updated.len(), 1);
        assert_eq!(report.failed.len(), 1);
        assert_eq!(report.failed[0].symbol.as_deref(), Some("USDHKD=X"));
        assert!(report.fx.is_none());
        let rate = crate::store::fx::usd_rate(&pool).await.expect("rate");
        assert_eq!(rate, Some(7.8));
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
