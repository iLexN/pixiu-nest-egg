//! Compare the app's computed summary against the workbook's cached values.
//!
//! Usage: `cargo run --bin check_parity -- [workbook.xlsx]`
//! Exits non-zero when anything differs, so it can gate the migration.

use std::path::PathBuf;
use std::process::ExitCode;

use wealth_backend::parity::Outcome;
use wealth_backend::{db, parity, xlsx};

const DEFAULT_WORKBOOK: &str = "財富分析報告.xlsx";

#[tokio::main]
async fn main() -> anyhow::Result<ExitCode> {
    let workbook = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(default_workbook);
    let db_path = std::env::var("WEALTH_DB").unwrap_or_else(|_| db::DEFAULT_DB_PATH.to_string());

    println!("workbook: {}", workbook.display());
    println!("database: {db_path}");

    let data = xlsx::read(&workbook)?;
    let pool = db::connect(&db_path).await?;
    let report = parity::check(&pool, &data).await?;
    pool.close().await;

    let mut compared = 0usize;
    for row in &report.rows {
        match &row.outcome {
            Outcome::Match => compared += 1,
            Outcome::SkippedNoData => println!(
                "skipped {} {}: no figures on either side",
                row.market.as_str(),
                row.code
            ),
            Outcome::Difference {
                field,
                computed,
                sheet,
            } => println!(
                "DIFF {} {} {field}: computed {computed}, sheet {sheet}",
                row.market.as_str(),
                row.code
            ),
            Outcome::MissingSheetValue { field } => println!(
                "DIFF {} {}: the sheet has no cached {field} to compare against",
                row.market.as_str(),
                row.code
            ),
            Outcome::MissingStock => println!(
                "DIFF {} {}: listed on the sheet but not in the database",
                row.market.as_str(),
                row.code
            ),
        }
    }

    let problems = report.problems().count();
    println!("{compared} stock(s) match, {problems} difference(s)");
    Ok(if problems == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

fn default_workbook() -> PathBuf {
    let here = PathBuf::from(DEFAULT_WORKBOOK);
    if here.exists() {
        here
    } else {
        PathBuf::from("..").join(DEFAULT_WORKBOOK)
    }
}
