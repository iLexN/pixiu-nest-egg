//! Apply a `current-price.json` file to `stocks.manual_price`.
//!
//! Usage: `cargo run --bin import_prices -- [current-price.json]`
//! The database path comes from `WEALTH_DB`, defaulting to `data/wealth.db`.

use std::path::PathBuf;

use wealth_backend::{db, prices};

const DEFAULT_FILE: &str = "current-price.json";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let file = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_FILE));
    let db_path = std::env::var("WEALTH_DB").unwrap_or_else(|_| db::DEFAULT_DB_PATH.to_string());

    println!("prices: {}", file.display());
    println!("database: {db_path}");

    let body = std::fs::read_to_string(&file)?;
    let upload = prices::parse(&body)?;
    let pool = db::connect(&db_path).await?;
    let report = prices::apply(&pool, upload).await?;
    pool.close().await;

    for entry in &report.updated {
        println!(
            "updated {} {} ({}): {}",
            entry.market.as_str(),
            entry.code,
            entry.symbol,
            entry.price
        );
    }
    println!("updated: {}", report.updated.len());
    if !report.unmatched.is_empty() {
        println!("unmatched symbols: {}", report.unmatched.join(", "));
    }
    if !report.invalid.is_empty() {
        println!("invalid entries: {}", report.invalid.len());
        for entry in &report.invalid {
            println!(
                "  symbol={:?} price={:?}: {}",
                entry.symbol, entry.price, entry.reason
            );
        }
    }
    if !report.not_updated.is_empty() {
        println!(
            "stocks with no price in file: {}",
            report.not_updated.join(", ")
        );
    }
    Ok(())
}
