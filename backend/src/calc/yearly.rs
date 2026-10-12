use super::*;

use std::collections::HashMap;

use chrono::Datelike;
use serde::Serialize;
use utoipa::ToSchema;

use crate::models::TradeType;

/// Frozen per-(market, year) figures from the `year_snapshots` table. A None
/// field is no override — the yearly row falls back to the computed figure.
#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
pub struct YearSnapshot {
    pub invested: Option<f64>,
    pub cost: Option<f64>,
    pub market_value: Option<f64>,
    /// 賣出損益: the year's realized P/L, entered by hand (the workbook never
    /// recorded SELL trades). NULL reports the row's `sold_pl` absent.
    pub sold_pl: Option<f64>,
    pub updated_at: String,
}

/// One row of the per-market yearly summary table: the sheet's B–M year
/// block (net invested, sold P/L, 成本, 報酬率s, year-end value, 派息, month,
/// and the two year-over-year changes).
#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
pub struct YearRow {
    pub year: i32,
    /// net invested: Σ BUY total − Σ SELL total in the year; a stored
    /// snapshot value wins over the computation.
    pub invested: f64,
    /// 年末總成本: cumulative Σ BUY total through Dec 31 (SELL rows do not
    /// reduce it); a stored snapshot value wins.
    pub cost: f64,
    /// 年末總市值: the snapshot value, the live total for the current year
    /// without a snapshot, or empty for a past year without one.
    pub market_value: Option<f64>,
    /// 當年派息: Σ received_amount with pay_date in the year; pending
    /// estimates are excluded.
    pub dividends: f64,
    /// 報酬率 1 = dividends ÷ cost; empty when cost is not positive.
    pub yield_on_cost: Option<f64>,
    /// 報酬率 2 = dividends ÷ market_value; empty without a market value.
    pub yield_on_value: Option<f64>,
    /// 月均派息 = dividends ÷ 12.
    pub monthly_dividend: f64,
    /// (dividends − prior year's dividends) ÷ prior year's dividends;
    /// empty for the first row or a zero prior year.
    pub dividend_yoy: Option<f64>,
    /// (cost − prior year's cost) ÷ prior year's cost — the sheet's
    /// (F−F′)/F′ over the cumulative cost column; empty like dividend_yoy.
    pub invested_yoy: Option<f64>,
    /// 賣出損益: the year's realized sell P/L — the stored manual figure,
    /// absent while none is stored (SELL trades are not recorded).
    pub sold_pl: Option<f64>,
    /// The stored snapshot, when the year has one — the UI marks frozen
    /// cells from its non-null fields.
    pub snapshot: Option<YearSnapshot>,
}

/// One market's yearly rows: from the earliest year with a trade, dividend
/// or snapshot through `current_year`. `live_market_value` fills the current
/// year's 總市值 when that year has no stored snapshot.
pub fn yearly_rows(
    trades: &[(chrono::NaiveDate, TradeFacts)],
    dividends: &[DividendFacts<'_>],
    snapshots: &HashMap<i32, YearSnapshot>,
    live_market_value: Option<f64>,
    current_year: i32,
) -> Vec<YearRow> {
    let mut buy_by_year: HashMap<i32, f64> = HashMap::new();
    let mut sell_by_year: HashMap<i32, f64> = HashMap::new();
    for (date, trade) in trades {
        let bucket = match trade.trade_type {
            TradeType::Buy => &mut buy_by_year,
            TradeType::Sell => &mut sell_by_year,
        };
        *bucket.entry(date.year()).or_default() += trade.total;
    }
    let mut dividends_by_year: HashMap<i32, f64> = HashMap::new();
    for dividend in dividends {
        if let Some(received) = dividend.received_amount {
            *dividends_by_year
                .entry(dividend.pay_date.year())
                .or_default() += received;
        }
    }

    let first = buy_by_year
        .keys()
        .chain(sell_by_year.keys())
        .chain(dividends_by_year.keys())
        .chain(snapshots.keys())
        .copied()
        .min()
        .unwrap_or(current_year)
        .min(current_year);

    let mut rows = Vec::new();
    let mut cumulative_cost = 0.0;
    let mut previous: Option<(f64, f64)> = None; // prior row's (cost, dividends)
    for year in first..=current_year {
        let bought = buy_by_year.get(&year).copied().unwrap_or(0.0);
        let sold = sell_by_year.get(&year).copied().unwrap_or(0.0);
        cumulative_cost += bought;

        let snapshot = snapshots.get(&year);
        let override_field = |field: fn(&YearSnapshot) -> Option<f64>| snapshot.and_then(field);
        let invested = override_field(|s| s.invested).unwrap_or(bought - sold);
        let cost = override_field(|s| s.cost).unwrap_or(cumulative_cost);
        let market_value = override_field(|s| s.market_value).or_else(|| {
            (year == current_year)
                .then_some(live_market_value)
                .flatten()
        });
        let dividends = dividends_by_year.get(&year).copied().unwrap_or(0.0);

        let yield_on_cost = (cost > 0.0).then(|| dividends / cost);
        let yield_on_value = market_value
            .filter(|value| *value > 0.0)
            .map(|value| dividends / value);
        let (dividend_yoy, invested_yoy) = match previous {
            Some((prev_cost, prev_dividends)) => (
                (prev_dividends > 0.0).then(|| (dividends - prev_dividends) / prev_dividends),
                (prev_cost > 0.0).then(|| (cost - prev_cost) / prev_cost),
            ),
            None => (None, None),
        };
        previous = Some((cost, dividends));

        rows.push(YearRow {
            year,
            invested,
            cost,
            market_value,
            dividends,
            yield_on_cost,
            yield_on_value,
            monthly_dividend: dividends / 12.0,
            dividend_yoy,
            invested_yoy,
            sold_pl: override_field(|s| s.sold_pl),
            snapshot: snapshot.cloned(),
        });
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calc::test_helpers::*;

    #[test]
    fn yearly_rows_accumulate_cost_and_report_per_year_figures() {
        let trades = [
            dated_buy("2023-06-01", 100.0, 96523.15),
            dated_buy("2024-03-01", 100.0, 242135.49),
            dated_buy("2025-03-01", 100.0, 496006.18),
        ];
        let dividends = [
            dividend_fact("中國銀行", "2024-12-16", Some(20641.59)),
            dividend_fact("中國銀行", "2025-08-29", Some(38209.27)),
        ];
        let rows = yearly_rows(&trades, &dividends, &HashMap::new(), Some(500.0), 2025);

        assert_eq!(rows.len(), 3);
        assert!(approx_eq(rows[0].invested, 96523.15));
        assert!(approx_eq(rows[0].cost, 96523.15));
        // Cumulative 成本: each year adds its buys.
        assert!(approx_eq(rows[1].cost, 338658.64));
        assert!(approx_eq(rows[2].cost, 834664.82));
        assert!(approx_eq(rows[1].dividends, 20641.59));
        assert!(approx_eq(rows[2].monthly_dividend, 38209.27 / 12.0));
    }

    #[test]
    fn yearly_rows_count_sells_against_invested_but_not_cost() {
        let trades = [
            dated_buy("2024-01-01", 100.0, 1000.0),
            (
                day("2025-01-01"),
                TradeFacts {
                    trade_type: TradeType::Sell,
                    shares: 40.0,
                    total: 400.0,
                },
            ),
            (day("2025-06-01"), buy(10.0, 500.0)),
        ];
        let rows = yearly_rows(&trades, &[], &HashMap::new(), Some(600.0), 2025);

        // 2025 invested = Σ BUY − Σ SELL = 500 − 400; cost stays Σ BUY only.
        assert!(approx_eq(rows[1].invested, 100.0));
        assert!(approx_eq(rows[1].cost, 1500.0));
    }

    #[test]
    fn yearly_rows_first_year_has_no_yoy_and_past_years_have_no_value() {
        let trades = [
            dated_buy("2024-01-01", 100.0, 300373.62),
            dated_buy("2025-01-01", 100.0, 428635.41),
        ];
        let dividends = [
            dividend_fact("中國銀行", "2024-12-16", Some(20641.59)),
            dividend_fact("中國銀行", "2025-08-29", Some(38209.27)),
        ];
        let rows = yearly_rows(&trades, &dividends, &HashMap::new(), None, 2025);

        assert_eq!(rows[0].dividend_yoy, None);
        assert_eq!(rows[0].invested_yoy, None);
        // 2024 has no snapshot and is not current: 總市值 stays empty, so
        // 報酬率 2 is empty too while the other columns still compute.
        assert_eq!(rows[0].market_value, None);
        assert_eq!(rows[0].yield_on_value, None);
        assert!(approx_eq(
            rows[0].yield_on_cost.unwrap(),
            20641.59 / 300373.62
        ));
        // No live total supplied: the current year's 總市值 stays empty too.
        assert_eq!(rows[1].market_value, None);
        assert!(approx_eq(
            rows[1].dividend_yoy.unwrap(),
            (38209.27 - 20641.59) / 20641.59
        ));
        assert!(approx_eq(
            rows[1].invested_yoy.unwrap(),
            (729009.03 - 300373.62) / 300373.62
        ));
    }

    #[test]
    fn yearly_rows_live_value_fills_current_year_and_snapshot_wins() {
        let trades = [dated_buy("2024-01-01", 100.0, 300373.62)];
        let mut snapshots = HashMap::new();
        snapshots.insert(2024, snapshot_year(None, Some(358800.34), Some(405850.0)));
        snapshots.insert(2025, snapshot_year(Some(428635.41), None, None));
        let rows = yearly_rows(&trades, &[], &snapshots, Some(1316299.0), 2025);

        assert_eq!(rows.len(), 2);
        // Frozen 2024: stored cost/value override the computed figures.
        assert!(approx_eq(rows[0].cost, 358800.34));
        assert_eq!(rows[0].market_value, Some(405850.0));
        assert!(rows[0].snapshot.is_some());
        // 2025: invested override applies; cost still computes cumulatively
        // (no 2025 trades, so it carries 2024's 300373.62), and the live
        // market value fills the current year's 總市值.
        assert!(approx_eq(rows[1].invested, 428635.41));
        assert!(approx_eq(rows[1].cost, 300373.62));
        assert_eq!(rows[1].market_value, Some(1316299.0));
        // YoY uses the effective (override-aware) cost column.
        assert!(approx_eq(
            rows[1].invested_yoy.unwrap(),
            (300373.62 - 358800.34) / 358800.34
        ));
    }

    #[test]
    fn yearly_rows_dividends_count_received_only() {
        let dividends = [
            dividend_fact("中國銀行", "2025-12-16", Some(7342.08)),
            dividend_fact("港交所", "2025-10-01", None),
        ];
        let rows = yearly_rows(&[], &dividends, &HashMap::new(), None, 2025);
        assert_eq!(rows.len(), 1);
        assert!(approx_eq(rows[0].dividends, 7342.08));
    }

    // ----- MPF -----
}
