use super::*;

use std::collections::{BTreeMap, HashMap};

use chrono::Datelike;
use serde::Serialize;
use utoipa::ToSchema;

use crate::models::MonthItemCategory;

// --- month stat (月結) ---

/// The stored columns of one `month_stats` row that the derivations read.
/// `total_assets`/`liquid_assets` are the *effective* values — the caller
/// resolves stored-or-live before calling.
#[derive(Debug, Clone, Copy)]
pub struct MonthStatRow {
    /// The first day of the month.
    pub month: chrono::NaiveDate,
    pub start_cash: Option<f64>,
    pub salary: Option<f64>,
    pub total_assets: Option<f64>,
    pub liquid_assets: Option<f64>,
    /// Hand-frozen H 月尾; wins over the `=F(n+1) − salary` chain.
    pub end_cash_override: Option<f64>,
    /// N 利息: the caller resolves `auto_interest + Σ interest items` before
    /// building the row — nothing is stored.
    pub interest: f64,
    pub pool_input: f64,
}

/// The only item facts the monthly derivations need.
#[derive(Debug, Clone, Copy)]
pub struct MonthItemFacts {
    pub category: MonthItemCategory,
    pub amount: f64,
    /// Entertainment items only: also subtract from `living_spend`.
    pub exclude_from_living: bool,
}

/// Per-month sums of the five item categories.
#[derive(Debug, Clone, Copy, Default)]
pub struct MonthItemSums {
    pub adjustment: f64,
    pub extra_spend: f64,
    pub income: f64,
    /// O 娛樂支出: Σ entertainment items.
    pub entertainment: f64,
    /// Σ entertainment items flagged `exclude_from_living`.
    pub entertainment_excluded: f64,
    /// The manual part of N 利息: Σ interest items.
    pub interest: f64,
}

pub fn month_item_sums(items: &[MonthItemFacts]) -> MonthItemSums {
    let mut sums = MonthItemSums::default();
    for item in items {
        match item.category {
            MonthItemCategory::Adjustment => sums.adjustment += item.amount,
            MonthItemCategory::ExtraSpend => sums.extra_spend += item.amount,
            MonthItemCategory::Income => sums.income += item.amount,
            MonthItemCategory::Entertainment => {
                sums.entertainment += item.amount;
                if item.exclude_from_living {
                    sums.entertainment_excluded += item.amount;
                }
            }
            MonthItemCategory::Interest => sums.interest += item.amount,
        }
    }
    sums
}

/// The figures derived on read for one month row; every field is absent while
/// its inputs are missing.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, ToSchema)]
pub struct MonthDerived {
    /// H 月尾(出糧前): the stored override, else the next row's start_cash −
    /// this row's salary.
    pub end_cash: Option<f64>,
    /// I 月支出: start_cash + Σadjustment − end_cash.
    pub month_spend: Option<f64>,
    /// J 生活支出: month_spend − Σextra_spend − Σ flagged entertainment.
    pub living_spend: Option<f64>,
    /// L 存: salary − month_spend + Σincome.
    pub saved: Option<f64>,
    /// C Changed: next row's total_assets − this row's.
    pub total_change: Option<f64>,
    /// E Changed: next row's liquid_assets − this row's.
    pub liquid_change: Option<f64>,
    /// The sheet's K column: `(J − J same month last year) / J` — the YoY
    /// living-spend change as a share of the current month's spend; absent
    /// while either side has no 生活支出 or J is zero.
    pub living_yoy: Option<f64>,
}

/// Derive every row's computed columns. `rows` must be sorted by month
/// ascending; `items` maps a month (first day) to its items.
pub fn month_derived(
    rows: &[MonthStatRow],
    items: &HashMap<chrono::NaiveDate, Vec<MonthItemFacts>>,
) -> Vec<MonthDerived> {
    month_derived_with_tail(rows, items, None)
}

/// `live_tail` stands in as the last row's missing "next row" for the C/E
/// Changed cells — the sheet's last-row cells diff against the live B1/H1,
/// reporting the in-progress month's change so far. Callers pass it only
/// while the latest stored row IS the current month: a future placeholder
/// has no knowable change, and a stale last row must not pull a live diff
/// into an old year.
pub fn month_derived_with_tail(
    rows: &[MonthStatRow],
    items: &HashMap<chrono::NaiveDate, Vec<MonthItemFacts>>,
    live_tail: Option<LiveTotals>,
) -> Vec<MonthDerived> {
    let mut derived: Vec<MonthDerived> = rows
        .iter()
        .enumerate()
        .map(|(index, row)| {
            let next = rows.get(index + 1);
            let sums = items
                .get(&row.month)
                .map(|items| month_item_sums(items))
                .unwrap_or_default();

            // The sheet's `=F(n+1) − <salary>` subtracts THIS month's salary:
            // start_cash(n+1) is the post-salary balance, so the month-end
            // cash is it minus the salary row n itself recorded. A stored
            // override wins — the chain cannot reproduce hand-frozen cells
            // (e.g. the first ledger month's typed bank balance).
            let end_cash = row
                .end_cash_override
                .or_else(|| next.and_then(|next| Some(next.start_cash? - row.salary?)));
            let month_spend = match (row.start_cash, end_cash) {
                (Some(start_cash), Some(end_cash)) => Some(start_cash + sums.adjustment - end_cash),
                _ => None,
            };
            let living_spend =
                month_spend.map(|spend| spend - sums.extra_spend - sums.entertainment_excluded);
            let saved = match (row.salary, month_spend) {
                (Some(salary), Some(spend)) => Some(salary - spend + sums.income),
                _ => None,
            };
            let total_change = match next {
                Some(next) => match (next.total_assets, row.total_assets) {
                    (Some(next_total), Some(total)) => Some(next_total - total),
                    _ => None,
                },
                None => match (live_tail, row.total_assets) {
                    (Some(live), Some(total)) => Some(live.total_assets - total),
                    _ => None,
                },
            };
            let liquid_change = match next {
                Some(next) => match (next.liquid_assets, row.liquid_assets) {
                    (Some(next_liquid), Some(liquid)) => Some(next_liquid - liquid),
                    _ => None,
                },
                None => match (live_tail, row.liquid_assets) {
                    (Some(live), Some(liquid)) => Some(live.liquid_assets - liquid),
                    _ => None,
                },
            };
            MonthDerived {
                end_cash,
                month_spend,
                living_spend,
                saved,
                total_change,
                liquid_change,
                living_yoy: None,
            }
        })
        .collect();
    // Second pass: the K column needs the same month one year earlier, which
    // is a date lookup (stored months may skip), not `index - 12`.
    let by_month: HashMap<chrono::NaiveDate, Option<f64>> = rows
        .iter()
        .zip(derived.iter())
        .map(|(row, d)| (row.month, d.living_spend))
        .collect();
    for (row, d) in rows.iter().zip(derived.iter_mut()) {
        let prior = row
            .month
            .checked_sub_months(chrono::Months::new(12))
            .and_then(|prev| by_month.get(&prev).copied().flatten());
        d.living_yoy = match (d.living_spend, prior) {
            (Some(now), Some(prev)) if now != 0.0 => Some((now - prev) / now),
            _ => None,
        };
    }
    derived
}

/// The year's pool rate: the exact year's rate, else the latest earlier
/// year's, else none — the sheet's rate changes at year-end and applies to
/// that whole year.
pub fn pool_rate_for_year(rates: &BTreeMap<i32, f64>, year: i32) -> Option<f64> {
    rates
        .get(&year)
        .copied()
        .or_else(|| rates.range(..year).next_back().map(|(_, rate)| *rate))
}

/// The 開心Pool closing balance per year, chained:
/// `balance(y) = balance(y−1) + Σinterest(y) × rate(y) − Σentertainment(y)
/// + Σpool_input(y)`, base 0. A year with no applicable rate contributes no
///   pool income but still rolls its entertainment and inputs forward.
pub fn pool_balances(
    rows: &[MonthStatRow],
    items: &HashMap<chrono::NaiveDate, Vec<MonthItemFacts>>,
    rates: &BTreeMap<i32, f64>,
) -> BTreeMap<i32, f64> {
    let mut by_year: BTreeMap<i32, (f64, f64, f64)> = BTreeMap::new();
    for row in rows {
        let sums = items
            .get(&row.month)
            .map(|items| month_item_sums(items))
            .unwrap_or_default();
        let entry = by_year.entry(row.month.year()).or_default();
        entry.0 += row.interest;
        entry.1 += sums.entertainment;
        entry.2 += row.pool_input;
    }
    let mut balance = 0.0;
    let mut balances = BTreeMap::new();
    for (year, (interest, entertainment, pool_input)) in by_year {
        let rate = pool_rate_for_year(rates, year).unwrap_or(0.0);
        balance += interest * rate - entertainment + pool_input;
        balances.insert(year, balance);
    }
    balances
}

/// One year's aggregate row (the sheet's rows 2–4). Sums and averages are
/// absent while no month of the year has the underlying figure; the scalar
/// columns (interest/entertainment/pool_input) always sum.
#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
pub struct MonthYearSummary {
    pub year: i32,
    /// 總數+: Σ total_change.
    pub total_change_sum: Option<f64>,
    /// 平均總數.
    pub total_change_avg: Option<f64>,
    /// 支出: Σ month_spend.
    pub spend_sum: Option<f64>,
    /// 平均支出.
    pub spend_avg: Option<f64>,
    /// 生活平均支出.
    pub living_avg: Option<f64>,
    /// 娛樂支出: Σ entertainment.
    pub entertainment_sum: f64,
    /// 利息回報: Σ interest.
    pub interest_sum: f64,
    /// 平均回報: interest_sum ÷ months that carry a value — the sheet's
    /// `AVERAGE(N…)` skips empty N cells, so a stored row only counts when it
    /// has a 月初 (`start_cash`) or a nonzero 利息.
    pub interest_avg: f64,
    /// 投資純利: interest_sum + the year's HK sold P/L (`'港股'!D32`-style
    /// figure the app does not compute yet); absent while the year has no
    /// sold-P/L entry.
    pub net_investment: Option<f64>,
    /// Pool income: interest_sum × the year's rate (0 without one).
    pub pool_income: f64,
    /// 開心Pool結餘: the year's chained closing balance.
    pub pool_balance: f64,
    /// Irene + 開心 Pool: Σ pool_input.
    pub pool_input_sum: f64,
    /// Stored months in the year.
    pub months: usize,
}

/// Per-year aggregates over `rows` (sorted by month ascending), with the pool
/// figures priced by `rates` (`overview.pool_rate.<year>`). `hk_sold_pl` maps
/// a year to its HK sold P/L for 投資純利; years without an entry report none.
/// `live_tail` is the gated live totals for the last row's Changed cell (see
/// `month_derived_with_tail`) — the sheet's yearly ΣC counts the current
/// month's in-flight change.
pub fn month_year_summaries(
    rows: &[MonthStatRow],
    items: &HashMap<chrono::NaiveDate, Vec<MonthItemFacts>>,
    rates: &BTreeMap<i32, f64>,
    hk_sold_pl: &BTreeMap<i32, f64>,
    live_tail: Option<LiveTotals>,
) -> Vec<MonthYearSummary> {
    let derived = month_derived_with_tail(rows, items, live_tail);
    let balances = pool_balances(rows, items, rates);

    struct Acc {
        total_change: Vec<f64>,
        spend: Vec<f64>,
        living: Vec<f64>,
        entertainment: f64,
        interest: f64,
        interest_months: usize,
        pool_input: f64,
        months: usize,
    }

    let mut years: BTreeMap<i32, Acc> = BTreeMap::new();
    for (row, derived) in rows.iter().zip(derived.iter()) {
        let acc = years.entry(row.month.year()).or_insert_with(|| Acc {
            total_change: Vec::new(),
            spend: Vec::new(),
            living: Vec::new(),
            entertainment: 0.0,
            interest: 0.0,
            interest_months: 0,
            pool_input: 0.0,
            months: 0,
        });
        acc.months += 1;
        acc.entertainment += items
            .get(&row.month)
            .map(|items| month_item_sums(items).entertainment)
            .unwrap_or_default();
        acc.interest += row.interest;
        if row.start_cash.is_some() || row.interest != 0.0 {
            acc.interest_months += 1;
        }
        acc.pool_input += row.pool_input;
        if let Some(change) = derived.total_change {
            acc.total_change.push(change);
        }
        if let Some(spend) = derived.month_spend {
            acc.spend.push(spend);
        }
        if let Some(living) = derived.living_spend {
            acc.living.push(living);
        }
    }

    years
        .into_iter()
        .map(|(year, acc)| {
            let sum = |values: &[f64]| (!values.is_empty()).then(|| values.iter().sum::<f64>());
            let avg = |values: &[f64]| {
                (!values.is_empty()).then(|| values.iter().sum::<f64>() / values.len() as f64)
            };
            let interest_sum = acc.interest;
            let rate = pool_rate_for_year(rates, year).unwrap_or(0.0);
            MonthYearSummary {
                year,
                total_change_sum: sum(&acc.total_change),
                total_change_avg: avg(&acc.total_change),
                spend_sum: sum(&acc.spend),
                spend_avg: avg(&acc.spend),
                living_avg: avg(&acc.living),
                entertainment_sum: acc.entertainment,
                interest_sum,
                interest_avg: if acc.interest_months == 0 {
                    0.0
                } else {
                    interest_sum / acc.interest_months as f64
                },
                net_investment: hk_sold_pl.get(&year).map(|sold_pl| interest_sum + sold_pl),
                pool_income: interest_sum * rate,
                pool_balance: balances.get(&year).copied().unwrap_or(0.0),
                pool_input_sum: acc.pool_input,
                months: acc.months,
            }
        })
        .collect()
}

/// The sheet's row-8 running averages over every stored month.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, ToSchema)]
pub struct MonthRunningAverages {
    /// AVERAGE of total_change over months that have it.
    pub total_change_avg: Option<f64>,
    /// AVERAGE of liquid_change (流動資產 Changed) over months that have it.
    pub liquid_change_avg: Option<f64>,
    /// AVERAGE of 存 over months that have it.
    pub saved_avg: Option<f64>,
    /// AVERAGE of 利息 over months that carry a value — same rule as the
    /// yearly `interest_avg` (non-NULL `start_cash` or nonzero interest).
    pub interest_avg: Option<f64>,
}

pub fn month_running_averages(
    rows: &[MonthStatRow],
    items: &HashMap<chrono::NaiveDate, Vec<MonthItemFacts>>,
    live_tail: Option<LiveTotals>,
) -> MonthRunningAverages {
    let derived = month_derived_with_tail(rows, items, live_tail);
    let avg = |values: Vec<f64>| {
        (!values.is_empty()).then(|| values.iter().sum::<f64>() / values.len() as f64)
    };
    MonthRunningAverages {
        total_change_avg: avg(derived.iter().filter_map(|d| d.total_change).collect()),
        liquid_change_avg: avg(derived.iter().filter_map(|d| d.liquid_change).collect()),
        saved_avg: avg(derived.iter().filter_map(|d| d.saved).collect()),
        interest_avg: avg(rows
            .iter()
            .filter(|row| row.start_cash.is_some() || row.interest != 0.0)
            .map(|row| row.interest)
            .collect()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calc::test_helpers::*;

    #[test]
    fn month_spend_falls_out_of_the_balance_difference() {
        // Spec scenario: September 30000 start +8009.3 adjustment; October
        // start 32000 salary 52700 → 月尾 −20700, 月支出 58709.3.
        let rows = [
            month_row("2026-09-01", Some(30000.0), Some(52700.0)),
            month_row("2026-10-01", Some(32000.0), Some(52700.0)),
        ];
        let mut items = HashMap::new();
        items.insert(
            ym("2026-09-01"),
            vec![item(MonthItemCategory::Adjustment, 8009.3)],
        );
        let derived = month_derived(&rows, &items);
        assert!(approx_eq(derived[0].end_cash.unwrap(), -20700.0));
        assert!(
            approx_eq(derived[0].month_spend.unwrap(), 58709.3),
            "month_spend = {:?}",
            derived[0].month_spend
        );
        assert!(approx_eq(derived[0].living_spend.unwrap(), 58709.3));
        assert!(approx_eq(derived[0].saved.unwrap(), -6009.3));
    }

    #[test]
    fn latest_month_has_no_month_end() {
        let rows = [month_row("2026-10-01", Some(32000.0), Some(52700.0))];
        let derived = month_derived(&rows, &HashMap::new());
        assert_eq!(derived[0].end_cash, None);
        assert_eq!(derived[0].month_spend, None);
        assert_eq!(derived[0].living_spend, None);
        assert_eq!(derived[0].saved, None);
    }

    #[test]
    fn end_cash_uses_the_rows_own_salary() {
        // The sheet's `=F(n+1) − salary` subtracts the row's own salary: with
        // an April raise the March H literal still uses the March figure.
        let rows = [
            month_row("2026-09-01", Some(30000.0), Some(45500.0)),
            month_row("2026-10-01", Some(32000.0), Some(52700.0)),
        ];
        let derived = month_derived(&rows, &HashMap::new());
        assert!(approx_eq(derived[0].end_cash.unwrap(), -13500.0));
        // And end_cash is absent while the next row lacks start_cash or this
        // row lacks a salary.
        let rows = [
            month_row("2026-09-01", Some(30000.0), Some(45500.0)),
            month_row("2026-10-01", None, Some(52700.0)),
        ];
        assert_eq!(month_derived(&rows, &HashMap::new())[0].end_cash, None);
        let rows = [
            month_row("2026-09-01", Some(30000.0), None),
            month_row("2026-10-01", Some(32000.0), Some(52700.0)),
        ];
        assert_eq!(month_derived(&rows, &HashMap::new())[0].end_cash, None);
    }

    #[test]
    fn end_cash_override_wins_over_the_chain() {
        // 2023-12: the sheet's first row typed the real bank balance
        // (24610.32) because no next-row 月初 convention existed yet.
        let rows = [
            MonthStatRow {
                end_cash_override: Some(24610.32),
                ..month_row("2023-12-01", Some(64925.18), Some(45500.0))
            },
            month_row("2024-01-01", Some(26391.71), Some(45500.0)),
        ];
        let mut items = HashMap::new();
        items.insert(
            ym("2023-12-01"),
            vec![item(MonthItemCategory::Adjustment, -25508.47)],
        );
        let derived = month_derived(&rows, &items);
        assert!(approx_eq(derived[0].end_cash.unwrap(), 24610.32));
        assert!(approx_eq(derived[0].month_spend.unwrap(), 14806.39));
        assert!(approx_eq(derived[0].saved.unwrap(), 30693.61));
    }

    #[test]
    fn in_out_adjustment_pair_leaves_month_spend_unchanged() {
        // Spec scenario: start_cash 30000 with −50000 定期 start and +55000
        // 定期 end, next row start 62700 salary 52700 → end_cash 10000 and
        // 月支出 25000 — the deposit flows moved the balance, not the spend.
        let rows = [
            month_row("2026-09-01", Some(30000.0), Some(52700.0)),
            month_row("2026-10-01", Some(62700.0), Some(52700.0)),
        ];
        let mut items = HashMap::new();
        items.insert(
            ym("2026-09-01"),
            vec![
                item(MonthItemCategory::Adjustment, -50000.0),
                item(MonthItemCategory::Adjustment, 55000.0),
            ],
        );
        let derived = month_derived(&rows, &items)[0];
        assert!(approx_eq(derived.end_cash.unwrap(), 10000.0));
        assert!(approx_eq(derived.month_spend.unwrap(), 25000.0));
    }

    #[test]
    fn extra_spend_lowers_only_living_spend() {
        let rows = [
            month_row("2026-09-01", Some(30000.0), Some(52700.0)),
            month_row("2026-10-01", Some(57700.0), Some(52700.0)),
        ];
        let mut items = HashMap::new();
        items.insert(
            ym("2026-09-01"),
            vec![item(MonthItemCategory::ExtraSpend, 12000.0)],
        );
        let derived = month_derived(&rows, &items)[0];
        assert!(approx_eq(derived.month_spend.unwrap(), 25000.0));
        assert!(approx_eq(derived.living_spend.unwrap(), 13000.0));
        assert!(approx_eq(derived.saved.unwrap(), 27700.0));
    }

    #[test]
    fn living_yoy_compares_the_same_month_last_year() {
        // The sheet's K cell: `=(J − J a year earlier) / J`.
        // 2025-08 living 42700, 2026-08 living 47700 → (47700−42700)/47700.
        let rows = [
            month_row("2025-08-01", Some(30000.0), Some(52700.0)),
            month_row("2025-09-01", Some(40000.0), Some(52700.0)),
            month_row("2026-08-01", Some(35000.0), Some(52700.0)),
            month_row("2026-09-01", Some(40000.0), Some(52700.0)),
        ];
        let derived = month_derived(&rows, &HashMap::new());
        assert_eq!(derived[0].living_yoy, None);
        assert!(approx_eq(
            derived[2].living_yoy.unwrap(),
            (47700.0 - 42700.0) / 47700.0
        ));
        // A gap (2025-08 missing entirely) leaves the figure absent — the
        // lookup is by date, not by position.
        let sparse = [
            month_row("2026-08-01", Some(35000.0), Some(52700.0)),
            month_row("2026-09-01", Some(40000.0), Some(52700.0)),
        ];
        assert_eq!(month_derived(&sparse, &HashMap::new())[0].living_yoy, None);
    }

    #[test]
    fn flagged_entertainment_also_leaves_living_spend() {
        // Spec scenario: spend 25000, extra_spend 12000, entertainment items
        // 500 + 4700 (flagged) + 75 → 生活支出 8300, 娛樂支出 5275.
        let rows = [
            month_row("2026-09-01", Some(30000.0), Some(52700.0)),
            month_row("2026-10-01", Some(57700.0), Some(52700.0)),
        ];
        let mut items = HashMap::new();
        items.insert(
            ym("2026-09-01"),
            vec![
                item(MonthItemCategory::ExtraSpend, 12000.0),
                item(MonthItemCategory::Entertainment, 500.0),
                MonthItemFacts {
                    category: MonthItemCategory::Entertainment,
                    amount: 4700.0,
                    exclude_from_living: true,
                },
                item(MonthItemCategory::Entertainment, 75.0),
            ],
        );
        let derived = month_derived(&rows, &items)[0];
        assert!(approx_eq(derived.month_spend.unwrap(), 25000.0));
        assert!(approx_eq(derived.living_spend.unwrap(), 8300.0));
        let sums = month_item_sums(&items[&ym("2026-09-01")]);
        assert!(approx_eq(sums.entertainment, 5275.0));
        assert!(approx_eq(sums.entertainment_excluded, 4700.0));
    }

    #[test]
    fn income_raises_saved() {
        let rows = [
            month_row("2026-09-01", Some(30000.0), Some(52700.0)),
            month_row("2026-10-01", Some(57700.0), Some(52700.0)),
        ];
        let mut items = HashMap::new();
        items.insert(
            ym("2026-09-01"),
            vec![item(MonthItemCategory::Income, 20000.0)],
        );
        let derived = month_derived(&rows, &items)[0];
        assert!(approx_eq(derived.month_spend.unwrap(), 25000.0));
        assert!(approx_eq(derived.saved.unwrap(), 47700.0));
    }

    #[test]
    fn changes_diff_against_the_next_row() {
        let mut rows = [
            month_row("2026-09-01", Some(30000.0), Some(52700.0)),
            month_row("2026-10-01", Some(32000.0), Some(52700.0)),
        ];
        rows[0].total_assets = Some(1_000_000.0);
        rows[0].liquid_assets = Some(500_000.0);
        rows[1].total_assets = Some(1_050_000.0);
        rows[1].liquid_assets = Some(490_000.0);
        let derived = month_derived(&rows, &HashMap::new());
        assert!(approx_eq(derived[0].total_change.unwrap(), 50_000.0));
        assert!(approx_eq(derived[0].liquid_change.unwrap(), -10_000.0));
        // The latest row has nothing to diff against, and a missing total on
        // either side leaves the change absent.
        assert_eq!(derived[1].total_change, None);
        rows[1].liquid_assets = None;
        assert_eq!(month_derived(&rows, &HashMap::new())[0].liquid_change, None);
    }

    #[test]
    fn last_row_diffs_the_live_tail() {
        // The sheet's last-row C/E cells read the live B1/H1 — the current
        // month's in-progress change — so the last stored row diffs the live
        // totals even with no next row.
        let mut rows = [
            month_row("2026-09-01", Some(30000.0), Some(52700.0)),
            month_row("2026-10-01", Some(32000.0), Some(52700.0)),
        ];
        rows[0].total_assets = Some(1_000_000.0);
        rows[0].liquid_assets = Some(500_000.0);
        rows[1].total_assets = Some(1_050_000.0);
        rows[1].liquid_assets = Some(490_000.0);
        let live = LiveTotals {
            total_assets: 1_060_000.0,
            liquid_assets: 495_000.0,
        };
        let derived = month_derived_with_tail(&rows, &HashMap::new(), Some(live));
        assert!(approx_eq(derived[0].total_change.unwrap(), 50_000.0));
        assert!(approx_eq(derived[1].total_change.unwrap(), 10_000.0));
        assert!(approx_eq(derived[1].liquid_change.unwrap(), 5_000.0));

        // A live-filled last row reports 0 — its own total IS the live total.
        rows[1].total_assets = Some(1_060_000.0);
        rows[1].liquid_assets = Some(495_000.0);
        let derived = month_derived_with_tail(&rows, &HashMap::new(), Some(live));
        assert_eq!(derived[1].total_change, Some(0.0));

        // No tail and no next row → still absent; a missing last total leaves
        // the change absent even with a tail.
        assert_eq!(month_derived(&rows, &HashMap::new())[1].total_change, None);
        rows[1].total_assets = None;
        assert_eq!(
            month_derived_with_tail(&rows, &HashMap::new(), Some(live))[1].total_change,
            None
        );
    }

    #[test]
    fn year_summary_aggregates_and_prices_the_pool() {
        let mut rows = vec![
            month_row("2025-11-01", Some(1.0), Some(1.0)),
            month_row("2025-12-01", Some(1.0), Some(1.0)),
            month_row("2026-01-01", Some(1.0), Some(1.0)),
            month_row("2026-02-01", Some(1.0), Some(1.0)),
        ];
        rows[2].interest = 1000.0;
        rows[3].interest = 500.0;
        rows[3].pool_input = 300.0;
        // 2026-01: start 1 + adj 0 − (1−1) = spend 1; total_change needs totals.
        for (index, row) in rows.iter_mut().enumerate() {
            row.total_assets = Some(100.0 * (index + 1) as f64);
        }
        let mut items = HashMap::new();
        items.insert(
            ym("2026-01-01"),
            vec![item(MonthItemCategory::Entertainment, 200.0)],
        );
        let rates = BTreeMap::from([(2025, 0.425), (2026, 0.337)]);
        let sold_pl = BTreeMap::from([(2026, 24124.13)]);
        let summaries = month_year_summaries(&rows, &items, &rates, &sold_pl, None);
        let y2026 = summaries.iter().find(|s| s.year == 2026).unwrap();
        assert_eq!(y2026.months, 2);
        assert!(approx_eq(y2026.interest_sum, 1500.0));
        assert!(approx_eq(y2026.interest_avg, 750.0));
        assert!(approx_eq(y2026.entertainment_sum, 200.0));
        assert!(approx_eq(y2026.pool_input_sum, 300.0));
        assert!(approx_eq(y2026.pool_income, 1500.0 * 0.337));
        // 投資純利 = interest_sum + the year's HK sold P/L; absent without one.
        assert!(approx_eq(y2026.net_investment.unwrap(), 1500.0 + 24124.13));
        assert_eq!(
            summaries
                .iter()
                .find(|s| s.year == 2025)
                .unwrap()
                .net_investment,
            None
        );
        // 2025 contributed no interest, so 2026 closes at
        // 0 + 1500×0.337 − 200 + 300.
        assert!(approx_eq(y2026.pool_balance, 605.5));
        // Only January has a next row, so only it reports a spend.
        assert!(approx_eq(y2026.spend_sum.unwrap(), 1.0));
    }

    #[test]
    fn pool_rate_falls_back_to_the_latest_earlier_year() {
        let rates = BTreeMap::from([(2024, 0.53), (2026, 0.337)]);
        assert_eq!(pool_rate_for_year(&rates, 2026), Some(0.337));
        assert_eq!(pool_rate_for_year(&rates, 2025), Some(0.53));
        assert_eq!(pool_rate_for_year(&rates, 2023), None);
    }

    #[test]
    fn pool_balance_chains_and_zero_rate_gives_no_pool_income() {
        let mut rows = vec![
            month_row("2025-12-01", Some(1.0), Some(1.0)),
            month_row("2026-01-01", Some(1.0), Some(1.0)),
        ];
        rows[0].interest = 1000.0;
        rows[1].interest = 2000.0;
        rows[1].pool_input = 50.0;
        let mut items = HashMap::new();
        items.insert(
            ym("2025-12-01"),
            vec![item(MonthItemCategory::Entertainment, 100.0)],
        );
        let rates = BTreeMap::from([(2026, 0.5)]);
        let balances = pool_balances(&rows, &items, &rates);
        // 2025 has no rate: 0 + 0 − 100 + 0 = −100. 2026: −100 + 1000 − 0 + 50.
        assert!(approx_eq(balances[&2025], -100.0));
        assert!(approx_eq(balances[&2026], 950.0));
        // Spec scenario: 2025 closed 5717.57; 2026 interest 62033.15,
        // entertainment 8593, pool input 4142.66 at 0.337 → ≈ 22172.4.
        let mut rows = vec![
            month_row("2025-06-01", Some(1.0), Some(1.0)),
            month_row("2026-06-01", Some(1.0), Some(1.0)),
        ];
        rows[0].interest = 5717.57 / 0.425;
        rows[1].interest = 62033.15;
        rows[1].pool_input = 4142.66;
        let mut items = HashMap::new();
        items.insert(
            ym("2026-06-01"),
            vec![item(MonthItemCategory::Entertainment, 8593.0)],
        );
        let rates = BTreeMap::from([(2025, 0.425), (2026, 0.337)]);
        let balances = pool_balances(&rows, &items, &rates);
        assert!(
            approx_eq(
                balances[&2026],
                5717.57 + 62033.15 * 0.337 - 8593.0 + 4142.66
            ),
            "balance = {}",
            balances[&2026]
        );
    }
}
