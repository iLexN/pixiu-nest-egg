//! Compares the app's computed summary against the workbook's cached values.

use anyhow::anyhow;
use sqlx::SqlitePool;

use crate::calc::{
    active_month_rollup, active_totals, approx_eq, bank_rollup, year_rollups, DepositFacts,
};
use crate::models::Market;
use crate::xlsx::{SheetActiveSums, SheetYearSums, WorkbookData};

#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    Match,
    /// The computed and cached figures disagree beyond tolerance.
    Difference {
        field: &'static str,
        computed: f64,
        sheet: f64,
    },
    /// The sheet holds no cached value for a stock that has trades.
    MissingSheetValue {
        field: &'static str,
    },
    /// The sheet lists the stock but neither side has figures to compare.
    SkippedNoData,
    /// The sheet lists a stock the database does not know about.
    MissingStock,
}

#[derive(Debug, Clone)]
pub struct ParityRow {
    pub market: Market,
    pub code: String,
    pub outcome: Outcome,
}

#[derive(Debug, Clone)]
pub struct DepositParityRow {
    /// What is being compared, e.g. `定期 active principal` or `2027 1月`.
    pub name: String,
    pub outcome: Outcome,
}

#[derive(Debug, Clone, Default)]
pub struct ParityReport {
    pub rows: Vec<ParityRow>,
    pub deposits: Vec<DepositParityRow>,
}

impl ParityReport {
    pub fn problems(&self) -> impl Iterator<Item = &ParityRow> {
        self.rows
            .iter()
            .filter(|row| !matches!(row.outcome, Outcome::Match | Outcome::SkippedNoData))
    }

    pub fn deposit_problems(&self) -> impl Iterator<Item = &DepositParityRow> {
        self.deposits
            .iter()
            .filter(|row| !matches!(row.outcome, Outcome::Match | Outcome::SkippedNoData))
    }

    pub fn problem_count(&self) -> usize {
        self.problems().count() + self.deposit_problems().count()
    }

    pub fn is_clean(&self) -> bool {
        self.problem_count() == 0
    }
}

pub async fn check(pool: &SqlitePool, data: &WorkbookData) -> anyhow::Result<ParityReport> {
    let mut report = ParityReport::default();
    for market in [Market::Hk, Market::Us] {
        check_market(pool, data, market, &mut report).await?;
    }
    check_deposits(pool, data, &mut report).await?;
    Ok(report)
}

async fn check_market(
    pool: &SqlitePool,
    data: &WorkbookData,
    market: Market,
    report: &mut ParityReport,
) -> anyhow::Result<()> {
    let summary = crate::routes::summary::build(pool, market)
        .await
        .map_err(|err| anyhow!("computing the {} summary failed: {err}", market.as_str()))?;

    for sheet in &data.market(market).summary {
        let Some(computed) = summary
            .stocks
            .iter()
            .find(|row| row.stock.code == sheet.code)
        else {
            report.rows.push(ParityRow {
                market,
                code: sheet.code.clone(),
                outcome: Outcome::MissingStock,
            });
            continue;
        };

        let has_trades = computed.trade_count > 0;
        let comparisons: [(&'static str, Option<f64>, f64); 3] = [
            ("股數", sheet.shares_held, computed.summary.shares_held),
            (
                "總買入成本",
                sheet.total_buy_cost,
                computed.summary.total_buy_cost,
            ),
            (
                "加權平均買入單價",
                sheet.weighted_avg_buy_price,
                computed.summary.weighted_avg_buy_price,
            ),
        ];

        let mut outcome = Outcome::Match;
        let mut sheet_values = 0;
        for (field, sheet_value, computed_value) in comparisons {
            match sheet_value {
                None if has_trades => {
                    outcome = Outcome::MissingSheetValue { field };
                    break;
                }
                None => continue,
                Some(sheet_value) => {
                    sheet_values += 1;
                    if !approx_eq(computed_value, sheet_value) {
                        outcome = Outcome::Difference {
                            field,
                            computed: computed_value,
                            sheet: sheet_value,
                        };
                        break;
                    }
                }
            }
        }
        if outcome == Outcome::Match && sheet_values == 0 && !has_trades {
            outcome = Outcome::SkippedNoData;
        }

        report.rows.push(ParityRow {
            market,
            code: sheet.code.clone(),
            outcome,
        });
    }

    Ok(())
}

fn deposit_row(report: &mut ParityReport, name: impl Into<String>, outcome: Outcome) {
    report.deposits.push(DepositParityRow {
        name: name.into(),
        outcome,
    });
}

/// First difference wins, like check_market. `field` names the column
/// (total / 利息 / 定期) so the report says which sum diverged.
fn compare_active_sums(
    report: &mut ParityReport,
    name: impl Into<String>,
    computed: &SheetActiveSums,
    sheet: &SheetActiveSums,
) {
    for (field, computed_value, sheet_value) in [
        ("Total", computed.total, sheet.total),
        ("利息", computed.interest, sheet.interest),
        ("定期", computed.principal, sheet.principal),
    ] {
        if !approx_eq(computed_value, sheet_value) {
            deposit_row(
                report,
                name,
                Outcome::Difference {
                    field,
                    computed: computed_value,
                    sheet: sheet_value,
                },
            );
            return;
        }
    }
    deposit_row(report, name, Outcome::Match);
}

fn compare_year_sums(
    report: &mut ParityReport,
    name: impl Into<String>,
    computed: &SheetYearSums,
    sheet: &SheetYearSums,
) {
    for (field, computed_value, sheet_value) in [
        ("Total", computed.total, sheet.total),
        ("利息", computed.interest, sheet.interest),
        ("定期", computed.payout, sheet.payout),
    ] {
        if !approx_eq(computed_value, sheet_value) {
            deposit_row(
                report,
                name,
                Outcome::Difference {
                    field,
                    computed: computed_value,
                    sheet: sheet_value,
                },
            );
            return;
        }
    }
    deposit_row(report, name, Outcome::Match);
}

/// Deposit aggregates against the workbook's cached values. Status is
/// date-derived, so cached figures reflect the sheet's last recalculation.
async fn check_deposits(
    pool: &SqlitePool,
    data: &WorkbookData,
    report: &mut ParityReport,
) -> anyhow::Result<()> {
    let today = crate::routes::today();
    let deposits = crate::routes::deposits::load_all(pool)
        .await
        .map_err(|err| anyhow!("loading deposits failed: {err}"))?;
    let facts: Vec<DepositFacts<'_>> = deposits
        .iter()
        .map(|deposit| {
            chrono::NaiveDate::parse_from_str(&deposit.end_date, "%Y-%m-%d")
                .map(|end_date| DepositFacts {
                    bank: deposit.bank.as_deref(),
                    principal: deposit.principal,
                    interest: deposit.interest,
                    end_date,
                })
                .map_err(|_| anyhow!("stored end_date {} is not valid", deposit.end_date))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let cached = &data.deposit_cached;
    let totals = active_totals(&facts, today);
    match cached.active_principal {
        Some(sheet) if approx_eq(totals.principal, sheet) => {
            deposit_row(report, "定期 active principal", Outcome::Match)
        }
        Some(sheet) => deposit_row(
            report,
            "定期 active principal",
            Outcome::Difference {
                field: "input",
                computed: totals.principal,
                sheet,
            },
        ),
        None => deposit_row(
            report,
            "定期 active principal",
            Outcome::MissingSheetValue { field: "input" },
        ),
    }

    // The sheet's month table buckets by month number only, so the computed
    // (year, month) buckets are merged by month before comparing.
    let computed_months = active_month_rollup(&facts, today);
    for (month, sheet) in &cached.months {
        let mut computed = SheetActiveSums {
            total: 0.0,
            interest: 0.0,
            principal: 0.0,
        };
        for bucket in computed_months
            .iter()
            .filter(|bucket| bucket.month == *month)
        {
            computed.principal += bucket.principal;
            computed.interest += bucket.interest;
            computed.total += bucket.total;
        }
        compare_active_sums(report, format!("定期 {month}月"), &computed, sheet);
    }
    if let Some(sheet) = &cached.grand_total {
        let computed = SheetActiveSums {
            total: totals.total,
            interest: totals.interest,
            principal: totals.principal,
        };
        compare_active_sums(report, "定期 month-table total", &computed, sheet);
    }

    let computed_banks = bank_rollup(&facts, today);
    for (prefix, sheet) in &cached.banks {
        let computed = computed_banks
            .iter()
            .find(|group| group.bank == *prefix)
            .map(|group| SheetActiveSums {
                total: group.total,
                interest: group.interest,
                principal: group.principal,
            })
            .unwrap_or(SheetActiveSums {
                total: 0.0,
                interest: 0.0,
                principal: 0.0,
            });
        compare_active_sums(report, format!("定期 bank {prefix}"), &computed, sheet);
    }

    let computed_years = year_rollups(&facts);
    for sheet_year in &cached.years {
        let computed_year = computed_years
            .iter()
            .find(|rollup| rollup.year == sheet_year.year);
        for (month, sheet) in &sheet_year.months {
            let computed = computed_year
                .and_then(|rollup| rollup.months.iter().find(|row| row.month == *month))
                .map(|row| SheetYearSums {
                    total: row.total,
                    interest: row.interest,
                    payout: row.payout,
                })
                .unwrap_or(SheetYearSums {
                    total: 0.0,
                    interest: 0.0,
                    payout: 0.0,
                });
            let empty = computed.total == 0.0
                && computed.interest == 0.0
                && computed.payout == 0.0
                && sheet.total == 0.0
                && sheet.interest == 0.0
                && sheet.payout == 0.0;
            if empty {
                continue;
            }
            compare_year_sums(
                report,
                format!("{} {month}月", sheet_year.year),
                &computed,
                sheet,
            );
        }
    }

    Ok(())
}
