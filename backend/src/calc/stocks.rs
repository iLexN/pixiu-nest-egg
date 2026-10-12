use super::*;

use serde::Serialize;
use utoipa::ToSchema;

use crate::models::{InputMode, TradeType};

pub const UNCATEGORIZED_SECTOR: &str = "未分類";

/// A trade as it arrives from the user, before validation.
#[derive(Debug, Clone)]
pub struct TradeInput<'a> {
    pub trade_type: &'a str,
    pub trade_date: &'a str,
    pub shares: f64,
    pub unit_price: f64,
    /// HK input: buy total including fee.
    pub total: Option<f64>,
    /// US input: fee on top of shares * unit price.
    pub fee: Option<f64>,
    pub input_mode: InputMode,
    pub note: Option<&'a str>,
}

/// A validated trade with both money figures resolved.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedTrade {
    pub trade_type: TradeType,
    pub trade_date: String,
    pub shares: f64,
    pub unit_price: f64,
    pub fee: f64,
    pub total: f64,
    pub input_mode: InputMode,
}

/// HK rule: `fee = total − shares × unit_price`.
pub fn derive_fee(shares: f64, unit_price: f64, total: f64) -> f64 {
    total - shares * unit_price
}

/// US rule: `total = shares × unit_price + fee`.
pub fn derive_total(shares: f64, unit_price: f64, fee: f64) -> f64 {
    shares * unit_price + fee
}

/// 平均單價 for a single trade: total (fee included) per share.
pub fn unit_price_incl_fee(total: f64, shares: f64) -> Option<f64> {
    if shares == 0.0 {
        None
    } else {
        Some(total / shares)
    }
}

/// Validate user input and resolve fee/total per the trade's input mode.
pub fn validate_trade(input: TradeInput<'_>) -> Result<ValidatedTrade, Vec<FieldError>> {
    let mut errors = Vec::new();

    let trade_type = TradeType::parse(input.trade_type);
    if trade_type.is_none() {
        errors.push(FieldError::new(
            "trade_type",
            "類別 must be either BUY or SELL",
        ));
    }

    if chrono::NaiveDate::parse_from_str(input.trade_date, "%Y-%m-%d").is_err() {
        errors.push(FieldError::new(
            "trade_date",
            "日期 must be a calendar date in YYYY-MM-DD form",
        ));
    }

    if !input.shares.is_finite() || input.shares < 0.0 {
        errors.push(FieldError::new("shares", "股數 must not be negative"));
    }
    if !input.unit_price.is_finite() || input.unit_price < 0.0 {
        errors.push(FieldError::new("unit_price", "單價 must not be negative"));
    }
    if input.shares == 0.0 && !has_note(input.note) {
        errors.push(FieldError::new(
            "shares",
            "股數 0 is only accepted with a note explaining the adjustment",
        ));
    }

    let (fee, total) = match input.input_mode {
        InputMode::HkTotal => match input.total {
            None => {
                errors.push(FieldError::new("total", "buy total is required"));
                (0.0, 0.0)
            }
            Some(total) if !total.is_finite() => {
                errors.push(FieldError::new("total", "buy total must be a number"));
                (0.0, 0.0)
            }
            Some(total) => {
                let fee = derive_fee(input.shares, input.unit_price, total);
                if let Some(given) = input.fee
                    && (given - fee).abs() > TOLERANCE
                {
                    errors.push(FieldError::new(
                        "fee",
                        "fee conflicts with buy total; leave fee empty so it can be derived",
                    ));
                }
                (fee, total)
            }
        },
        InputMode::UsFee => match input.fee {
            None => {
                errors.push(FieldError::new("fee", "fee is required"));
                (0.0, 0.0)
            }
            Some(fee) if !fee.is_finite() => {
                errors.push(FieldError::new("fee", "fee must be a number"));
                (0.0, 0.0)
            }
            Some(fee) => {
                let total = derive_total(input.shares, input.unit_price, fee);
                if let Some(given) = input.total
                    && (given - total).abs() > TOLERANCE
                {
                    errors.push(FieldError::new(
                        "total",
                        "buy total conflicts with fee; leave buy total empty so it can be derived",
                    ));
                }
                (fee, total)
            }
        },
    };

    if fee < -TOLERANCE && !has_note(input.note) {
        errors.push(FieldError::new(
            "fee",
            "buy total is below 股數 × 單價, implying a negative fee; add a note to record the adjustment",
        ));
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    Ok(ValidatedTrade {
        trade_type: trade_type.expect("checked above"),
        trade_date: input.trade_date.to_string(),
        shares: input.shares,
        unit_price: input.unit_price,
        fee,
        total,
        input_mode: input.input_mode,
    })
}

/// The only trade facts the summary needs.
#[derive(Debug, Clone, Copy)]
pub struct TradeFacts {
    pub trade_type: TradeType,
    pub shares: f64,
    pub total: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
pub struct StockSummary {
    /// ΣBUY 股數 − ΣSELL 股數
    pub shares_held: f64,
    /// Σ BUY total, fees included
    pub total_buy_cost: f64,
    /// 總買入成本 ÷ Σ BUY 股數 (shares bought, not held), 0 when nothing held
    pub weighted_avg_buy_price: f64,
    pub current_price: Option<f64>,
    pub market_value: Option<f64>,
    pub unrealized_amount: Option<f64>,
    pub unrealized_return: Option<f64>,
    /// 累計派息: Σ received dividend amounts; pending estimates excluded.
    pub dividends_received: f64,
    /// 累計派息% = 累計派息 ÷ 總買入成本; empty when cost is 0.
    pub dividend_return: Option<f64>,
    /// 淨投入總本金 = 總買入成本 − 累計派息.
    pub net_invested: f64,
    /// 淨攤薄單價 = 淨投入總本金 ÷ 股數 held; empty when held is 0.
    pub net_diluted_price: Option<f64>,
    /// 實質動態總回報% = (當前總市值 − 淨投入總本金) ÷ 淨投入總本金;
    /// empty without a market value or when 淨投入總本金 is 0.
    pub real_total_return: Option<f64>,
}

/// `dividend_total` is the stock's Σ received 派息 — money actually paid out.
pub fn summarize(
    trades: &[TradeFacts],
    current_price: Option<f64>,
    dividend_total: f64,
) -> StockSummary {
    let mut bought_shares = 0.0;
    let mut sold_shares = 0.0;
    let mut total_buy_cost = 0.0;
    for trade in trades {
        match trade.trade_type {
            TradeType::Buy => {
                bought_shares += trade.shares;
                total_buy_cost += trade.total;
            }
            TradeType::Sell => sold_shares += trade.shares,
        }
    }
    let shares_held = bought_shares - sold_shares;

    let weighted_avg_buy_price = if shares_held > 0.0 && bought_shares != 0.0 {
        total_buy_cost / bought_shares
    } else {
        0.0
    };

    let market_value = match current_price {
        Some(price) if shares_held > 0.0 => Some(price * shares_held),
        _ => None,
    };
    let unrealized_amount = market_value.map(|value| value - total_buy_cost);
    let unrealized_return = match unrealized_amount {
        Some(amount) if total_buy_cost != 0.0 => Some(amount / total_buy_cost),
        _ => None,
    };

    let dividend_return = if total_buy_cost != 0.0 {
        Some(dividend_total / total_buy_cost)
    } else {
        None
    };
    let net_invested = total_buy_cost - dividend_total;
    let net_diluted_price = if shares_held != 0.0 {
        Some(net_invested / shares_held)
    } else {
        None
    };
    let real_total_return = match market_value {
        Some(value) if net_invested != 0.0 => Some((value - net_invested) / net_invested),
        _ => None,
    };

    StockSummary {
        shares_held,
        total_buy_cost,
        weighted_avg_buy_price,
        current_price,
        market_value,
        unrealized_amount,
        unrealized_return,
        dividends_received: dividend_total,
        dividend_return,
        net_invested,
        net_diluted_price,
        real_total_return,
    }
}

/// One stock's contribution to the rollups.
#[derive(Debug, Clone)]
pub struct RollupInput<'a> {
    pub code: &'a str,
    pub sector: Option<&'a str>,
    pub total_buy_cost: f64,
    pub market_value: Option<f64>,
    /// 累計派息: Σ received dividend amounts.
    pub dividends_received: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
pub struct SectorRollup {
    pub sector: String,
    /// Buy cost of every stock in the sector, priced or not.
    pub buy_cost: f64,
    /// Buy cost of the stocks that have a 現價, i.e. the comparable subset.
    pub buy_cost_priced: f64,
    pub market_value: Option<f64>,
    pub share_of_market_value: Option<f64>,
    pub percent_change: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
pub struct MarketTotals {
    pub buy_cost: f64,
    pub buy_cost_priced: f64,
    pub market_value: Option<f64>,
    pub net_amount: Option<f64>,
    pub net_percent: Option<f64>,
    /// Stocks left out of `market_value`/`net_*` because they have no 現價.
    pub excluded_codes: Vec<String>,
    /// Σ 累計派息 across all stocks in the market.
    pub dividends_received: f64,
    /// Σ 累計派息 ÷ Σ 總買入成本; empty when `buy_cost` is 0.
    pub dividend_return: Option<f64>,
    /// Σ 淨投入總本金 across all stocks: `buy_cost − dividends_received`.
    pub net_invested: f64,
    /// Σ 淨投入總本金 of the priced subset, mirroring `buy_cost_priced`.
    pub net_invested_priced: f64,
    /// (market_value − net_invested_priced) ÷ net_invested_priced;
    /// empty when nothing is priced or the denominator is 0.
    pub real_total_return: Option<f64>,
}

pub fn market_totals(rows: &[RollupInput<'_>]) -> MarketTotals {
    let mut buy_cost = 0.0;
    let mut buy_cost_priced = 0.0;
    let mut market_value = 0.0;
    let mut priced = 0usize;
    let mut excluded_codes = Vec::new();
    let mut dividends_received = 0.0;
    let mut net_invested_priced = 0.0;

    for row in rows {
        buy_cost += row.total_buy_cost;
        dividends_received += row.dividends_received;
        match row.market_value {
            Some(value) => {
                buy_cost_priced += row.total_buy_cost;
                net_invested_priced += row.total_buy_cost - row.dividends_received;
                market_value += value;
                priced += 1;
            }
            None => excluded_codes.push(row.code.to_string()),
        }
    }

    let market_value = if priced == 0 {
        None
    } else {
        Some(market_value)
    };
    let net_amount = market_value.map(|value| value - buy_cost_priced);
    let net_percent = match net_amount {
        Some(amount) if buy_cost_priced != 0.0 => Some(amount / buy_cost_priced),
        _ => None,
    };
    let real_total_return = match market_value {
        Some(value) if net_invested_priced != 0.0 => {
            Some((value - net_invested_priced) / net_invested_priced)
        }
        _ => None,
    };
    let dividend_return = if buy_cost != 0.0 {
        Some(dividends_received / buy_cost)
    } else {
        None
    };

    MarketTotals {
        buy_cost,
        buy_cost_priced,
        market_value,
        net_amount,
        net_percent,
        excluded_codes,
        dividends_received,
        dividend_return,
        net_invested: buy_cost - dividends_received,
        net_invested_priced,
        real_total_return,
    }
}

/// Sector rows ordered by descending market value, then sector name, so the
/// output is stable across runs.
pub fn sector_rollup(rows: &[RollupInput<'_>]) -> Vec<SectorRollup> {
    let totals = market_totals(rows);
    let mut groups: Vec<SectorRollup> = Vec::new();

    for row in rows {
        let sector = row
            .sector
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(UNCATEGORIZED_SECTOR)
            .to_string();

        let entry = match groups.iter_mut().find(|g| g.sector == sector) {
            Some(existing) => existing,
            None => {
                groups.push(SectorRollup {
                    sector,
                    buy_cost: 0.0,
                    buy_cost_priced: 0.0,
                    market_value: None,
                    share_of_market_value: None,
                    percent_change: None,
                });
                groups.last_mut().expect("just pushed")
            }
        };

        entry.buy_cost += row.total_buy_cost;
        if let Some(value) = row.market_value {
            entry.buy_cost_priced += row.total_buy_cost;
            entry.market_value = Some(entry.market_value.unwrap_or(0.0) + value);
        }
    }

    for group in groups.iter_mut() {
        group.percent_change = match group.market_value {
            Some(value) if group.buy_cost_priced != 0.0 => {
                Some((value - group.buy_cost_priced) / group.buy_cost_priced)
            }
            _ => None,
        };
        group.share_of_market_value = match (group.market_value, totals.market_value) {
            (Some(value), Some(total)) if total != 0.0 => Some(value / total),
            _ => None,
        };
    }

    groups.sort_by(|a, b| {
        b.market_value
            .unwrap_or(f64::MIN)
            .partial_cmp(&a.market_value.unwrap_or(f64::MIN))
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.sector.cmp(&b.sector))
    });
    groups
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calc::test_helpers::*;

    fn base_input<'a>(mode: InputMode) -> TradeInput<'a> {
        TradeInput {
            trade_type: "BUY",
            trade_date: "2026-01-02",
            shares: 100.0,
            unit_price: 10.0,
            total: None,
            fee: None,
            input_mode: mode,
            note: None,
        }
    }

    // --- 3.1 derivation ---

    #[test]
    fn hk_input_derives_fee_from_buy_total() {
        // 中國銀行 2023-05-23: 30000 shares @ 3.2, buy total 96523.15 -> fee 523.15
        let validated = validate_trade(TradeInput {
            trade_type: "BUY",
            trade_date: "2023-05-23",
            shares: 30000.0,
            unit_price: 3.2,
            total: Some(96523.15),
            fee: None,
            input_mode: InputMode::HkTotal,
            note: None,
        })
        .expect("valid HK trade");
        assert!(approx_eq(validated.fee, 523.15), "fee = {}", validated.fee);
        assert!(approx_eq(validated.total, 96523.15));
    }

    #[test]
    fn us_input_derives_total_from_fee() {
        // VOO 2026-06-02: 3 shares @ 696.04 + fee 1.000009 -> total 2089.120009
        let validated = validate_trade(TradeInput {
            trade_type: "BUY",
            trade_date: "2026-06-02",
            shares: 3.0,
            unit_price: 696.04,
            total: None,
            fee: Some(1.000009),
            input_mode: InputMode::UsFee,
            note: None,
        })
        .expect("valid US trade");
        assert!(
            approx_eq(validated.total, 2089.120009),
            "total = {}",
            validated.total
        );
        assert!(approx_eq(validated.fee, 1.000009));
    }

    #[test]
    fn us_input_accepts_fractional_shares() {
        let validated = validate_trade(TradeInput {
            trade_type: "BUY",
            trade_date: "2026-06-02",
            shares: 0.5,
            unit_price: 700.0,
            total: None,
            fee: Some(0.35),
            input_mode: InputMode::UsFee,
            note: None,
        })
        .expect("fractional shares are valid");
        assert!(approx_eq(validated.total, 350.35));
    }

    // --- 3.2 per-trade 平均單價 ---

    #[test]
    fn per_trade_unit_price_includes_fee() {
        let value = unit_price_incl_fee(96523.15, 30000.0).expect("has shares");
        assert!(approx_eq(value, 3.217438333333), "value = {value}");
    }

    #[test]
    fn per_trade_unit_price_is_empty_for_zero_shares() {
        assert_eq!(unit_price_incl_fee(-0.01, 0.0), None);
    }

    // --- 3.3 per-stock summary ---

    #[test]
    fn summary_matches_sheet_for_boc() {
        // 中國銀行: three buys totalling 68000 shares / 208746.64
        let trades = [
            buy(30000.0, 96523.15),
            buy(18000.0, 53731.5),
            buy(20000.0, 58491.99),
        ];
        let summary = summarize(&trades, None, 0.0);
        assert!(approx_eq(summary.shares_held, 68000.0));
        assert!(approx_eq(summary.total_buy_cost, 208746.64));
        assert!(
            approx_eq(summary.weighted_avg_buy_price, 3.069803529),
            "avg = {}",
            summary.weighted_avg_buy_price
        );
    }

    #[test]
    fn sell_reduces_holdings_but_not_average_or_cost() {
        let trades = [buy(1000.0, 10030.0), sell(400.0)];
        let summary = summarize(&trades, None, 0.0);
        assert!(approx_eq(summary.shares_held, 600.0));
        assert!(approx_eq(summary.total_buy_cost, 10030.0));
        // Sheet behaviour: divided by shares bought, not shares held.
        assert!(approx_eq(summary.weighted_avg_buy_price, 10.03));
    }

    #[test]
    fn closed_position_reports_zero_average() {
        let trades = [buy(1000.0, 10030.0), sell(1000.0)];
        let summary = summarize(&trades, Some(12.0), 0.0);
        assert!(approx_eq(summary.shares_held, 0.0));
        assert_eq!(summary.weighted_avg_buy_price, 0.0);
        assert_eq!(summary.market_value, None);
        assert_eq!(summary.unrealized_amount, None);
        assert_eq!(summary.unrealized_return, None);
    }

    // --- 3.4 unrealized figures ---

    #[test]
    fn unrealized_figures_match_sheet_for_boc() {
        let trades = [
            buy(30000.0, 96523.15),
            buy(18000.0, 53731.5),
            buy(20000.0, 58491.99),
        ];
        let summary = summarize(&trades, Some(5.91), 0.0);
        assert!(approx_eq(summary.market_value.unwrap(), 401880.0));
        assert!(approx_eq(summary.unrealized_amount.unwrap(), 193133.36));
        assert!(
            approx_eq(summary.unrealized_return.unwrap(), 0.9252046404),
            "return = {:?}",
            summary.unrealized_return
        );
    }

    #[test]
    fn missing_price_leaves_unrealized_figures_empty() {
        let summary = summarize(&[buy(100.0, 1000.0)], None, 0.0);
        assert_eq!(summary.market_value, None);
        assert_eq!(summary.unrealized_amount, None);
        assert_eq!(summary.unrealized_return, None);
    }

    #[test]
    fn zero_cost_basis_leaves_return_empty() {
        let summary = summarize(&[buy(100.0, 0.0)], Some(5.0), 0.0);
        assert!(approx_eq(summary.market_value.unwrap(), 500.0));
        assert!(approx_eq(summary.unrealized_amount.unwrap(), 500.0));
        assert_eq!(summary.unrealized_return, None);
    }

    // --- 3.4b dividend-adjusted net position ---

    #[test]
    fn net_position_matches_sheet_for_boc() {
        // 中國銀行: 總買入成本 208746.64, 68000 held, 現價 5.91, 累計派息 54023.87.
        let trades = [
            buy(30000.0, 96523.15),
            buy(18000.0, 53731.5),
            buy(20000.0, 58491.99),
        ];
        let summary = summarize(&trades, Some(5.91), 54023.87);
        assert!(approx_eq(summary.dividends_received, 54023.87));
        assert!(
            approx_eq(summary.dividend_return.unwrap(), 0.2588011476),
            "yield = {:?}",
            summary.dividend_return
        );
        assert!(approx_eq(summary.net_invested, 154722.77));
        assert!(
            approx_eq(summary.net_diluted_price.unwrap(), 2.275334853),
            "diluted = {:?}",
            summary.net_diluted_price
        );
        assert!(
            approx_eq(summary.real_total_return.unwrap(), 1.597419593),
            "return = {:?}",
            summary.real_total_return
        );
    }

    #[test]
    fn no_dividends_leaves_net_invested_equal_to_cost() {
        let summary = summarize(&[buy(1000.0, 10030.0)], Some(12.0), 0.0);
        assert!(approx_eq(summary.dividends_received, 0.0));
        assert!(approx_eq(summary.dividend_return.unwrap(), 0.0));
        assert!(approx_eq(summary.net_invested, 10030.0));
        assert!(approx_eq(summary.net_diluted_price.unwrap(), 10.03));
        assert!(approx_eq(
            summary.real_total_return.unwrap(),
            1970.0 / 10030.0
        ));
    }

    #[test]
    fn closed_position_leaves_diluted_price_and_return_empty() {
        let trades = [buy(1000.0, 10030.0), sell(1000.0)];
        let summary = summarize(&trades, Some(12.0), 500.0);
        assert!(approx_eq(summary.net_invested, 9530.0));
        assert_eq!(summary.net_diluted_price, None);
        assert_eq!(summary.real_total_return, None);
    }

    #[test]
    fn zero_cost_basis_leaves_dividend_return_empty() {
        let summary = summarize(&[buy(100.0, 0.0)], Some(5.0), 0.0);
        assert_eq!(summary.dividend_return, None);
        assert_eq!(summary.real_total_return, None);
    }

    #[test]
    fn missing_price_leaves_real_total_return_empty() {
        let summary = summarize(&[buy(100.0, 1000.0)], None, 200.0);
        assert!(approx_eq(summary.net_invested, 800.0));
        assert!(approx_eq(summary.net_diluted_price.unwrap(), 8.0));
        assert_eq!(summary.real_total_return, None);
    }

    // --- 3.5 rollups ---

    fn rollup_input<'a>(
        code: &'a str,
        sector: Option<&'a str>,
        cost: f64,
        market_value: Option<f64>,
        dividends: f64,
    ) -> RollupInput<'a> {
        RollupInput {
            code,
            sector,
            total_buy_cost: cost,
            market_value,
            dividends_received: dividends,
        }
    }

    #[test]
    fn sector_rollup_groups_multiple_stocks() {
        let rows = [
            rollup_input("港燈", Some("Utilities"), 100.0, Some(120.0), 0.0),
            rollup_input("香港中華煤氣", Some("Utilities\t"), 100.0, Some(80.0), 0.0),
            rollup_input("匯豐", Some("Banks - Diversified"), 200.0, Some(300.0), 0.0),
        ];
        let groups = sector_rollup(&rows);
        assert_eq!(groups.len(), 2);
        let utilities = groups
            .iter()
            .find(|g| g.sector == "Utilities")
            .expect("utilities group");
        assert!(approx_eq(utilities.buy_cost, 200.0));
        assert!(approx_eq(utilities.market_value.unwrap(), 200.0));
        assert!(approx_eq(utilities.percent_change.unwrap(), 0.0));
        assert!(approx_eq(utilities.share_of_market_value.unwrap(), 0.4));
    }

    #[test]
    fn stock_without_sector_lands_in_uncategorized() {
        let rows = [
            rollup_input("VOO", None, 100.0, Some(150.0), 0.0),
            rollup_input("BE", Some("   "), 100.0, Some(50.0), 0.0),
        ];
        let groups = sector_rollup(&rows);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].sector, UNCATEGORIZED_SECTOR);
        assert!(approx_eq(groups[0].buy_cost, 200.0));
    }

    #[test]
    fn totals_exclude_stocks_without_a_price() {
        let rows = [
            rollup_input("港燈", Some("Utilities"), 100.0, Some(150.0), 0.0),
            rollup_input("港交所", Some("Financial Services"), 400.0, None, 0.0),
        ];
        let totals = market_totals(&rows);
        assert!(approx_eq(totals.buy_cost, 500.0));
        assert!(approx_eq(totals.buy_cost_priced, 100.0));
        assert!(approx_eq(totals.market_value.unwrap(), 150.0));
        assert!(approx_eq(totals.net_amount.unwrap(), 50.0));
        assert!(approx_eq(totals.net_percent.unwrap(), 0.5));
        assert_eq!(totals.excluded_codes, vec!["港交所".to_string()]);
    }

    #[test]
    fn totals_without_any_price_have_no_market_value() {
        let rows = [rollup_input("港交所", None, 400.0, None, 0.0)];
        let totals = market_totals(&rows);
        assert_eq!(totals.market_value, None);
        assert_eq!(totals.net_amount, None);
        assert_eq!(totals.net_percent, None);
    }

    #[test]
    fn totals_aggregate_dividend_adjusted_figures() {
        let rows = [
            rollup_input("中國銀行", None, 208746.64, Some(401880.0), 54023.87),
            rollup_input("港交所", None, 400.0, None, 10.0),
        ];
        let totals = market_totals(&rows);
        assert!(approx_eq(totals.dividends_received, 54033.87));
        assert!(approx_eq(
            totals.dividend_return.unwrap(),
            54033.87 / 209146.64
        ));
        assert!(approx_eq(totals.net_invested, 209146.64 - 54033.87));
        // Only the priced stock's net invested feeds the aggregate return.
        assert!(approx_eq(totals.net_invested_priced, 154722.77));
        assert!(
            approx_eq(totals.real_total_return.unwrap(), 1.597419593),
            "return = {:?}",
            totals.real_total_return
        );
    }

    // --- deposits ---

    #[test]
    fn rejects_malformed_date() {
        let mut input = base_input(InputMode::UsFee);
        input.trade_date = "2026/13/45";
        input.fee = Some(1.0);
        let errors = validate_trade(input).expect_err("must fail");
        assert!(errors.iter().any(|e| e.field == "trade_date"));
    }

    #[test]
    fn rejects_unknown_trade_type() {
        let mut input = base_input(InputMode::UsFee);
        input.trade_type = "TRANSFER";
        input.fee = Some(1.0);
        let errors = validate_trade(input).expect_err("must fail");
        assert!(errors.iter().any(|e| e.field == "trade_type"));
    }

    #[test]
    fn rejects_negative_shares_and_price() {
        let mut input = base_input(InputMode::UsFee);
        input.shares = -1.0;
        input.unit_price = -2.0;
        input.fee = Some(1.0);
        let errors = validate_trade(input).expect_err("must fail");
        assert!(errors.iter().any(|e| e.field == "shares"));
        assert!(errors.iter().any(|e| e.field == "unit_price"));
    }

    #[test]
    fn rejects_negative_fee_without_note() {
        let mut input = base_input(InputMode::HkTotal);
        input.total = Some(900.0); // below 100 * 10
        let errors = validate_trade(input).expect_err("must fail");
        assert!(errors.iter().any(|e| e.field == "fee"));
    }

    #[test]
    fn accepts_negative_fee_with_note() {
        let mut input = base_input(InputMode::HkTotal);
        input.total = Some(900.0);
        input.note = Some("broker rebate");
        let validated = validate_trade(input).expect("note explains the adjustment");
        assert!(approx_eq(validated.fee, -100.0));
    }

    #[test]
    fn rejects_zero_shares_without_note() {
        let mut input = base_input(InputMode::HkTotal);
        input.shares = 0.0;
        input.total = Some(-0.01);
        let errors = validate_trade(input).expect_err("must fail");
        assert!(errors.iter().any(|e| e.field == "shares"));
    }

    #[test]
    fn accepts_zero_share_adjustment_with_note() {
        let mut input = base_input(InputMode::HkTotal);
        input.shares = 0.0;
        input.total = Some(-0.01);
        input.note = Some("Adjustment");
        let validated = validate_trade(input).expect("adjustment row is allowed");
        assert!(approx_eq(validated.total, -0.01));
    }

    #[test]
    fn rejects_missing_money_field_for_mode() {
        let hk = validate_trade(base_input(InputMode::HkTotal)).expect_err("total required");
        assert!(hk.iter().any(|e| e.field == "total"));
        let us = validate_trade(base_input(InputMode::UsFee)).expect_err("fee required");
        assert!(us.iter().any(|e| e.field == "fee"));
    }

    #[test]
    fn rejects_conflicting_money_fields() {
        let mut input = base_input(InputMode::HkTotal);
        input.total = Some(1005.0);
        input.fee = Some(50.0); // derived fee is 5.0
        let errors = validate_trade(input).expect_err("must fail");
        assert!(errors.iter().any(|e| e.field == "fee"));
    }

    // --- dividends ---
}
