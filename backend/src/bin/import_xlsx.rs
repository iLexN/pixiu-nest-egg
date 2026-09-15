//! Import the spreadsheet's trade history into the SQLite database.
//!
//! Usage: `cargo run --bin import_xlsx -- [workbook.xlsx]`
//! The database path comes from `WEALTH_DB`, defaulting to `data/wealth.db`.

use std::path::PathBuf;

use wealth_backend::models::Market;
use wealth_backend::{db, import, xlsx};

const DEFAULT_WORKBOOK: &str = "財富分析報告.xlsx";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let workbook = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(default_workbook);
    let db_path = std::env::var("WEALTH_DB").unwrap_or_else(|_| db::DEFAULT_DB_PATH.to_string());

    println!("workbook: {}", workbook.display());
    println!("database: {db_path}");

    let data = xlsx::read(&workbook)?;
    let pool = db::connect(&db_path).await?;
    let report = import::import(&pool, &data).await?;

    for market in [Market::Hk, Market::Us] {
        let market_report = report.market(market);
        let sheets = data.market(market);
        println!(
            "{}: {} trade row(s) in the sheet -> {} imported, {} skipped (already present); \
             stocks {} created, {} updated",
            market.as_str(),
            sheets.trades.len(),
            market_report.trades_imported,
            market_report.trades_skipped,
            market_report.stocks_created,
            market_report.stocks_updated,
        );
    }

    pool.close().await;
    Ok(())
}

fn default_workbook() -> PathBuf {
    let here = PathBuf::from(DEFAULT_WORKBOOK);
    if here.exists() {
        here
    } else {
        PathBuf::from("..").join(DEFAULT_WORKBOOK)
    }
}
