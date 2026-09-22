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
            Outcome::Info { .. } => {}
        }
    }

    let mut market_figures_compared = 0usize;
    for row in &report.market_figures {
        match &row.outcome {
            Outcome::Match => market_figures_compared += 1,
            Outcome::SkippedNoData => {}
            Outcome::Difference {
                field,
                computed,
                sheet,
            } => println!(
                "DIFF {} {field}: computed {computed}, sheet {sheet}",
                row.name
            ),
            Outcome::MissingSheetValue { field } => println!(
                "DIFF {}: the sheet has no cached {field} to compare against",
                row.name
            ),
            Outcome::MissingStock => {}
            Outcome::Info { .. } => {}
        }
    }

    let mut deposits_compared = 0usize;
    for row in &report.deposits {
        match &row.outcome {
            Outcome::Match => deposits_compared += 1,
            Outcome::SkippedNoData => {}
            Outcome::Difference {
                field,
                computed,
                sheet,
            } => println!(
                "DIFF 定期 {} {field}: computed {computed}, sheet {sheet}",
                row.name
            ),
            Outcome::MissingSheetValue { field } => println!(
                "DIFF 定期 {}: the sheet has no cached {field} to compare against",
                row.name
            ),
            Outcome::MissingStock => {}
            Outcome::Info { .. } => {}
        }
    }

    let mut dividends_compared = 0usize;
    for row in &report.dividends {
        match &row.outcome {
            Outcome::Match => dividends_compared += 1,
            Outcome::SkippedNoData => {}
            Outcome::Difference {
                field,
                computed,
                sheet,
            } => println!(
                "DIFF {} {field}: computed {computed}, sheet {sheet}",
                row.name
            ),
            Outcome::MissingSheetValue { field } => println!(
                "DIFF {}: the sheet has no cached {field} to compare against",
                row.name
            ),
            Outcome::MissingStock => {}
            Outcome::Info { .. } => {}
        }
    }

    let mut mpf_compared = 0usize;
    for row in &report.mpf {
        match &row.outcome {
            Outcome::Match => mpf_compared += 1,
            Outcome::SkippedNoData => {}
            Outcome::Difference {
                field,
                computed,
                sheet,
            } => println!(
                "DIFF {} {field}: computed {computed}, sheet {sheet}",
                row.name
            ),
            Outcome::MissingSheetValue { field } => println!(
                "DIFF {}: the sheet has no cached {field} to compare against",
                row.name
            ),
            Outcome::MissingStock => println!(
                "DIFF {}: listed on the sheet but not in the database",
                row.name
            ),
            Outcome::Info { .. } => {}
        }
    }

    let mut bonds_compared = 0usize;
    for row in &report.bonds {
        match &row.outcome {
            Outcome::Match => bonds_compared += 1,
            Outcome::SkippedNoData => {}
            Outcome::Difference {
                field,
                computed,
                sheet,
            } => println!(
                "DIFF {} {field}: computed {computed}, sheet {sheet}",
                row.name
            ),
            Outcome::MissingSheetValue { field } => println!(
                "DIFF {}: the sheet has no cached {field} to compare against",
                row.name
            ),
            Outcome::MissingStock => println!(
                "DIFF {}: listed on the sheet but not in the database",
                row.name
            ),
            Outcome::Info { .. } => {}
        }
    }

    let mut aia_compared = 0usize;
    for row in &report.aia {
        match &row.outcome {
            Outcome::Match => aia_compared += 1,
            Outcome::SkippedNoData => {}
            Outcome::Difference {
                field,
                computed,
                sheet,
            } => println!(
                "DIFF {} {field}: computed {computed}, sheet {sheet}",
                row.name
            ),
            Outcome::MissingSheetValue { field } => println!(
                "DIFF {}: the sheet has no cached {field} to compare against",
                row.name
            ),
            Outcome::MissingStock => println!(
                "DIFF {}: listed on the sheet but not in the database",
                row.name
            ),
            Outcome::Info { .. } => {}
        }
    }

    let mut months_compared = 0usize;
    let mut months_info = 0usize;
    for row in &report.months {
        match &row.outcome {
            Outcome::Match => months_compared += 1,
            Outcome::SkippedNoData => {}
            Outcome::Difference {
                field,
                computed,
                sheet,
            } => println!(
                "DIFF Month Stat {} {field}: computed {computed}, sheet {sheet}",
                row.name
            ),
            Outcome::MissingSheetValue { field } => println!(
                "DIFF Month Stat {}: the sheet has no cached {field} to compare against",
                row.name
            ),
            Outcome::MissingStock => println!(
                "DIFF Month Stat {}: on the sheet but not in the database",
                row.name
            ),
            Outcome::Info { computed, sheet } => {
                months_info += 1;
                println!("INFO {}: computed {computed}, sheet {sheet}", row.name)
            }
        }
    }

    let mut overview_compared = 0usize;
    let mut overview_info = 0usize;
    for row in &report.overview {
        match &row.outcome {
            Outcome::Match => overview_compared += 1,
            Outcome::SkippedNoData => {}
            Outcome::Difference {
                field,
                computed,
                sheet,
            } => println!(
                "DIFF {} {field}: computed {computed}, sheet {sheet}",
                row.name
            ),
            Outcome::MissingSheetValue { field } => println!(
                "DIFF {}: the sheet has no cached {field} to compare against",
                row.name
            ),
            Outcome::MissingStock => {
                println!("DIFF {}: on the sheet but not in the database", row.name)
            }
            Outcome::Info { computed, sheet } => {
                overview_info += 1;
                println!("INFO {}: computed {computed}, sheet {sheet}", row.name)
            }
        }
    }

    let problems = report.problem_count();
    println!(
        "{compared} stock(s) match, {market_figures_compared} market figure(s) match, \
         {deposits_compared} deposit figure(s) match, \
         {dividends_compared} dividend figure(s) match, {mpf_compared} MPF figure(s) match, \
         {bonds_compared} bond figure(s) match, {aia_compared} AIA figure(s) match, \
         {months_compared} month figure(s) match ({months_info} informational), \
         {overview_compared} overview figure(s) match ({overview_info} informational), \
         {problems} difference(s)"
    );
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
