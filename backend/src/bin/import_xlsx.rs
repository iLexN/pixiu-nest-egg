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
             stocks {} created, {} updated; 派息 {} row(s) -> {} imported, {} skipped",
            market.as_str(),
            sheets.trades.len(),
            market_report.trades_imported,
            market_report.trades_skipped,
            market_report.stocks_created,
            market_report.stocks_updated,
            sheets.dividends.len(),
            market_report.dividends_imported,
            market_report.dividends_skipped,
        );
        for pending in &market_report.dividends_pending {
            println!("  派息 pending (imported as estimate): {pending}");
        }
    }
    println!(
        "定期: {} deposit row(s) in the sheet -> {} imported, {} skipped (already present)",
        data.deposits.len(),
        report.deposits.deposits_imported,
        report.deposits.deposits_skipped,
    );
    if report.deposits.start_dates_seeded > 0 {
        println!(
            "  定期 start_date seeded on {} existing row(s)",
            report.deposits.start_dates_seeded
        );
    }
    println!(
        "year snapshots: {} seeded, {} skipped (current year or empty)",
        report.snapshots.snapshots_seeded, report.snapshots.snapshots_skipped,
    );
    println!(
        "MPF: {} account row(s) in the sheet -> {} imported, {} skipped (already present)",
        data.mpf.len(),
        report.mpf.accounts_created,
        report.mpf.accounts_skipped,
    );
    println!(
        "債券: {} bond row(s) in the sheet -> {} imported, {} skipped; 付息 {} row(s) -> {} imported, {} skipped",
        data.bonds.len(),
        report.bonds.bonds_imported,
        report.bonds.bonds_skipped,
        data.bonds.iter().map(|bond| bond.coupons.len()).sum::<usize>(),
        report.bonds.coupons_imported,
        report.bonds.coupons_skipped,
    );
    println!(
        "AIA: {} policy row(s) in the sheet -> {} imported, {} skipped (already present); rate seeded: {}",
        data.aia.len(),
        report.aia.policies_imported,
        report.aia.policies_skipped,
        report.aia.rate_seeded == 1,
    );
    for warning in &report.aia.warnings {
        println!("  AIA warning: {warning}");
    }
    println!(
        "Month Stat: {} month row(s) in the sheet -> {} imported, {} skipped (already present); \
         {} item(s) imported; {} setting(s) seeded; {} 月尾 override(s) seeded",
        data.month_stat.months.len(),
        report.months.months_imported,
        report.months.months_skipped,
        report.months.items_imported,
        report.months.settings_seeded,
        report.months.end_cash_seeded,
    );

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
