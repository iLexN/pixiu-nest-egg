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
pub struct NamedParityRow {
    /// What is being compared, e.g. `定期 active principal` or `派息 HK count`.
    pub name: String,
    pub outcome: Outcome,
}

#[derive(Debug, Clone, Default)]
pub struct ParityReport {
    pub rows: Vec<ParityRow>,
    /// Market-level cached 上月/最高 cells versus the derived figures.
    pub market_figures: Vec<NamedParityRow>,
    pub deposits: Vec<NamedParityRow>,
    pub dividends: Vec<NamedParityRow>,
    pub mpf: Vec<NamedParityRow>,
}

impl ParityReport {
    pub fn problems(&self) -> impl Iterator<Item = &ParityRow> {
        self.rows
            .iter()
            .filter(|row| !matches!(row.outcome, Outcome::Match | Outcome::SkippedNoData))
    }

    fn named_problems(rows: &[NamedParityRow]) -> impl Iterator<Item = &NamedParityRow> {
        rows.iter()
            .filter(|row| !matches!(row.outcome, Outcome::Match | Outcome::SkippedNoData))
    }

    pub fn market_figure_problems(&self) -> impl Iterator<Item = &NamedParityRow> {
        Self::named_problems(&self.market_figures)
    }

    pub fn deposit_problems(&self) -> impl Iterator<Item = &NamedParityRow> {
        Self::named_problems(&self.deposits)
    }

    pub fn dividend_problems(&self) -> impl Iterator<Item = &NamedParityRow> {
        Self::named_problems(&self.dividends)
    }

    pub fn mpf_problems(&self) -> impl Iterator<Item = &NamedParityRow> {
        Self::named_problems(&self.mpf)
    }

    pub fn problem_count(&self) -> usize {
        self.problems().count()
            + self.market_figure_problems().count()
            + self.deposit_problems().count()
            + self.dividend_problems().count()
            + self.mpf_problems().count()
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
    check_dividends(pool, data, &mut report).await?;
    check_mpf(pool, data, &mut report).await?;
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

    // The sheet's K column counts every J–O row, so parity uses the effective
    // amount (received else estimated), not the UI's received-only figure.
    let dividend_rows = sqlx::query(
        "SELECT d.stock_id, SUM(COALESCE(d.received_amount, d.estimated_amount)) AS effective \
         FROM dividends d JOIN stocks s ON s.id = d.stock_id \
         WHERE s.market = ? GROUP BY d.stock_id",
    )
    .bind(market.as_str())
    .fetch_all(pool)
    .await?;
    let mut effective_dividends: std::collections::HashMap<i64, f64> =
        std::collections::HashMap::with_capacity(dividend_rows.len());
    for row in &dividend_rows {
        use sqlx::Row;
        effective_dividends.insert(row.try_get("stock_id")?, row.try_get("effective")?);
    }

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

        // 港股's dividend-adjusted columns K/P/U/V (W is skipped: the sheet's
        // market value used live prices, the app uses manual 現價).
        if outcome == Outcome::Match && market == Market::Hk {
            let effective = effective_dividends
                .get(&computed.stock.id)
                .copied()
                .unwrap_or(0.0);
            let cost = computed.summary.total_buy_cost;
            let held = computed.summary.shares_held;
            let net_invested = cost - effective;
            let net_comparisons: [(&'static str, Option<f64>, Option<f64>); 4] = [
                ("累計派息", sheet.cumulative_dividends, Some(effective)),
                (
                    "累計派息%",
                    sheet.dividend_return,
                    (cost != 0.0).then(|| effective / cost),
                ),
                ("淨投入總本金", sheet.net_invested, Some(net_invested)),
                (
                    "淨攤薄單價",
                    sheet.net_diluted_price,
                    (held != 0.0).then(|| net_invested / held),
                ),
            ];
            for (field, sheet_value, computed_value) in net_comparisons {
                match (sheet_value, computed_value) {
                    (None, _) => continue,
                    (Some(sheet_value), computed_value) => {
                        sheet_values += 1;
                        let computed_value = computed_value.unwrap_or(f64::NAN);
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

    // The sheet's cached last-month/max cells versus the derived figures.
    // 上月 compares loosely both ways: real records may legitimately drift
    // from the sheet's stale cached rate once they supersede the seed. The
    // 最高 marks are floors — beating them is the expected outcome, so only
    // a shortfall beyond the loose tolerance counts as a difference.
    let cached = &data.market(market).cached;
    for (code, field, computed, sheet_value, floor) in [
        (
            "上月",
            "last month",
            summary.last_month.and_then(|figures| figures.percent),
            cached.last_month_percent,
            false,
        ),
        (
            "最高報酬率",
            "max Balance %",
            summary.max.and_then(|figures| figures.percent),
            cached.max_percent,
            true,
        ),
        (
            "最高金額",
            "max net",
            summary.max.map(|figures| figures.amount),
            cached.max_amount,
            true,
        ),
    ] {
        let Some(sheet_value) = sheet_value else {
            continue;
        };
        let computed_value = computed.unwrap_or(f64::NAN);
        let equal = if floor {
            floor_eq(computed_value, sheet_value)
        } else {
            loose_eq(computed_value, sheet_value)
        };
        named_row(
            &mut report.market_figures,
            format!("{} {code}", market.as_str()),
            if equal {
                Outcome::Match
            } else {
                Outcome::Difference {
                    field,
                    computed: computed_value,
                    sheet: sheet_value,
                }
            },
        );
    }

    Ok(())
}

fn named_row(rows: &mut Vec<NamedParityRow>, name: impl Into<String>, outcome: Outcome) {
    rows.push(NamedParityRow {
        name: name.into(),
        outcome,
    });
}

fn deposit_row(report: &mut ParityReport, name: impl Into<String>, outcome: Outcome) {
    named_row(&mut report.deposits, name, outcome);
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

/// Seeded last-month/max net gains are reconstructions (the sheet stores
/// rates only), so those two comparisons use a loose relative tolerance while
/// everything else matches the usual figures exactly.
const MPF_SEED_GAIN_TOLERANCE: f64 = 0.02;

fn loose_eq(computed: f64, sheet: f64) -> bool {
    (computed - sheet).abs() <= MPF_SEED_GAIN_TOLERANCE * sheet.abs().max(1.0)
}

/// Seeded 最高 marks are floors: meeting or beating the sheet's mark is the
/// expected outcome, so only a shortfall beyond the loose tolerance counts.
fn floor_eq(computed: f64, sheet: f64) -> bool {
    computed >= sheet - MPF_SEED_GAIN_TOLERANCE * sheet.abs().max(1.0)
}

fn mpf_row(report: &mut ParityReport, name: impl Into<String>, outcome: Outcome) {
    named_row(&mut report.mpf, name, outcome);
}

/// One named comparison: `sheet None` means nothing to compare (skipped);
/// `computed None` against a sheet value is a difference.
fn compare_named(
    report: &mut ParityReport,
    name: impl Into<String>,
    field: &'static str,
    computed: Option<f64>,
    sheet: Option<f64>,
    loose: bool,
) {
    let Some(sheet_value) = sheet else { return };
    let computed_value = computed.unwrap_or(f64::NAN);
    let equal = if loose {
        loose_eq(computed_value, sheet_value)
    } else {
        approx_eq(computed_value, sheet_value)
    };
    mpf_row(
        report,
        name,
        if equal {
            Outcome::Match
        } else {
            Outcome::Difference {
                field,
                computed: computed_value,
                sheet: sheet_value,
            }
        },
    );
}

/// MPF accounts and portfolio totals against the workbook's cached MPF sheet.
async fn check_mpf(
    pool: &SqlitePool,
    data: &WorkbookData,
    report: &mut ParityReport,
) -> anyhow::Result<()> {
    use crate::routes::mpf as mpf_routes;

    let today = crate::routes::today();
    let stored = mpf_routes::load_all_stored(pool)
        .await
        .map_err(|err| anyhow!("loading MPF accounts failed: {err}"))?;
    let history = mpf_routes::load_history(pool, None)
        .await
        .map_err(|err| anyhow!("loading MPF history failed: {err}"))?;
    let accounts: Vec<crate::models::MpfAccount> = stored
        .iter()
        .map(|account| mpf_routes::present_account(account, &history, today))
        .collect();
    let facts: Vec<crate::calc::MpfAccountFacts> = stored
        .iter()
        .map(|account| mpf_routes::facts_of(account, &history))
        .collect();
    let totals = crate::calc::mpf_totals(
        &facts,
        today,
        mpf_routes::meta_f64(pool, crate::mpf::SEED_MAX_RATE_KEY).await?,
        mpf_routes::meta_f64(pool, crate::mpf::SEED_MAX_GAIN_KEY).await?,
    );

    for sheet_account in &data.mpf {
        let name = format!("MPF {}", sheet_account.label);
        let Some(computed) = accounts
            .iter()
            .find(|account| account.label == sheet_account.label)
        else {
            mpf_row(report, name, Outcome::MissingStock);
            continue;
        };
        let mut outcome = Outcome::Match;
        for (field, sheet_value, computed_value) in [
            (
                "總供款額",
                sheet_account.contributions,
                Some(computed.contributions),
            ),
            ("帳戶結存", sheet_account.balance, Some(computed.balance)),
            ("回報率", sheet_account.rate, computed.rate),
            // The seeded row reproduces the sheet's rate exactly.
            (
                "last month",
                sheet_account.last_month_rate,
                computed.last_month.and_then(|figures| figures.rate),
            ),
        ] {
            let Some(sheet_value) = sheet_value else {
                continue;
            };
            let computed_value = computed_value.unwrap_or(f64::NAN);
            if !approx_eq(computed_value, sheet_value) {
                outcome = Outcome::Difference {
                    field,
                    computed: computed_value,
                    sheet: sheet_value,
                };
                break;
            }
        }
        mpf_row(report, name, outcome);
    }

    let cached = &data.mpf_cached;
    compare_named(
        report,
        "MPF buy",
        "總供款額",
        Some(totals.buy),
        cached.buy,
        false,
    );
    compare_named(
        report,
        "MPF now",
        "帳戶結存",
        Some(totals.now),
        cached.now,
        false,
    );
    compare_named(
        report,
        "MPF 回報率",
        "回報率",
        totals.rate,
        cached.rate,
        false,
    );
    compare_named(
        report,
        "MPF 淨收益",
        "gain",
        Some(totals.gain),
        cached.gain,
        false,
    );
    // The sheet's portfolio last-month rate is a hand-entered snapshot, not
    // a contribution-weighted figure, so it gets the loose comparison too.
    compare_named(
        report,
        "MPF last month rate",
        "last month",
        totals.last_month.and_then(|f| f.rate),
        cached.last_month_rate,
        true,
    );
    compare_named(
        report,
        "MPF last month gain",
        "last month",
        totals.last_month.map(|f| f.gain),
        cached.last_month_gain,
        true,
    );
    compare_named(
        report,
        "MPF max rate",
        "max",
        totals.max.rate,
        cached.max_rate,
        false,
    );
    compare_named(
        report,
        "MPF max gain",
        "max",
        Some(totals.max.gain),
        cached.max_gain,
        true,
    );

    Ok(())
}

/// The J–O 派息 block stores no aggregate, so the check compares the row
/// count and the total amount per market — enough to catch missed or
/// duplicated rows.
async fn check_dividends(
    pool: &SqlitePool,
    data: &WorkbookData,
    report: &mut ParityReport,
) -> anyhow::Result<()> {
    use sqlx::Row;
    for market in [Market::Hk, Market::Us] {
        let sheet = data.market(market);
        let sheet_count = sheet.dividends.len() as f64;
        let sheet_total: f64 = sheet.dividends.iter().map(|d| d.amount).sum();

        let row = sqlx::query(
            "SELECT COUNT(*), COALESCE(SUM(COALESCE(d.received_amount, d.estimated_amount)), 0.0) \
             FROM dividends d JOIN stocks s ON s.id = d.stock_id WHERE s.market = ?",
        )
        .bind(market.as_str())
        .fetch_one(pool)
        .await?;
        let count: i64 = row.try_get(0)?;
        let total: f64 = row.try_get(1)?;

        let name = format!("派息 {}", market.as_str());
        if (count as f64 - sheet_count).abs() > 0.5 {
            named_row(
                &mut report.dividends,
                format!("{name} count"),
                Outcome::Difference {
                    field: "rows",
                    computed: count as f64,
                    sheet: sheet_count,
                },
            );
        } else if !approx_eq(total, sheet_total) {
            named_row(
                &mut report.dividends,
                format!("{name} total"),
                Outcome::Difference {
                    field: "amount",
                    computed: total,
                    sheet: sheet_total,
                },
            );
        } else {
            named_row(&mut report.dividends, name, Outcome::Match);
        }
    }
    Ok(())
}
