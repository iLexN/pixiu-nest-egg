use super::*;

use std::collections::{BTreeMap, HashMap};

use chrono::Datelike;
use serde::Serialize;
use utoipa::ToSchema;

// ----- Year in review (YearInReview) -----

/// The stored `year_review` row: the figures the sheet enters by hand, plus
/// nullable overrides for cells whose history was deleted from the workbook
/// (pre-app bonds and deposits). A None override derives live.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, ToSchema)]
pub struct YearReviewRecord {
    /// 收入: the year's income, entered by hand (the sheet's own cell is a
    /// hand-built formula).
    pub income: Option<f64>,
    /// Added to the year's invested on top of the year's `ibkr_transfers` sum
    /// — the sheet's `invested` cell is `港股!C<y> ± manual amounts` (e.g.
    /// `− 110000 + 美股!B1` for 2026, where the 美股!B1 part now derives from
    /// the transfer log).
    pub invested_adjustment: Option<f64>,
    /// 月薪增幅 override for the Overview 投資目標 formula (the `salary_raise`
    /// column — `raise` is a SQLite keyword); None derives from the stored
    /// month salaries (last of the year minus last of the prior).
    pub raise: Option<f64>,
    /// Overrides for the 債券 row; None derives from coupons/bonds.
    pub bond_principal: Option<f64>,
    pub bond_interest: Option<f64>,
    /// Overrides for the 定期 row; None derives from deposits ending in the
    /// year.
    pub deposit_principal: Option<f64>,
    pub deposit_interest: Option<f64>,
}

/// The only bond facts the year review needs.
#[derive(Debug, Clone)]
pub struct BondYearFacts {
    pub principal: f64,
    /// The first coupon's pay year — the purchase proxy (bonds carry no
    /// start date). Falls back to the maturity year.
    pub first_coupon_year: Option<i32>,
    pub maturity_date: chrono::NaiveDate,
    /// (pay year, received_amount) of the received coupons.
    pub received: Vec<(i32, f64)>,
}

/// The sheet's A–D ledger group for one year: the Month Stat yearly block
/// re-averaged the sheet's way (`=C/12` flat, not AVERAGE over the months
/// present) plus the 開心Pool trio and the D-column YoY deltas.
#[derive(Debug, Clone, Default, PartialEq, Serialize, ToSchema)]
pub struct YearReviewLedger {
    /// 總數+: Σ total_change.
    pub asset_gain: Option<f64>,
    /// 平均總數+: asset_gain ÷ 12.
    pub asset_gain_avg: Option<f64>,
    /// 支出: Σ month_spend.
    pub spend: Option<f64>,
    /// 平均支出: spend ÷ 12.
    pub spend_avg: Option<f64>,
    /// 生活平均支出: mean living_spend.
    pub living_avg: Option<f64>,
    /// 開心 Pool 收入: Σ interest × the year's pool rate.
    pub pool_income: f64,
    /// 開心 Pool 支出: Σ entertainment items.
    pub pool_spend: f64,
    /// 開心 Pool 結餘: the chained closing balance.
    pub pool_balance: f64,
    /// The D-column YoY deltas; absent without a usable prior row.
    pub asset_gain_yoy: Option<f64>,
    pub spend_yoy: Option<f64>,
    pub living_yoy: Option<f64>,
    pub pool_income_yoy: Option<f64>,
}

/// The sheet's E–G investment group for one year.
#[derive(Debug, Clone, Default, PartialEq, Serialize, ToSchema)]
pub struct YearReviewInvestment {
    /// 利息回報: Σ derived month interest.
    pub interest: f64,
    /// 平均回報: interest ÷ 12 flat (the sheet's `=F/12`).
    pub interest_avg: f64,
    /// 投資P/L: the year's stored HK sold_pl; absent while none is stored.
    pub sold_pl: Option<f64>,
    /// 投資純利: interest + sold_pl; absent without sold_pl.
    pub net_investment: Option<f64>,
    /// IBKR 轉入: Σ the year's `ibkr_transfers`; absent while none.
    pub transferred: Option<f64>,
    /// invested: HK net invested + the year's IBKR 轉入 + invested_adjustment;
    /// absent while none of the three exists.
    pub invested: Option<f64>,
    /// invested %: invested ÷ (income + interest); absent without income.
    pub invested_pct: Option<f64>,
    /// Irene + 開心 Pool: Σ pool_input.
    pub irene_pool: f64,
    /// 月薪增幅: the stored `raise` override, else the salary-derived figure;
    /// absent while neither exists. Feeds the Overview 投資目標 target.
    pub raise: Option<f64>,
    /// The G-column YoY deltas on 平均回報 and invested.
    pub interest_avg_yoy: Option<f64>,
    pub invested_yoy: Option<f64>,
}

/// The sheet's H–M asset-returns group for one year.
#[derive(Debug, Clone, Default, PartialEq, Serialize, ToSchema)]
pub struct YearReviewAssets {
    /// 債券: principal held in the year and coupon interest received — the
    /// stored overrides win where set. Absent while neither derives nor an
    /// override exists.
    pub bond_principal: Option<f64>,
    pub bond_interest: Option<f64>,
    /// 債券 interest ÷ principal.
    pub bond_rate: Option<f64>,
    /// 股票 (HK only, matching the sheet — US enters only through the
    /// invested adjustment): year-end 成本 / 當年派息 / 總市值.
    pub stock_cost: Option<f64>,
    pub stock_dividends: Option<f64>,
    /// dividends ÷ cost.
    pub stock_rate: Option<f64>,
    pub stock_now_value: Option<f64>,
    /// dividends ÷ now_value.
    pub stock_value_rate: Option<f64>,
    /// 定期: principal and interest of deposits whose end_date is in the
    /// year and not after today (the sheet's `End` filter) — the stored
    /// overrides win where set.
    pub deposit_principal: Option<f64>,
    pub deposit_interest: Option<f64>,
    /// 回報率 blends (the 2026 layout):
    /// income ÷ (bond principal + stock cost).
    pub income_cost_rate: Option<f64>,
    /// all returns ÷ (bond principal + stock now value).
    pub total_value_rate: Option<f64>,
    /// (bond + stock returns) ÷ (bond principal + stock now value).
    pub income_value_rate: Option<f64>,
    /// 收入: the stored manual figure.
    pub income: Option<f64>,
    /// 收入 ÷ 12.
    pub income_avg: Option<f64>,
    /// The L-column YoY on 收入.
    pub income_yoy: Option<f64>,
    /// 存: income − spend.
    pub saved: Option<f64>,
    /// 存 ÷ 12.
    pub saved_avg: Option<f64>,
    /// 存 %: saved ÷ income.
    pub saved_pct: Option<f64>,
    /// True when a stored override replaced the derived figure.
    pub bond_overridden: bool,
    pub deposit_overridden: bool,
}

/// One row of the 年結 → 回顧 page: the YearInReview sheet's three groups for
/// one year, plus the stored record behind the manual/overridden cells.
#[derive(Debug, Clone, Default, PartialEq, Serialize, ToSchema)]
pub struct YearReviewRow {
    pub year: i32,
    pub ledger: YearReviewLedger,
    pub investment: YearReviewInvestment,
    pub assets: YearReviewAssets,
    /// The stored `year_review` record, when the year has one — the UI marks
    /// overridden cells from its non-null fields.
    pub record: Option<YearReviewRecord>,
}

fn yoy(current: Option<f64>, prior: Option<f64>) -> Option<f64> {
    match (current, prior) {
        (Some(current), Some(prior)) if prior != 0.0 => Some((current - prior) / prior),
        _ => None,
    }
}

/// The bond principal held in `year`: Σ principal of bonds with
/// `min(first coupon year, maturity year) ≤ year ≤ maturity year` — the
/// coupon schedule's start approximates the purchase a missing start date
/// would record. Returns None when no bond qualifies.
fn bond_year_principal(bonds: &[BondYearFacts], year: i32) -> Option<f64> {
    let principal: f64 = bonds
        .iter()
        .filter(|bond| {
            let maturity_year = bond.maturity_date.year();
            let start_year = bond
                .first_coupon_year
                .unwrap_or(maturity_year)
                .min(maturity_year);
            start_year <= year && year <= maturity_year
        })
        .map(|bond| bond.principal)
        .sum();
    (principal != 0.0).then_some(principal)
}

/// Σ received coupon amounts whose pay_date falls in `year`.
fn bond_year_interest(bonds: &[BondYearFacts], year: i32) -> Option<f64> {
    let interest: f64 = bonds
        .iter()
        .flat_map(|bond| bond.received.iter())
        .filter(|(pay_year, _)| *pay_year == year)
        .map(|(_, amount)| *amount)
        .sum();
    (interest != 0.0).then_some(interest)
}

/// Σ principal / Σ interest of deposits whose end_date is in `year` and not
/// after `today` — the sheet's `End` status filter. None when nothing ends.
fn deposit_year_figures(
    deposits: &[DepositFacts<'_>],
    year: i32,
    today: chrono::NaiveDate,
) -> (Option<f64>, Option<f64>) {
    let mut principal = 0.0;
    let mut interest = 0.0;
    let mut any = false;
    for deposit in deposits
        .iter()
        .filter(|deposit| deposit.end_date.year() == year && deposit.end_date <= today)
    {
        any = true;
        principal += deposit.principal.unwrap_or(0.0);
        interest += deposit.interest.unwrap_or(0.0);
    }
    (any.then_some(principal), any.then_some(interest))
}

/// The inputs `year_review_rows` derives against: `summaries` and `hk_years`
/// are the already-built Month Stat yearly block and HK yearly table;
/// `bonds`/`deposits` derive the asset rows that stored overrides can
/// replace; `transfers` and `last_salaries` feed `invested` and 月薪增幅.
#[derive(Debug)]
pub struct YearReviewInputs<'a> {
    pub summaries: &'a [MonthYearSummary],
    pub hk_years: &'a [YearRow],
    pub records: &'a BTreeMap<i32, YearReviewRecord>,
    pub bonds: &'a [BondYearFacts],
    pub deposits: &'a [DepositFacts<'a>],
    pub transfers: &'a BTreeMap<i32, f64>,
    pub last_salaries: &'a BTreeMap<i32, f64>,
}

/// Assemble the YearInReview blocks: one row per year that has a month
/// summary or a stored `year_review` record, ascending.
pub fn year_review_rows(
    inputs: &YearReviewInputs<'_>,
    today: chrono::NaiveDate,
) -> Vec<YearReviewRow> {
    let YearReviewInputs {
        summaries,
        hk_years,
        records,
        bonds,
        deposits,
        transfers,
        last_salaries,
    } = *inputs;
    let years: BTreeMap<i32, ()> = summaries
        .iter()
        .map(|summary| summary.year)
        .chain(records.keys().copied())
        .map(|year| (year, ()))
        .collect();

    // The fields a row's YoY deltas read off the previous row.
    struct Prior {
        asset_gain: Option<f64>,
        spend: Option<f64>,
        living: Option<f64>,
        pool_income: Option<f64>,
        interest_avg: Option<f64>,
        invested: Option<f64>,
        income: Option<f64>,
    }

    let mut rows = Vec::with_capacity(years.len());
    let mut prior: Option<Prior> = None;
    for &year in years.keys() {
        let summary = summaries.iter().find(|summary| summary.year == year);
        let record = records.get(&year).copied().unwrap_or_default();
        let stored_record = records.get(&year).copied();
        let hk = hk_years.iter().find(|row| row.year == year);

        let income = record.income;
        let interest = summary.map(|s| s.interest_sum).unwrap_or(0.0);
        let interest_avg = interest / 12.0;
        let sold_pl = hk.and_then(|row| row.sold_pl);
        let transferred = transfers.get(&year).copied();
        let invested = match (
            hk.map(|row| row.invested),
            transferred,
            record.invested_adjustment,
        ) {
            (None, None, None) => None,
            (base, transfer, adjustment) => {
                Some(base.unwrap_or(0.0) + transfer.unwrap_or(0.0) + adjustment.unwrap_or(0.0))
            }
        };
        let spend = summary.and_then(|s| s.spend_sum);
        let saved = match (income, spend) {
            (Some(income), Some(spend)) => Some(income - spend),
            _ => None,
        };
        // 月薪增幅: the stored override wins; else the step between the last
        // stored salary of each year. Absent while either side is missing.
        let raise = record.raise.or_else(|| {
            match (last_salaries.get(&year), last_salaries.get(&(year - 1))) {
                (Some(current), Some(prior)) => Some((current - prior).max(0.0)),
                _ => None,
            }
        });

        let bond_overridden = record.bond_principal.is_some() || record.bond_interest.is_some();
        let deposit_overridden =
            record.deposit_principal.is_some() || record.deposit_interest.is_some();
        let bond_principal = record
            .bond_principal
            .or_else(|| bond_year_principal(bonds, year));
        let bond_interest = record
            .bond_interest
            .or_else(|| bond_year_interest(bonds, year));
        let (derived_principal, derived_interest) = deposit_year_figures(deposits, year, today);
        let deposit_principal = record.deposit_principal.or(derived_principal);
        let deposit_interest = record.deposit_interest.or(derived_interest);

        let bond_rate = match (bond_interest, bond_principal) {
            (Some(interest), Some(principal)) if principal > 0.0 => Some(interest / principal),
            _ => None,
        };
        let stock_cost = hk.map(|row| row.cost);
        let stock_dividends = hk.map(|row| row.dividends);
        let stock_now_value = hk.and_then(|row| row.market_value);
        let stock_rate = match (stock_dividends, stock_cost) {
            (Some(dividends), Some(cost)) if cost > 0.0 => Some(dividends / cost),
            _ => None,
        };
        let stock_value_rate = match (stock_dividends, stock_now_value) {
            (Some(dividends), Some(value)) if value > 0.0 => Some(dividends / value),
            _ => None,
        };

        // SUM ranges treat blank cells as zero, so each blend counts whichever
        // components exist and reports a rate once its denominator is known.
        let income_returns = bond_interest.unwrap_or(0.0) + stock_dividends.unwrap_or(0.0);
        let all_returns = income_returns + deposit_interest.unwrap_or(0.0);
        let cost_basis = (bond_principal.unwrap_or(0.0) > 0.0 || stock_cost.is_some())
            .then(|| bond_principal.unwrap_or(0.0) + stock_cost.unwrap_or(0.0));
        let value_basis = match (bond_principal, stock_now_value) {
            (bond, stock) if bond.unwrap_or(0.0) + stock.unwrap_or(0.0) > 0.0 => {
                Some(bond.unwrap_or(0.0) + stock.unwrap_or(0.0))
            }
            _ => None,
        };
        let income_cost_rate = cost_basis.map(|basis| income_returns / basis);
        let total_value_rate = value_basis.map(|basis| all_returns / basis);
        let income_value_rate = value_basis.map(|basis| income_returns / basis);

        let row = YearReviewRow {
            year,
            ledger: YearReviewLedger {
                asset_gain: summary.and_then(|s| s.total_change_sum),
                asset_gain_avg: summary.and_then(|s| s.total_change_sum.map(|v| v / 12.0)),
                spend,
                spend_avg: spend.map(|v| v / 12.0),
                living_avg: summary.and_then(|s| s.living_avg),
                pool_income: summary.map(|s| s.pool_income).unwrap_or(0.0),
                pool_spend: summary.map(|s| s.entertainment_sum).unwrap_or(0.0),
                pool_balance: summary.map(|s| s.pool_balance).unwrap_or(0.0),
                asset_gain_yoy: yoy(
                    summary.and_then(|s| s.total_change_sum),
                    prior.as_ref().and_then(|p| p.asset_gain),
                ),
                spend_yoy: yoy(spend, prior.as_ref().and_then(|p| p.spend)),
                living_yoy: yoy(
                    summary.and_then(|s| s.living_avg),
                    prior.as_ref().and_then(|p| p.living),
                ),
                pool_income_yoy: yoy(
                    summary.map(|s| s.pool_income),
                    prior.as_ref().and_then(|p| p.pool_income),
                ),
            },
            investment: YearReviewInvestment {
                interest,
                interest_avg,
                sold_pl,
                net_investment: sold_pl.map(|sold_pl| interest + sold_pl),
                transferred,
                invested,
                invested_pct: match (invested, income) {
                    (Some(invested), Some(income)) if income + interest != 0.0 => {
                        Some(invested / (income + interest))
                    }
                    _ => None,
                },
                irene_pool: summary.map(|s| s.pool_input_sum).unwrap_or(0.0),
                raise,
                interest_avg_yoy: yoy(
                    summary.map(|_| interest_avg),
                    prior.as_ref().and_then(|p| p.interest_avg),
                ),
                invested_yoy: yoy(invested, prior.as_ref().and_then(|p| p.invested)),
            },
            assets: YearReviewAssets {
                bond_principal,
                bond_interest,
                bond_rate,
                stock_cost,
                stock_dividends,
                stock_rate,
                stock_now_value,
                stock_value_rate,
                deposit_principal,
                deposit_interest,
                income_cost_rate,
                total_value_rate,
                income_value_rate,
                income,
                income_avg: income.map(|v| v / 12.0),
                income_yoy: yoy(income, prior.as_ref().and_then(|p| p.income)),
                saved,
                saved_avg: saved.map(|v| v / 12.0),
                saved_pct: match (saved, income) {
                    (Some(saved), Some(income)) if income != 0.0 => Some(saved / income),
                    _ => None,
                },
                bond_overridden,
                deposit_overridden,
            },
            record: stored_record,
        };
        prior = Some(Prior {
            asset_gain: row.ledger.asset_gain,
            spend: row.ledger.spend,
            living: row.ledger.living_avg,
            pool_income: summary.map(|_| row.ledger.pool_income),
            interest_avg: summary.map(|_| row.investment.interest_avg),
            invested: row.investment.invested,
            income: row.assets.income,
        });
        rows.push(row);
    }
    rows
}

/// The Overview J22:N27 投資目標 block, projected from the year-review rows:
///
/// `target(Y) = invested(Y-1) − interest(Y-1) − pool_spend(Y-1)×0.7
///            + raise(Y)×0.5×12 + interest(Y) + pool_spend(Y)×0.7`
///
/// — last year's organic invested (with its embedded entertainment match
/// stripped), plus half the year's salary raise annualized, plus this year's
/// returns reinvested, plus 70% of the YoY increase in entertainment spend.
/// `avg_invested` is the sheet's J22: the mean of `invested` over the last
/// three completed years.
pub fn invest_targets(rows: &[YearReviewRow], current_year: i32) -> crate::models::InvestTargets {
    use crate::models::{InvestTargetRow, InvestTargets};
    use std::collections::HashMap;

    let by_year: HashMap<i32, &YearReviewRow> = rows.iter().map(|row| (row.year, row)).collect();
    let mut target_rows = Vec::with_capacity(rows.len());
    for row in rows {
        let prior = by_year.get(&(row.year - 1)).copied();
        let prior_invested = prior.and_then(|prior| prior.investment.invested);
        // The prior row's interest/pool_spend are already f64s (0 when the
        // year has no months); an absent raise counts as 0.
        let target = prior_invested.map(|prior_invested| {
            prior_invested
                - prior.map(|prior| prior.investment.interest).unwrap_or(0.0)
                - prior.map(|prior| prior.ledger.pool_spend).unwrap_or(0.0) * 0.7
                + row.investment.raise.unwrap_or(0.0) * 0.5 * 12.0
                + row.investment.interest
                + row.ledger.pool_spend * 0.7
        });
        let invested = row.investment.invested;
        let remain = if row.year == current_year {
            target.and_then(|target| invested.map(|invested| target - invested))
        } else {
            None
        };
        // N column: completed years grow on actual invested; the current (or
        // any later) year measures its target against last year's invested.
        let growth = if row.year < current_year {
            yoy(invested, prior_invested)
        } else {
            yoy(target, prior_invested)
        };
        target_rows.push(InvestTargetRow {
            year: row.year,
            invested,
            raise: row.investment.raise,
            interest: row.investment.interest,
            pool_spend: row.ledger.pool_spend,
            target,
            remain,
            growth,
        });
    }

    // J22: AVERAGE over the three most recent completed years' K cells,
    // skipping rows with no invested (AVERAGE semantics).
    let values: Vec<f64> = rows
        .iter()
        .filter(|row| row.year < current_year)
        .rev()
        .take(3)
        .filter_map(|row| row.investment.invested)
        .collect();
    let avg_invested =
        (!values.is_empty()).then(|| values.iter().sum::<f64>() / values.len() as f64);

    InvestTargets {
        avg_invested,
        rows: target_rows,
    }
}
/// completed month rows (`month < current_month`), per column skipping months
/// with no value, matching the sheet's hand-anchored OFFSET window.
#[derive(Debug, Clone, Default)]
pub struct TrailingAverages {
    /// G4 總數增加 (C Changed).
    pub total_change: Option<f64>,
    /// G5 支出 (I 月支出).
    pub month_spend: Option<f64>,
    /// G6 生活支出 (J).
    pub living_spend: Option<f64>,
    /// G7 存 (L).
    pub saved: Option<f64>,
    /// G8 利息 (N).
    pub interest: Option<f64>,
    /// H6 生活預算 = ROUNDUP(living_spend × 1.05, −2) — away from zero to the
    /// nearest 100. Absent while the window has no living-spend values.
    pub living_budget: Option<f64>,
    /// Window bounds: the earliest and latest month rows in the window.
    pub window_start: Option<chrono::NaiveDate>,
    pub window_end: Option<chrono::NaiveDate>,
}

pub fn trailing_averages(
    rows: &[MonthStatRow],
    items: &HashMap<chrono::NaiveDate, Vec<MonthItemFacts>>,
    current_month: chrono::NaiveDate,
) -> TrailingAverages {
    // Derive over the full row set first so Changed/end_cash see the next row.
    let derived = month_derived(rows, items);
    let mut window: Vec<(&MonthStatRow, &MonthDerived)> = rows
        .iter()
        .zip(&derived)
        .filter(|(row, _)| row.month < current_month)
        .collect();
    window.sort_by_key(|a| std::cmp::Reverse(a.0.month));
    window.truncate(12);

    let avg = |values: Vec<f64>| {
        (!values.is_empty()).then(|| values.iter().sum::<f64>() / values.len() as f64)
    };
    let living_spend = avg(window
        .iter()
        .filter_map(|(_, derived)| derived.living_spend)
        .collect());
    TrailingAverages {
        total_change: avg(window
            .iter()
            .filter_map(|(_, derived)| derived.total_change)
            .collect()),
        month_spend: avg(window
            .iter()
            .filter_map(|(_, derived)| derived.month_spend)
            .collect()),
        living_spend,
        saved: avg(window
            .iter()
            .filter_map(|(_, derived)| derived.saved)
            .collect()),
        interest: avg(window
            .iter()
            .filter(|(row, _)| row.start_cash.is_some() || row.interest != 0.0)
            .map(|(row, _)| row.interest)
            .collect()),
        living_budget: living_spend.map(|avg| (avg * 1.05 / 100.0).ceil() * 100.0),
        window_start: window.last().map(|(row, _)| row.month),
        window_end: window.first().map(|(row, _)| row.month),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calc::test_helpers::*;
    use crate::models::MonthItemCategory;

    #[test]
    fn year_review_combines_summaries_yearly_rows_and_overrides() {
        // 2025-12 → 2026-02: totals step by 60; January spends 400.
        let mut rows = vec![
            month_row("2025-12-01", Some(1000.0), Some(100.0)),
            month_row("2026-01-01", Some(1100.0), Some(100.0)),
            month_row("2026-02-01", Some(800.0), Some(100.0)),
        ];
        rows[0].total_assets = Some(500.0);
        rows[1].total_assets = Some(560.0);
        rows[2].total_assets = Some(620.0);
        rows[1].interest = 120.0;
        rows[2].interest = 240.0;
        let mut items = HashMap::new();
        items.insert(
            ym("2026-01-01"),
            vec![item(MonthItemCategory::Entertainment, 60.0)],
        );
        let rates = BTreeMap::from([(2026, 0.5)]);
        let summaries = month_year_summaries(&rows, &items, &rates, &BTreeMap::new(), None);

        // HK: 1000 bought in 2025, 500 in 2026; a 2025 snapshot freezes cost
        // 900 / value 2000 / sold_pl −40; a 2026 snapshot stores sold_pl 0.
        let trades = [
            dated_buy("2025-06-01", 100.0, 1000.0),
            dated_buy("2026-03-01", 50.0, 500.0),
        ];
        let dividends = [dividend_fact("中國銀行", "2026-08-01", Some(30.0))];
        let snapshots = HashMap::from([
            (
                2025,
                YearSnapshot {
                    sold_pl: Some(-40.0),
                    ..snapshot_year(None, Some(900.0), Some(2000.0))
                },
            ),
            (
                2026,
                YearSnapshot {
                    sold_pl: Some(0.0),
                    ..snapshot_year(None, None, None)
                },
            ),
        ]);
        let hk_years = yearly_rows(&trades, &dividends, &snapshots, Some(1600.0), 2026);

        let records = BTreeMap::from([
            (
                2025,
                YearReviewRecord {
                    income: Some(720000.0),
                    bond_principal: Some(160000.0),
                    bond_interest: Some(7486.0),
                    deposit_principal: Some(932899.0),
                    deposit_interest: Some(6228.4),
                    ..YearReviewRecord::default()
                },
            ),
            (
                2026,
                YearReviewRecord {
                    income: Some(737020.0),
                    invested_adjustment: Some(-110000.0),
                    ..YearReviewRecord::default()
                },
            ),
        ]);
        let bonds = vec![BondYearFacts {
            principal: 50000.0,
            first_coupon_year: Some(2025),
            maturity_date: ym("2027-04-23"),
            received: vec![(2025, 997.25), (2025, 1002.75), (2026, 1000.0)],
        }];
        let deposits = [
            DepositFacts {
                bank: None,
                principal: Some(573266.77),
                interest: Some(6199.51),
                end_date: ym("2026-06-01"),
            },
            // Ends after `today` — the sheet's End filter skips it.
            DepositFacts {
                bank: None,
                principal: Some(10000.0),
                interest: Some(300.0),
                end_date: ym("2027-01-01"),
            },
        ];
        let today = ym("2026-09-22");
        // 2026's IBKR 轉入: the year's transfer-log sum joins invested.
        let transfers = BTreeMap::from([(2026, 131000.0)]);

        // The April step-ups in the stored month salaries: 47850 → 50810 →
        // 52700, so the derived raises are the sheet's 2960 and 1890.
        let last_salaries = BTreeMap::from([(2024, 47850.0), (2025, 50810.0), (2026, 52700.0)]);
        let rows_out = year_review_rows(
            &YearReviewInputs {
                summaries: &summaries,
                hk_years: &hk_years,
                records: &records,
                bonds: &bonds,
                deposits: &deposits,
                transfers: &transfers,
                last_salaries: &last_salaries,
            },
            today,
        );
        assert_eq!(rows_out.len(), 2);

        // 2025: the seeded overrides win over the coupon/deposit derivations.
        let y2025 = &rows_out[0];
        assert_eq!(y2025.year, 2025);
        assert!(approx_eq(y2025.assets.bond_principal.unwrap(), 160000.0));
        assert!(approx_eq(y2025.assets.bond_interest.unwrap(), 7486.0));
        assert!(y2025.assets.bond_overridden);
        assert!(approx_eq(y2025.assets.deposit_principal.unwrap(), 932899.0));
        // Snapshot cost wins over the trade-derived cumulative 1000.
        assert!(approx_eq(y2025.assets.stock_cost.unwrap(), 900.0));
        // 投資純利 = interest 0 + sold_pl −40.
        assert!(approx_eq(y2025.investment.net_investment.unwrap(), -40.0));
        // 存 = income − spend; 2025's spend is 0 (start == end + salary).
        assert!(approx_eq(y2025.assets.saved.unwrap(), 720000.0));
        assert!(approx_eq(y2025.assets.saved_pct.unwrap(), 1.0));

        // 2026: derived live. invested = HK net invested + 轉入 + adjustment.
        let y2026 = &rows_out[1];
        assert_eq!(y2025.investment.transferred, None);
        assert_eq!(y2026.investment.transferred, Some(131000.0));
        assert!(approx_eq(
            y2026.investment.invested.unwrap(),
            500.0 + 131000.0 - 110000.0
        ));
        assert!(approx_eq(y2026.investment.interest, 360.0));
        // 平均回報 divides by 12 flat, not the months present.
        assert!(approx_eq(y2026.investment.interest_avg, 30.0));
        assert!(approx_eq(y2026.investment.net_investment.unwrap(), 360.0));
        assert!(approx_eq(
            y2026.investment.invested_pct.unwrap(),
            21500.0 / (737020.0 + 360.0)
        ));
        // Bond derives from the held-in-year rule; the 2027-01 deposit is
        // excluded by the End filter.
        assert!(approx_eq(y2026.assets.bond_principal.unwrap(), 50000.0));
        assert!(approx_eq(y2026.assets.bond_interest.unwrap(), 1000.0));
        assert!(!y2026.assets.bond_overridden);
        assert!(approx_eq(
            y2026.assets.deposit_principal.unwrap(),
            573266.77
        ));
        // Blends: (bond i + div) ÷ (bond p + cost) = 1030 ÷ 51500.
        assert!(approx_eq(
            y2026.assets.income_cost_rate.unwrap(),
            1030.0 / 51500.0
        ));
        // 存 = income − spend = 737020 − 400.
        assert!(approx_eq(y2026.assets.saved.unwrap(), 736620.0));
        // Pool: income = 360 × 0.5 = 180; spend = 60.
        assert!(approx_eq(y2026.ledger.pool_income, 180.0));
        assert!(approx_eq(y2026.ledger.pool_spend, 60.0));
        // 2026-01 spend = 1100 − (800 − 100) = 400; YoY vs 2025's 0 spend:
        // the prior row's spend is Some(0) → absent.
        assert!(approx_eq(y2026.ledger.spend.unwrap(), 400.0));
        assert_eq!(y2026.ledger.spend_yoy, None);
        // income YoY: (737020 − 720000) ÷ 720000.
        assert!(approx_eq(
            y2026.assets.income_yoy.unwrap(),
            17020.0 / 720000.0
        ));
        // 月薪增幅 derives from the last salaries: 50810−47850 / 52700−50810.
        assert!(approx_eq(y2025.investment.raise.unwrap(), 2960.0));
        assert!(approx_eq(y2026.investment.raise.unwrap(), 1890.0));
    }

    #[test]
    fn raise_prefers_the_override_and_clamps_at_zero() {
        let records = BTreeMap::from([(
            2026,
            YearReviewRecord {
                raise: Some(2500.0),
                ..Default::default()
            },
        )]);
        let salaries = BTreeMap::from([(2025, 50810.0), (2026, 50000.0)]);
        let build = |records: &BTreeMap<i32, YearReviewRecord>, salaries: &BTreeMap<i32, f64>| {
            year_review_rows(
                &YearReviewInputs {
                    summaries: &[],
                    hk_years: &[],
                    records,
                    bonds: &[],
                    deposits: &[],
                    transfers: &BTreeMap::new(),
                    last_salaries: salaries,
                },
                ym("2026-09-24"),
            )
        };
        // The stored override wins over the salary diff.
        let rows_out = build(&records, &salaries);
        assert_eq!(rows_out[0].investment.raise, Some(2500.0));

        // A pay cut derives 0, not a negative figure; a missing salary side
        // leaves the derived raise absent.
        let rows_out = build(
            &BTreeMap::from([(2026, YearReviewRecord::default())]),
            &salaries,
        );
        assert_eq!(rows_out[0].investment.raise, Some(0.0));
        let rows_out = build(
            &BTreeMap::from([(2026, YearReviewRecord::default())]),
            &BTreeMap::from([(2026, 52700.0)]),
        );
        assert_eq!(rows_out[0].investment.raise, None);
    }

    #[test]
    fn invest_targets_derive_target_remain_growth_and_average() {
        let row = |year, invested, interest, pool_spend, raise| YearReviewRow {
            year,
            ledger: YearReviewLedger {
                pool_spend,
                ..Default::default()
            },
            investment: YearReviewInvestment {
                invested,
                interest,
                raise,
                ..Default::default()
            },
            ..Default::default()
        };
        let rows = vec![
            row(2023, Some(206523.15), 0.0, 0.0, None),
            row(2024, Some(345365.11), 36577.86, 17719.0, Some(2350.0)),
            row(2025, Some(428635.41), 51900.65, 38729.5, Some(2960.0)),
            row(2026, Some(182935.39), 62753.15, 9593.0, Some(1890.0)),
        ];
        let targets = invest_targets(&rows, 2026);

        // J22 = the mean of the three completed years' invested.
        assert!(approx_eq(
            targets.avg_invested.unwrap(),
            (206523.15 + 345365.11 + 428635.41) / 3.0
        ));

        // target = 428635.41 − 51900.65 − 38729.5×0.7 + 1890×6 + 62753.15
        //          + 9593×0.7 = 430432.36.
        let y2026 = &targets.rows[3];
        assert!(approx_eq(y2026.target.unwrap(), 430432.36));
        assert!(approx_eq(y2026.remain.unwrap(), 430432.36 - 182935.39));
        // Current-year growth measures the target against last year's invested.
        assert!(approx_eq(
            y2026.growth.unwrap(),
            (430432.36 - 428635.41) / 428635.41
        ));

        // Completed years grow on actual invested and carry no remain.
        let y2025 = &targets.rows[2];
        assert!(approx_eq(
            y2025.growth.unwrap(),
            (428635.41 - 345365.11) / 345365.11
        ));
        assert_eq!(y2025.remain, None);

        // Each row also carries the raw formula inputs, 0 when the year has
        // none — the footnote expands the target from these.
        assert!(approx_eq(y2026.interest, 62753.15));
        assert!(approx_eq(y2026.pool_spend, 9593.0));
        assert!(approx_eq(y2025.interest, 51900.65));
        assert!(approx_eq(y2025.pool_spend, 38729.5));
        assert_eq!(targets.rows[0].interest, 0.0);
        assert_eq!(targets.rows[0].pool_spend, 0.0);

        // The first year has no prior: target/growth absent, invested present.
        assert_eq!(targets.rows[0].target, None);
        assert_eq!(targets.rows[0].growth, None);
        assert!(targets.rows[0].invested.is_some());

        // The average skips completed years with no invested.
        let sparse = vec![
            row(2024, None, 0.0, 0.0, None),
            row(2025, Some(10.0), 0.0, 0.0, None),
        ];
        assert!(approx_eq(
            invest_targets(&sparse, 2026).avg_invested.unwrap(),
            10.0
        ));
        // Fewer than three completed years averages what's there; none → absent.
        assert!(approx_eq(
            invest_targets(&rows[..2], 2025).avg_invested.unwrap(),
            (206523.15 + 345365.11) / 2.0
        ));
        assert_eq!(invest_targets(&rows[..1], 2023).avg_invested, None);
    }

    #[test]
    fn running_averages_cover_every_month_with_a_value() {
        let mut rows = [
            month_row("2026-08-01", Some(1.0), Some(1.0)),
            month_row("2026-09-01", Some(1.0), Some(1.0)),
            month_row("2026-10-01", Some(1.0), Some(1.0)),
        ];
        for (index, row) in rows.iter_mut().enumerate() {
            row.total_assets = Some((index * 100) as f64);
            row.interest = 10.0 * (index + 1) as f64;
        }
        let averages = month_running_averages(&rows, &HashMap::new(), None);
        // total_change exists on the first two rows: 100 and 100.
        assert!(approx_eq(averages.total_change_avg.unwrap(), 100.0));
        // saved exists on the first two rows: 1 − 1 = 0 each.
        assert!(approx_eq(averages.saved_avg.unwrap(), 0.0));
        // interest averages over all three rows: (10+20+30)/3.
        assert!(approx_eq(averages.interest_avg.unwrap(), 20.0));
        assert_eq!(
            month_running_averages(&[], &HashMap::new(), None).interest_avg,
            None
        );
    }

    #[test]
    fn trailing_averages_cover_the_twelve_completed_months() {
        // 16 rows 2025-07..2026-10; current month 2026-09 → the window is the
        // latest 12 rows before it: 2025-09..2026-08 (indexes 2..=13).
        let months = [
            "2025-07-01",
            "2025-08-01",
            "2025-09-01",
            "2025-10-01",
            "2025-11-01",
            "2025-12-01",
            "2026-01-01",
            "2026-02-01",
            "2026-03-01",
            "2026-04-01",
            "2026-05-01",
            "2026-06-01",
            "2026-07-01",
            "2026-08-01",
            "2026-09-01",
            "2026-10-01",
        ];
        let rows: Vec<MonthStatRow> = months
            .iter()
            .enumerate()
            .map(|(index, month)| {
                let mut row = month_row(month, Some(1000.0 + index as f64), Some(100.0));
                row.total_assets = Some((index as f64 + 1.0) * 100.0);
                row.interest = (index + 1) as f64;
                row
            })
            .collect();

        let averages = trailing_averages(&rows, &HashMap::new(), ym("2026-09-01"));
        assert_eq!(averages.window_start, Some(ym("2025-09-01")));
        assert_eq!(averages.window_end, Some(ym("2026-08-01")));
        // Interest (index+1) over rows 2..=13: (3 + … + 14) / 12 = 8.5 — a
        // shifted window would give a different mean.
        assert!(approx_eq(averages.interest.unwrap(), 8.5));
        // spend = start + 0 − (next_start − salary) = 99 each; saved = 1.
        assert!(approx_eq(averages.month_spend.unwrap(), 99.0));
        assert!(approx_eq(averages.saved.unwrap(), 1.0));
        assert!(approx_eq(averages.total_change.unwrap(), 100.0));
        // ROUNDUP(99 × 1.05, −2) = ROUNDUP(103.95, −2) = 200.
        assert_eq!(averages.living_budget, Some(200.0));
    }

    #[test]
    fn trailing_averages_skip_months_with_no_values() {
        let rows = vec![
            month_row("2026-08-01", None, None),
            month_row("2026-09-01", Some(1000.0), Some(100.0)),
            // The current month is excluded even though it has data.
            month_row("2026-10-01", Some(1100.0), Some(100.0)),
        ];
        let averages = trailing_averages(&rows, &HashMap::new(), ym("2026-10-01"));
        // Only Sep carries a spend: 1000 − (1100 − 100) = 0, averaged over 1.
        assert!(approx_eq(averages.month_spend.unwrap(), 0.0));
        // Interest averages only over rows with data (Sep + Oct excluded by
        // window → Sep alone).
        assert_eq!(averages.interest, Some(0.0));
        assert_eq!(averages.window_start, Some(ym("2026-08-01")));
        assert_eq!(averages.window_end, Some(ym("2026-09-01")));

        // No completed rows at all → every figure absent.
        let empty = trailing_averages(&[], &HashMap::new(), ym("2026-10-01"));
        assert_eq!(empty.month_spend, None);
        assert_eq!(empty.living_budget, None);
        assert_eq!(empty.window_start, None);
    }
}
