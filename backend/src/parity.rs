//! Compares the app's computed summary against the workbook's cached values.

use anyhow::anyhow;
use chrono::Datelike;
use sqlx::{Row, SqlitePool};

use crate::calc::{
    active_month_rollup, active_totals, approx_eq, bank_rollup, live_totals, month_derived,
    month_item_sums, month_running_averages, month_year_summaries, year_rollups, DepositFacts,
    MonthItemFacts, MonthStatRow,
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
    /// A difference expected by design — live-linked or edited cells are
    /// reported but never counted as problems.
    Info {
        computed: f64,
        sheet: f64,
    },
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
    pub bonds: Vec<NamedParityRow>,
    pub aia: Vec<NamedParityRow>,
    pub months: Vec<NamedParityRow>,
    /// The `Overview` A3:C18 block and the 美股 IBKR header cells.
    pub overview: Vec<NamedParityRow>,
}

impl ParityReport {
    pub fn problems(&self) -> impl Iterator<Item = &ParityRow> {
        self.rows
            .iter()
            .filter(|row| !matches!(row.outcome, Outcome::Match | Outcome::SkippedNoData))
    }

    fn named_problems(rows: &[NamedParityRow]) -> impl Iterator<Item = &NamedParityRow> {
        rows.iter().filter(|row| {
            !matches!(
                row.outcome,
                Outcome::Match | Outcome::SkippedNoData | Outcome::Info { .. }
            )
        })
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

    pub fn bond_problems(&self) -> impl Iterator<Item = &NamedParityRow> {
        Self::named_problems(&self.bonds)
    }

    pub fn aia_problems(&self) -> impl Iterator<Item = &NamedParityRow> {
        Self::named_problems(&self.aia)
    }

    pub fn months_problems(&self) -> impl Iterator<Item = &NamedParityRow> {
        Self::named_problems(&self.months)
    }

    pub fn overview_problems(&self) -> impl Iterator<Item = &NamedParityRow> {
        Self::named_problems(&self.overview)
    }

    pub fn problem_count(&self) -> usize {
        self.problems().count()
            + self.market_figure_problems().count()
            + self.deposit_problems().count()
            + self.dividend_problems().count()
            + self.mpf_problems().count()
            + self.bond_problems().count()
            + self.aia_problems().count()
            + self.months_problems().count()
            + self.overview_problems().count()
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
    check_bonds(pool, data, &mut report).await?;
    check_aia(pool, data, &mut report).await?;
    check_months(pool, data, &mut report).await?;
    check_overview(pool, data, &mut report).await?;
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

fn bond_row(report: &mut ParityReport, name: impl Into<String>, outcome: Outcome) {
    named_row(&mut report.bonds, name, outcome);
}

/// Bond figures against the 債券 sheet's cached values: the `Total` cell
/// against Σ active principal, and each coupon's derived expected amount
/// against the cached interest column. 待定 cells carry no cached figure and
/// are skipped. Status is date-derived, so a bond that matured since the
/// workbook last recalculated legitimately shows a difference.
async fn check_bonds(
    pool: &SqlitePool,
    data: &WorkbookData,
    report: &mut ParityReport,
) -> anyhow::Result<()> {
    let bonds = crate::routes::bonds::load_all_bonds(pool)
        .await
        .map_err(|err| anyhow!("loading bonds failed: {err}"))?;

    let active_principal: f64 = bonds
        .iter()
        .filter(|bond| bond.status == crate::models::BondStatus::Active)
        .map(|bond| bond.principal)
        .sum();
    match data.bond_cached.total_principal {
        Some(sheet) if approx_eq(active_principal, sheet) => {
            bond_row(report, "債券 total principal", Outcome::Match)
        }
        Some(sheet) => bond_row(
            report,
            "債券 total principal",
            Outcome::Difference {
                field: "principal",
                computed: active_principal,
                sheet,
            },
        ),
        None => bond_row(
            report,
            "債券 total principal",
            Outcome::MissingSheetValue { field: "principal" },
        ),
    }

    for sheet_bond in &data.bonds {
        let computed_bond =
            bonds
                .iter()
                .find(|bond| match (&bond.issue_no, &sheet_bond.issue_no) {
                    (Some(stored), Some(sheet)) => stored == sheet,
                    _ => {
                        bond.label == sheet_bond.label
                            && approx_eq(bond.principal, sheet_bond.principal)
                            && bond.maturity_date == sheet_bond.maturity_date
                    }
                });
        let Some(computed_bond) = computed_bond else {
            bond_row(
                report,
                format!("債券 {}", sheet_bond.label),
                Outcome::MissingStock,
            );
            continue;
        };
        let coupons = crate::routes::bonds::load_bond_coupons(pool, computed_bond.id)
            .await
            .map_err(|err| anyhow!("loading coupons of bond {} failed: {err}", computed_bond.id))?;
        for sheet_coupon in &sheet_bond.coupons {
            let Some(sheet_interest) = sheet_coupon.interest else {
                continue; // 待定 rows carry no cached figure to compare
            };
            let name = format!("債券 {} {}", sheet_bond.label, sheet_coupon.pay_date);
            // The sheet's interest column is what the coupon paid/pays — the
            // effective amount (received else expected), like the dividend
            // check's COALESCE. The cached cell can disagree with 每1萬利息
            // (a hand-entered E wins), so comparing expected alone would flag
            // the sheet's own inconsistency as an app difference.
            let effective = coupons
                .iter()
                .find(|coupon| coupon.pay_date == sheet_coupon.pay_date)
                .and_then(|coupon| coupon.received_amount.or(coupon.expected));
            match effective {
                Some(effective) if approx_eq(effective, sheet_interest) => {
                    bond_row(report, name, Outcome::Match)
                }
                effective => bond_row(
                    report,
                    name,
                    Outcome::Difference {
                        field: "interest",
                        computed: effective.unwrap_or(f64::NAN),
                        sheet: sheet_interest,
                    },
                ),
            }
        }
    }
    Ok(())
}

fn aia_row(report: &mut ParityReport, name: impl Into<String>, outcome: Outcome) {
    named_row(&mut report.aia, name, outcome);
}

/// Compare two figures where either side may be absent: both present means a
/// real comparison; the sheet leaving a cell blank is its own omission and
/// skips quietly (e.g. `irene 20%` has no `balance %%` formula), while a
/// figure the app cannot produce against a cached cell still counts.
fn compare_figures(computed: Option<f64>, sheet: Option<f64>, field: &'static str) -> Outcome {
    match (computed, sheet) {
        (Some(computed), Some(sheet)) if approx_eq(computed, sheet) => Outcome::Match,
        (Some(computed), Some(sheet)) => Outcome::Difference {
            field,
            computed,
            sheet,
        },
        (None, Some(_)) => Outcome::MissingSheetValue { field },
        _ => Outcome::SkippedNoData,
    }
}

/// AIA figures against the sheet's cached cells: per-policy `buy usd`,
/// `now usd` and `balance %%` against each stored row; the summary block's
/// USD totals, `AIA display value` and HKD conversions against the derived
/// totals (HKD figures convert with the stored `aia.usd_hkd_rate`, seeded
/// from the same `Overview!N3` the sheet's HKD cells use, so the comparison
/// is meaningful on both sides).
async fn check_aia(
    pool: &SqlitePool,
    data: &WorkbookData,
    report: &mut ParityReport,
) -> anyhow::Result<()> {
    let policies = crate::routes::aia::load_all_policies(pool)
        .await
        .map_err(|err| anyhow!("loading aia policies failed: {err}"))?;
    let rate = crate::mpf::meta_get(pool, crate::routes::aia::RATE_KEY)
        .await?
        .and_then(|raw| raw.parse::<f64>().ok());
    let facts: Vec<crate::calc::AiaPolicyFacts> =
        policies.iter().map(crate::routes::aia::facts_of).collect();
    let totals = crate::calc::aia_totals(&facts, rate);
    let cached = &data.aia_cached;

    for (name, field, computed, sheet) in [
        (
            "AIA buy usd",
            "premium",
            Some(totals.premium),
            cached.buy_usd,
        ),
        ("AIA now usd", "value", Some(totals.value), cached.now_usd),
        (
            "AIA balance %%",
            "balance_pct",
            totals.balance_pct,
            cached.balance_pct,
        ),
        (
            "AIA display value",
            "value",
            Some(totals.display_value),
            cached.display_value,
        ),
        (
            "AIA buy hkd",
            "premium_hkd",
            totals.premium_hkd,
            cached.buy_hkd,
        ),
        ("AIA now hkd", "value_hkd", totals.value_hkd, cached.now_hkd),
        (
            "AIA drew hkd",
            "withdrew_hkd",
            totals.withdrew_hkd,
            cached.drew_hkd,
        ),
        (
            "AIA net change hkd",
            "net_change_hkd",
            totals.net_change_hkd,
            cached.net_change_hkd,
        ),
    ] {
        aia_row(report, name, compare_figures(computed, sheet, field));
    }

    for sheet_policy in &data.aia {
        let stored =
            policies.iter().find(
                |policy| match (&policy.policy_no, &sheet_policy.policy_no) {
                    (Some(stored), Some(sheet)) => stored == sheet,
                    _ => {
                        policy.label == sheet_policy.label
                            && approx_eq(policy.premium_usd, sheet_policy.premium_usd)
                            && approx_eq(policy.value_usd, sheet_policy.value_usd)
                    }
                },
            );
        let name = format!(
            "AIA {}",
            sheet_policy
                .policy_no
                .as_deref()
                .unwrap_or(&sheet_policy.label)
        );
        let Some(stored) = stored else {
            aia_row(report, name, Outcome::MissingStock);
            continue;
        };
        for (field, computed, sheet) in [
            (
                "premium",
                Some(stored.premium_usd),
                Some(sheet_policy.premium_usd),
            ),
            (
                "value",
                Some(stored.value_usd),
                Some(sheet_policy.value_usd),
            ),
            ("balance_pct", stored.balance_pct, sheet_policy.balance_pct),
        ] {
            aia_row(
                report,
                format!("{name} {field}"),
                compare_figures(computed, sheet, field),
            );
        }
    }
    Ok(())
}

fn months_row(report: &mut ParityReport, name: impl Into<String>, outcome: Outcome) {
    named_row(&mut report.months, name, outcome);
}

/// Compare a stored/derived month figure against the sheet's cached cell.
/// `sheet None` means nothing to compare; `informational` rows (live-linked
/// cells or rows edited after import) downgrade differences to `Info`.
fn month_compare(
    report: &mut ParityReport,
    name: impl Into<String>,
    field: &'static str,
    computed: Option<f64>,
    sheet: Option<f64>,
    informational: bool,
) {
    let Some(sheet_value) = sheet else { return };
    let computed_value = computed.unwrap_or(f64::NAN);
    let outcome = if approx_eq(computed_value, sheet_value) {
        Outcome::Match
    } else if informational {
        Outcome::Info {
            computed: computed_value,
            sheet: sheet_value,
        }
    } else {
        Outcome::Difference {
            field,
            computed: computed_value,
            sheet: sheet_value,
        }
    };
    months_row(report, name, outcome);
}

/// Month Stat rows, yearly block, running averages and the seeded Overview
/// settings against the sheet's cached cells. Derived figures recompute from
/// the stored rows with the same live-fill rule as the API; 投資純利 (K) is
/// skipped because HK sold P/L is not computed.
async fn check_months(
    pool: &SqlitePool,
    data: &WorkbookData,
    report: &mut ParityReport,
) -> anyhow::Result<()> {
    struct Stored {
        month: String,
        start_cash: Option<f64>,
        salary: Option<f64>,
        total_assets: Option<f64>,
        liquid_assets: Option<f64>,
        pool_input: f64,
        end_cash_override: Option<f64>,
        /// `updated_at != created_at`: touched after import.
        edited: bool,
    }
    let rows = sqlx::query(
        "SELECT month, start_cash, salary, total_assets, liquid_assets, \
         pool_input, end_cash_override, created_at, updated_at \
         FROM month_stats ORDER BY month",
    )
    .fetch_all(pool)
    .await?;
    let mut stored = Vec::with_capacity(rows.len());
    for row in &rows {
        stored.push(Stored {
            month: row.try_get("month")?,
            start_cash: row.try_get("start_cash")?,
            salary: row.try_get("salary")?,
            total_assets: row.try_get("total_assets")?,
            liquid_assets: row.try_get("liquid_assets")?,
            pool_input: row.try_get("pool_input")?,
            end_cash_override: row.try_get("end_cash_override")?,
            edited: row.try_get::<String, _>("updated_at")?
                != row.try_get::<String, _>("created_at")?,
        });
    }

    let item_rows =
        sqlx::query("SELECT month, category, amount, exclude_from_living FROM month_items")
            .fetch_all(pool)
            .await?;
    let mut items: std::collections::HashMap<chrono::NaiveDate, Vec<MonthItemFacts>> =
        std::collections::HashMap::new();
    for row in &item_rows {
        let month: String = row.try_get("month")?;
        let category: String = row.try_get("category")?;
        let Some(category) = crate::models::MonthItemCategory::parse(&category) else {
            return Err(anyhow!(
                "stored month item category {category} is not valid"
            ));
        };
        items
            .entry(
                chrono::NaiveDate::parse_from_str(&month, "%Y-%m-%d")
                    .map_err(|_| anyhow!("stored month {month} is not valid"))?,
            )
            .or_default()
            .push(MonthItemFacts {
                category,
                amount: row.try_get("amount")?,
                exclude_from_living: row.try_get::<i64, _>("exclude_from_living")? != 0,
            });
    }

    // Live totals fill NULL B/D at/after the current month — same rule as
    // the API — so the live-linked rows produce real numbers here too.
    let today = crate::routes::today();
    let live_from = chrono::NaiveDate::from_ymd_opt(today.year(), today.month(), 1)
        .ok_or_else(|| anyhow!("today {today} has no first of month"))?;
    let parse_month = |month: &str| -> anyhow::Result<chrono::NaiveDate> {
        chrono::NaiveDate::parse_from_str(month, "%Y-%m-%d")
            .map_err(|_| anyhow!("stored month {month} is not valid"))
    };
    let needs_live = stored.iter().any(|s| {
        (s.total_assets.is_none() || s.liquid_assets.is_none())
            && parse_month(&s.month).is_ok_and(|month| month >= live_from)
    });
    let live = if needs_live {
        let input = crate::routes::months::live_totals_input(pool).await?;
        Some(live_totals(&input))
    } else {
        None
    };

    // 利息 is derived: auto events + `interest` items — same rule as the API.
    let interest_events = crate::routes::months::load_interest_events(pool).await?;
    let mut stat_rows = Vec::with_capacity(stored.len());
    for s in &stored {
        let month = parse_month(&s.month)?;
        let live = live.filter(|_| month >= live_from);
        let manual_interest = items
            .get(&month)
            .map(|items| month_item_sums(items).interest)
            .unwrap_or_default();
        stat_rows.push(MonthStatRow {
            month,
            start_cash: s.start_cash,
            salary: s.salary,
            total_assets: s.total_assets.or(live.map(|live| live.total_assets)),
            liquid_assets: s.liquid_assets.or(live.map(|live| live.liquid_assets)),
            end_cash_override: s.end_cash_override,
            interest: crate::calc::auto_interest(month, &interest_events) + manual_interest,
            pool_input: s.pool_input,
        });
    }
    let derived = month_derived(&stat_rows, &items);

    let rate_rows =
        sqlx::query("SELECT key, value FROM app_meta WHERE key LIKE 'overview.pool_rate.%'")
            .fetch_all(pool)
            .await?;
    let mut rates = std::collections::BTreeMap::new();
    for row in &rate_rows {
        let key: String = row.try_get("key")?;
        let value: String = row.try_get("value")?;
        if let (Some(year), Ok(rate)) = (
            key.strip_prefix(crate::routes::months::POOL_RATE_PREFIX)
                .and_then(|year| year.parse::<i32>().ok()),
            value.parse::<f64>(),
        ) {
            rates.insert(year, rate);
        }
    }
    let years = month_year_summaries(&stat_rows, &items, &rates, &Default::default());
    let running = month_running_averages(&stat_rows, &items);

    // Years containing a live-linked or edited month flag informational:
    // their aggregates legitimately move with the live cells.
    let mut informational_years = std::collections::HashSet::new();
    let mut any_informational = false;

    // Per-row flags first: a row's derived cells also depend on the NEXT
    // row's totals, so a live-linked or edited successor marks it too.
    let mut linked = vec![false; stored.len()];
    let mut edited = vec![false; stored.len()];
    for sheet_month in &data.month_stat.months {
        if let Some(index) = stored.iter().position(|s| s.month == sheet_month.month) {
            linked[index] = (sheet_month.total_assets.is_none()
                && sheet_month.derived.total_assets.is_some())
                || (sheet_month.liquid_assets.is_none()
                    && sheet_month.derived.liquid_assets.is_some());
            edited[index] = stored[index].edited;
        }
    }

    for sheet_month in &data.month_stat.months {
        let Some(index) = stored.iter().position(|s| s.month == sheet_month.month) else {
            months_row(
                report,
                format!("Month {}", sheet_month.month),
                Outcome::MissingStock,
            );
            continue;
        };
        let stored_row = &stored[index];
        let own = linked[index] || edited[index];
        let derived_informational = own
            || linked.get(index + 1).copied().unwrap_or(false)
            || edited.get(index + 1).copied().unwrap_or(false);
        if derived_informational {
            any_informational = true;
            informational_years.insert(stat_rows[index].month.year());
        }
        let name = |field: &str| format!("Month {} {field}", sheet_month.month);
        month_compare(
            report,
            name("start_cash"),
            "start_cash",
            stored_row.start_cash,
            sheet_month.start_cash,
            own,
        );
        month_compare(
            report,
            name("total_assets"),
            "total_assets",
            stat_rows[index].total_assets,
            sheet_month.derived.total_assets,
            own,
        );
        month_compare(
            report,
            name("liquid_assets"),
            "liquid_assets",
            stat_rows[index].liquid_assets,
            sheet_month.derived.liquid_assets,
            own,
        );
        month_compare(
            report,
            name("interest"),
            "interest",
            Some(stat_rows[index].interest),
            Some(sheet_month.interest),
            own,
        );
        month_compare(
            report,
            name("entertainment"),
            "entertainment",
            Some(
                items
                    .get(&stat_rows[index].month)
                    .map(|items| month_item_sums(items).entertainment)
                    .unwrap_or_default(),
            ),
            Some(sheet_month.entertainment),
            own,
        );
        month_compare(
            report,
            name("pool_input"),
            "pool_input",
            Some(stored_row.pool_input),
            Some(sheet_month.pool_input),
            own,
        );
        month_compare(
            report,
            name("end_cash"),
            "end_cash",
            derived[index].end_cash,
            sheet_month.derived.end_cash,
            derived_informational || sheet_month.end_cash_frozen,
        );
        month_compare(
            report,
            name("month_spend"),
            "month_spend",
            derived[index].month_spend,
            sheet_month.derived.month_spend,
            derived_informational || sheet_month.end_cash_frozen,
        );
        month_compare(
            report,
            name("living_spend"),
            "living_spend",
            derived[index].living_spend,
            sheet_month.derived.living_spend,
            derived_informational || sheet_month.end_cash_frozen,
        );
        // K also depends on the same month a year earlier — a live-linked or
        // edited prior-year row makes the comparison informational too.
        let prior_informational = sheet_month
            .month
            .get(..4)
            .and_then(|y| y.parse::<i32>().ok())
            .map(|year| format!("{}{}", year - 1, &sheet_month.month[4..]))
            .and_then(|prior_month| stored.iter().position(|s| s.month == prior_month))
            .map(|prior| linked[prior] || edited[prior])
            .unwrap_or(false);
        month_compare(
            report,
            name("living_yoy"),
            "living_yoy",
            derived[index].living_yoy,
            sheet_month.derived.living_yoy,
            derived_informational || sheet_month.end_cash_frozen || prior_informational,
        );
        month_compare(
            report,
            name("saved"),
            "saved",
            derived[index].saved,
            sheet_month.derived.saved,
            derived_informational || sheet_month.end_cash_frozen,
        );
        month_compare(
            report,
            name("total_change"),
            "total_change",
            derived[index].total_change,
            sheet_month.derived.total_change,
            derived_informational,
        );
        month_compare(
            report,
            name("liquid_change"),
            "liquid_change",
            derived[index].liquid_change,
            sheet_month.derived.liquid_change,
            derived_informational,
        );
    }

    for sheet_year in &data.month_stat.years {
        let name = |field: &str| format!("Month Stat {} {field}", sheet_year.year);
        let informational = informational_years.contains(&sheet_year.year);
        let Some(computed) = years.iter().find(|year| year.year == sheet_year.year) else {
            months_row(
                report,
                format!("Month Stat {} year", sheet_year.year),
                Outcome::MissingStock,
            );
            continue;
        };
        for (field, computed, sheet) in [
            (
                "total_change_sum",
                computed.total_change_sum,
                sheet_year.total_change_sum,
            ),
            (
                "total_change_avg",
                computed.total_change_avg,
                sheet_year.total_change_avg,
            ),
            ("spend_sum", computed.spend_sum, sheet_year.spend_sum),
            ("spend_avg", computed.spend_avg, sheet_year.spend_avg),
            ("living_avg", computed.living_avg, sheet_year.living_avg),
            (
                "entertainment_sum",
                Some(computed.entertainment_sum),
                sheet_year.entertainment_sum,
            ),
            (
                "interest_sum",
                Some(computed.interest_sum),
                sheet_year.interest_sum,
            ),
            (
                "interest_avg",
                Some(computed.interest_avg),
                sheet_year.interest_avg,
            ),
            (
                "pool_balance",
                Some(computed.pool_balance),
                sheet_year.pool_balance,
            ),
            (
                "pool_input_sum",
                Some(computed.pool_input_sum),
                sheet_year.pool_input_sum,
            ),
        ] {
            month_compare(report, name(field), field, computed, sheet, informational);
        }
    }

    for (name, field, computed, sheet) in [
        (
            "Month Stat running total_change",
            "avg_total_change",
            running.total_change_avg,
            data.month_stat.avg_total_change,
        ),
        (
            "Month Stat running saved",
            "avg_saved",
            running.saved_avg,
            data.month_stat.avg_saved,
        ),
        (
            "Month Stat running interest",
            "avg_interest",
            running.interest_avg,
            data.month_stat.avg_interest,
        ),
    ] {
        month_compare(report, name, field, computed, sheet, any_informational);
    }

    // Settings: Overview!E1 salary, N8 current-year pool rate, and the four
    // manual asset/cash cells against the seeded rows.
    let salary_meta = crate::mpf::meta_get(pool, crate::routes::months::SALARY_KEY).await?;
    let rate_meta =
        crate::mpf::meta_get(pool, &crate::routes::months::pool_rate_key(today.year())).await?;
    month_compare(
        report,
        "Month Stat settings salary",
        "salary",
        salary_meta.and_then(|raw| raw.parse::<f64>().ok()),
        data.overview.salary,
        false,
    );
    month_compare(
        report,
        "Month Stat settings pool_rate",
        "pool_rate",
        rate_meta.and_then(|raw| raw.parse::<f64>().ok()),
        data.overview.pool_rate,
        false,
    );
    for asset in &data.overview.manual_assets {
        let stored: Option<f64> =
            sqlx::query_scalar("SELECT amount FROM manual_assets WHERE label = ? AND kind = ?")
                .bind(&asset.label)
                .bind(asset.kind.as_str())
                .fetch_optional(pool)
                .await?;
        month_compare(
            report,
            format!("Month Stat manual {}", asset.label),
            "amount",
            stored,
            Some(asset.amount),
            false,
        );
    }
    Ok(())
}

fn overview_row(report: &mut ParityReport, name: impl Into<String>, outcome: Outcome) {
    named_row(&mut report.overview, name, outcome);
}

/// One Overview comparison: blank sheet cells are skipped, deterministic
/// module figures (債券, 基金, MPF, 已定期) count as problems, and everything
/// downstream of live prices or user-edited manual inputs reports `Info`.
fn overview_compare(
    report: &mut ParityReport,
    name: impl Into<String>,
    field: &'static str,
    computed: Option<f64>,
    sheet: Option<f64>,
    deterministic: bool,
) {
    let Some(sheet_value) = sheet else { return };
    let computed_value = computed.unwrap_or(f64::NAN);
    let outcome = if approx_eq(computed_value, sheet_value) {
        Outcome::Match
    } else if deterministic {
        Outcome::Difference {
            field,
            computed: computed_value,
            sheet: sheet_value,
        }
    } else {
        Outcome::Info {
            computed: computed_value,
            sheet: sheet_value,
        }
    };
    overview_row(report, name, outcome);
}

/// `Overview!A3:C18` plus the B1/H1/J1 headline and the 美股 IBKR header
/// cells against the derived dashboard.
async fn check_overview(
    pool: &SqlitePool,
    data: &WorkbookData,
    report: &mut ParityReport,
) -> anyhow::Result<()> {
    let cached = &data.overview;
    if cached.assets.is_empty() && cached.total_assets.is_none() {
        return Ok(());
    }
    let response =
        crate::routes::overview::overview(axum::extract::State(crate::routes::AppState {
            pool: pool.clone(),
        }))
        .await
        .map_err(|err| anyhow!("computing the overview failed: {err}"))?
        .0;

    // Asset rows compare by label; module rows (債券/基金/MPF) are
    // deterministic, price-driven and manual rows report Info.
    let mut computed_assets: std::collections::HashMap<String, (Option<f64>, Option<f64>, bool)> =
        std::collections::HashMap::new();
    for row in &response.assets {
        let deterministic = matches!(row.key.as_str(), "bonds" | "aia" | "mpf");
        computed_assets.insert(
            row.label.trim().to_string(),
            (row.amount, row.share, deterministic),
        );
    }
    for row in &cached.assets {
        let label = row.label.trim();
        let (amount, share, deterministic) = computed_assets
            .get(label)
            .copied()
            .unwrap_or((None, None, false));
        overview_compare(
            report,
            format!("Overview {label}"),
            "B",
            amount,
            row.amount,
            deterministic,
        );
        overview_compare(
            report,
            format!("Overview {label} share"),
            "C",
            share,
            row.share,
            false,
        );
    }

    overview_compare(
        report,
        "Overview Sum",
        "B10",
        Some(response.assets_sum),
        cached.assets_sum,
        false,
    );
    overview_compare(
        report,
        "Overview 半流動 share",
        "A13",
        response.semi_liquid.share,
        cached.semi_liquid_share,
        false,
    );
    overview_compare(
        report,
        "Overview 半流動資金",
        "B14",
        Some(response.semi_liquid.total),
        cached.semi_liquid_total,
        false,
    );
    overview_compare(
        report,
        "Overview 已定期",
        "B15",
        Some(response.semi_liquid.deposits),
        cached.deposits_active,
        true,
    );
    overview_compare(
        report,
        "Overview 活期",
        "B18",
        Some(response.semi_liquid.cash_sum),
        cached.cash_total,
        false,
    );
    overview_compare(
        report,
        "Overview 半流動 vs 25%流動",
        "C14",
        Some(response.semi_liquid.vs_quarter_liquid),
        cached.semi_liquid_vs_quarter,
        false,
    );
    overview_compare(
        report,
        "Overview 總數",
        "B1",
        Some(response.total_assets),
        cached.total_assets,
        false,
    );
    overview_compare(
        report,
        "Overview 流動資產",
        "H1",
        Some(response.liquid_assets),
        cached.liquid_assets,
        false,
    );
    overview_compare(
        report,
        "Overview J1",
        "J1",
        response.liquid_ratio,
        cached.liquid_ratio,
        false,
    );

    // The cached cash rows (B16/B17) are manual cells — informational.
    let manual = crate::routes::months::load_assets(pool).await?;
    for asset in &cached.manual_assets {
        if asset.kind != crate::models::ManualAssetKind::Cash {
            continue;
        }
        let stored = manual
            .iter()
            .find(|m| m.label.trim() == asset.label.trim() && m.kind == asset.kind)
            .map(|m| m.amount);
        overview_compare(
            report,
            format!("Overview {}", asset.label),
            "B",
            stored,
            Some(asset.amount),
            false,
        );
    }

    let account = &data.us_account;
    overview_compare(
        report,
        "美股 IBKR 累計轉入",
        "B1",
        response.ibkr.transferred_hkd,
        account.transferred_hkd,
        false,
    );
    overview_compare(
        report,
        "美股 IBKR now value",
        "B2",
        response.ibkr.now_value,
        account.now_value,
        false,
    );
    overview_compare(
        report,
        "美股 IBKR HKD cash",
        "B4",
        response.ibkr.hkd_cash,
        account.hkd_cash,
        false,
    );
    overview_compare(
        report,
        "美股 IBKR USD cash",
        "B5",
        response.ibkr.usd_cash,
        account.usd_cash,
        false,
    );
    overview_compare(
        report,
        "美股 IBKR manual cal now",
        "B7",
        response.ibkr.computed_total_hkd,
        account.computed_now,
        false,
    );

    // F3:G10 + H6 — all informational: the sheet's OFFSET window is anchored
    // by hand, month rows are user-edited, and G10 is the live pool balance.
    let averages = &response.averages;
    for (name, field, computed, sheet) in [
        (
            "Overview 平均總數增加",
            "G4",
            averages.total_change,
            cached.avg_total_change,
        ),
        (
            "Overview 平均支出",
            "G5",
            averages.month_spend,
            cached.avg_month_spend,
        ),
        (
            "Overview 平均生活支出",
            "G6",
            averages.living_spend,
            cached.avg_living_spend,
        ),
        (
            "Overview 生活預算",
            "H6",
            averages.living_budget,
            cached.living_budget,
        ),
        ("Overview 平均存", "G7", averages.saved, cached.avg_saved),
        (
            "Overview 平均利息",
            "G8",
            averages.interest,
            cached.avg_interest,
        ),
        (
            "Overview 開心Pool",
            "G10",
            Some(averages.pool_balance),
            cached.pool_balance,
        ),
    ] {
        overview_compare(report, name, field, computed, sheet, false);
    }
    Ok(())
}
