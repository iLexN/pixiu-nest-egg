//! Pure calculation core: no database, no HTTP types.
//!
//! Money is `f64` and is never rounded here; rounding belongs to display.
//! The formulas deliberately mirror the spreadsheet being replaced, including
//! 加權平均買入單價 dividing by shares *bought* rather than shares held.

use chrono::Datelike;
use serde::Serialize;

use crate::models::{InputMode, TradeType};

/// Relative tolerance used when comparing money figures.
pub const TOLERANCE: f64 = 1e-6;

pub const UNCATEGORIZED_SECTOR: &str = "未分類";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FieldError {
    pub field: String,
    pub message: String,
}

impl FieldError {
    pub fn new(field: &str, message: impl Into<String>) -> Self {
        Self {
            field: field.to_string(),
            message: message.into(),
        }
    }
}

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

fn has_note(note: Option<&str>) -> bool {
    note.map(|n| !n.trim().is_empty()).unwrap_or(false)
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
                if let Some(given) = input.fee {
                    if (given - fee).abs() > TOLERANCE {
                        errors.push(FieldError::new(
                            "fee",
                            "fee conflicts with buy total; leave fee empty so it can be derived",
                        ));
                    }
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
                if let Some(given) = input.total {
                    if (given - total).abs() > TOLERANCE {
                        errors.push(FieldError::new(
                            "total",
                            "buy total conflicts with fee; leave buy total empty so it can be derived",
                        ));
                    }
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

#[derive(Debug, Clone, PartialEq, Serialize)]
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
}

pub fn summarize(trades: &[TradeFacts], current_price: Option<f64>) -> StockSummary {
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

    StockSummary {
        shares_held,
        total_buy_cost,
        weighted_avg_buy_price,
        current_price,
        market_value,
        unrealized_amount,
        unrealized_return,
    }
}

/// One stock's contribution to the rollups.
#[derive(Debug, Clone)]
pub struct RollupInput<'a> {
    pub code: &'a str,
    pub sector: Option<&'a str>,
    pub total_buy_cost: f64,
    pub market_value: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
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

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MarketTotals {
    pub buy_cost: f64,
    pub buy_cost_priced: f64,
    pub market_value: Option<f64>,
    pub net_amount: Option<f64>,
    pub net_percent: Option<f64>,
    /// Stocks left out of `market_value`/`net_*` because they have no 現價.
    pub excluded_codes: Vec<String>,
}

pub fn market_totals(rows: &[RollupInput<'_>]) -> MarketTotals {
    let mut buy_cost = 0.0;
    let mut buy_cost_priced = 0.0;
    let mut market_value = 0.0;
    let mut priced = 0usize;
    let mut excluded_codes = Vec::new();

    for row in rows {
        buy_cost += row.total_buy_cost;
        match row.market_value {
            Some(value) => {
                buy_cost_priced += row.total_buy_cost;
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

    MarketTotals {
        buy_cost,
        buy_cost_priced,
        market_value,
        net_amount,
        net_percent,
        excluded_codes,
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

/// Relative comparison used by the parity check and the tests.
pub fn approx_eq(left: f64, right: f64) -> bool {
    let scale = left.abs().max(right.abs()).max(1.0);
    (left - right).abs() <= TOLERANCE * scale
}

/// Bucket label for deposits with no label, like 未分類 for sectors.
pub const UNLABELED_PREFIX: &str = "未分類";

/// A deposit as it arrives from the user, before validation.
#[derive(Debug, Clone)]
pub struct DepositInput<'a> {
    pub label: Option<&'a str>,
    /// Bank code (SC = 渣打, HS = 恒生); free text, new banks are allowed.
    pub bank: Option<&'a str>,
    pub principal: Option<f64>,
    pub rate: Option<f64>,
    pub interest: Option<f64>,
    pub end_date: &'a str,
}

/// A validated deposit: optional fields stay optional (interest-only and
/// label-only rows exist in the source sheet).
#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedDeposit {
    pub label: Option<String>,
    pub bank: Option<String>,
    pub principal: Option<f64>,
    pub rate: Option<f64>,
    pub interest: Option<f64>,
    pub end_date: String,
}

/// Validate user input for a deposit record.
pub fn validate_deposit(input: DepositInput<'_>) -> Result<ValidatedDeposit, Vec<FieldError>> {
    let mut errors = Vec::new();

    if chrono::NaiveDate::parse_from_str(input.end_date, "%Y-%m-%d").is_err() {
        errors.push(FieldError::new(
            "end_date",
            "end date must be a calendar date in YYYY-MM-DD form",
        ));
    }

    let label = input.label.map(str::trim).filter(|label| !label.is_empty());
    let bank = input.bank.map(str::trim).filter(|bank| !bank.is_empty());

    for (field, value, name) in [
        ("principal", input.principal, "input"),
        ("interest", input.interest, "利息"),
        ("rate", input.rate, "rate"),
    ] {
        if let Some(value) = value {
            if !value.is_finite() || value < 0.0 {
                errors.push(FieldError::new(
                    field,
                    format!("{name} must not be negative"),
                ));
            }
        }
    }
    if let Some(rate) = input.rate {
        if rate.is_finite() && rate >= 1.0 {
            errors.push(FieldError::new(
                "rate",
                "rate is stored as a fraction (0.03 = 3%) and must be less than 1",
            ));
        }
    }

    if label.is_none() && input.principal.is_none() && input.interest.is_none() {
        errors.push(FieldError::new(
            "label",
            "at least one of id, input or 利息 is required",
        ));
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    Ok(ValidatedDeposit {
        label: label.map(str::to_string),
        bank: bank.map(str::to_string),
        principal: input.principal,
        rate: input.rate,
        interest: input.interest,
        end_date: input.end_date.to_string(),
    })
}

/// The only deposit facts the rollups need.
#[derive(Debug, Clone, Copy)]
pub struct DepositFacts<'a> {
    pub bank: Option<&'a str>,
    pub principal: Option<f64>,
    pub interest: Option<f64>,
    pub end_date: chrono::NaiveDate,
}

/// The sheet's total column: `利息 + input`, blanks counting as 0.
pub fn deposit_total(principal: Option<f64>, interest: Option<f64>) -> f64 {
    principal.unwrap_or(0.0) + interest.unwrap_or(0.0)
}

/// The sheet's status rule: `End` when `TODAY() >= end date`.
pub fn deposit_active(end_date: chrono::NaiveDate, today: chrono::NaiveDate) -> bool {
    end_date > today
}

/// The label prefix used by the sheet's `SUMIF("SC-*")` bank rows: the text
/// before the first `-`, so `HS-Irene-53` lands in `HS`. Used at import time
/// to fill the `bank` column; rollups group by the column, not the label.
pub fn label_prefix(label: Option<&str>) -> Option<String> {
    let label = label?.trim();
    if label.is_empty() {
        return None;
    }
    let prefix = label.split('-').next().unwrap_or("").trim();
    if prefix.is_empty() {
        None
    } else {
        Some(prefix.to_string())
    }
}

/// One month bucket of the 定期 active-month table.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ActiveMonthBucket {
    pub year: i32,
    pub month: u32,
    /// 定期 column: Σ principal.
    pub principal: f64,
    /// 利息 column: Σ interest.
    pub interest: f64,
    /// Total column: Σ (principal + interest).
    pub total: f64,
}

/// Totals over the active deposits: `定期!B1` plus the other two sums.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ActiveTotals {
    pub principal: f64,
    pub interest: f64,
    pub total: f64,
}

/// One bank row of the 定期 rollup (sheet labels them SC, HS).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BankRollup {
    pub bank: String,
    pub principal: f64,
    pub interest: f64,
    pub total: f64,
}

/// One month row of a 定期Info year table.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct YearMonthRow {
    pub month: u32,
    /// 利息 column: Σ interest.
    pub interest: f64,
    /// 定期 column: Σ deposit total (principal + interest).
    pub payout: f64,
    /// The sheet's Total column: 利息 + 定期.
    pub total: f64,
}

/// A year's month table: always all 12 months, like the sheet.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct YearRollup {
    pub year: i32,
    pub months: Vec<YearMonthRow>,
}

/// Σ principal/interest/total over the active deposits.
pub fn active_totals(deposits: &[DepositFacts<'_>], today: chrono::NaiveDate) -> ActiveTotals {
    let mut totals = ActiveTotals {
        principal: 0.0,
        interest: 0.0,
        total: 0.0,
    };
    for deposit in deposits
        .iter()
        .filter(|deposit| deposit_active(deposit.end_date, today))
    {
        totals.principal += deposit.principal.unwrap_or(0.0);
        totals.interest += deposit.interest.unwrap_or(0.0);
        totals.total += deposit_total(deposit.principal, deposit.interest);
    }
    totals
}

/// Active deposits grouped by end (year, month), in chronological order.
/// The sheet buckets by month number only; grouping by (year, month) gives the
/// same result while the active span is under a year.
pub fn active_month_rollup(
    deposits: &[DepositFacts<'_>],
    today: chrono::NaiveDate,
) -> Vec<ActiveMonthBucket> {
    let mut buckets: Vec<ActiveMonthBucket> = Vec::new();
    for deposit in deposits
        .iter()
        .filter(|deposit| deposit_active(deposit.end_date, today))
    {
        let (year, month) = (deposit.end_date.year(), deposit.end_date.month());
        let bucket = match buckets
            .iter_mut()
            .find(|bucket| bucket.year == year && bucket.month == month)
        {
            Some(existing) => existing,
            None => {
                buckets.push(ActiveMonthBucket {
                    year,
                    month,
                    principal: 0.0,
                    interest: 0.0,
                    total: 0.0,
                });
                buckets.last_mut().expect("just pushed")
            }
        };
        bucket.principal += deposit.principal.unwrap_or(0.0);
        bucket.interest += deposit.interest.unwrap_or(0.0);
        bucket.total += deposit_total(deposit.principal, deposit.interest);
    }
    buckets.sort_by(|a, b| (a.year, a.month).cmp(&(b.year, b.month)));
    buckets
}

/// Active deposits grouped by bank, ordered by descending principal.
/// Deposits without a bank land in their own bucket, matching how the sheet's
/// `SUMIF` rows only count labelled deposits.
pub fn bank_rollup(deposits: &[DepositFacts<'_>], today: chrono::NaiveDate) -> Vec<BankRollup> {
    let mut groups: Vec<BankRollup> = Vec::new();
    for deposit in deposits
        .iter()
        .filter(|deposit| deposit_active(deposit.end_date, today))
    {
        let bank = deposit
            .bank
            .map(str::trim)
            .filter(|bank| !bank.is_empty())
            .unwrap_or(UNLABELED_PREFIX)
            .to_string();
        let group = match groups.iter_mut().find(|group| group.bank == bank) {
            Some(existing) => existing,
            None => {
                groups.push(BankRollup {
                    bank,
                    principal: 0.0,
                    interest: 0.0,
                    total: 0.0,
                });
                groups.last_mut().expect("just pushed")
            }
        };
        group.principal += deposit.principal.unwrap_or(0.0);
        group.interest += deposit.interest.unwrap_or(0.0);
        group.total += deposit_total(deposit.principal, deposit.interest);
    }
    groups.sort_by(|a, b| {
        b.principal
            .partial_cmp(&a.principal)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.bank.cmp(&b.bank))
    });
    groups
}

/// Per-year month tables over every deposit ending that year, regardless of
/// status — the sheet's `SUMIFS` take the whole list. One rollup per year
/// present in the data, ascending.
pub fn year_rollups(deposits: &[DepositFacts<'_>]) -> Vec<YearRollup> {
    let mut years: Vec<YearRollup> = Vec::new();
    for deposit in deposits {
        let year = deposit.end_date.year();
        let rollup = match years.iter_mut().find(|rollup| rollup.year == year) {
            Some(existing) => existing,
            None => {
                years.push(YearRollup {
                    year,
                    months: (1..=12)
                        .map(|month| YearMonthRow {
                            month,
                            interest: 0.0,
                            payout: 0.0,
                            total: 0.0,
                        })
                        .collect(),
                });
                years.last_mut().expect("just pushed")
            }
        };
        let month = &mut rollup.months[(deposit.end_date.month() - 1) as usize];
        month.interest += deposit.interest.unwrap_or(0.0);
        month.payout += deposit_total(deposit.principal, deposit.interest);
        month.total = month.interest + month.payout;
    }
    years.sort_by_key(|rollup| rollup.year);
    years
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buy(shares: f64, total: f64) -> TradeFacts {
        TradeFacts {
            trade_type: TradeType::Buy,
            shares,
            total,
        }
    }

    fn sell(shares: f64) -> TradeFacts {
        TradeFacts {
            trade_type: TradeType::Sell,
            shares,
            total: 0.0,
        }
    }

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
        let summary = summarize(&trades, None);
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
        let summary = summarize(&trades, None);
        assert!(approx_eq(summary.shares_held, 600.0));
        assert!(approx_eq(summary.total_buy_cost, 10030.0));
        // Sheet behaviour: divided by shares bought, not shares held.
        assert!(approx_eq(summary.weighted_avg_buy_price, 10.03));
    }

    #[test]
    fn closed_position_reports_zero_average() {
        let trades = [buy(1000.0, 10030.0), sell(1000.0)];
        let summary = summarize(&trades, Some(12.0));
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
        let summary = summarize(&trades, Some(5.91));
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
        let summary = summarize(&[buy(100.0, 1000.0)], None);
        assert_eq!(summary.market_value, None);
        assert_eq!(summary.unrealized_amount, None);
        assert_eq!(summary.unrealized_return, None);
    }

    #[test]
    fn zero_cost_basis_leaves_return_empty() {
        let summary = summarize(&[buy(100.0, 0.0)], Some(5.0));
        assert!(approx_eq(summary.market_value.unwrap(), 500.0));
        assert!(approx_eq(summary.unrealized_amount.unwrap(), 500.0));
        assert_eq!(summary.unrealized_return, None);
    }

    // --- 3.5 rollups ---

    fn rollup_input<'a>(
        code: &'a str,
        sector: Option<&'a str>,
        cost: f64,
        market_value: Option<f64>,
    ) -> RollupInput<'a> {
        RollupInput {
            code,
            sector,
            total_buy_cost: cost,
            market_value,
        }
    }

    #[test]
    fn sector_rollup_groups_multiple_stocks() {
        let rows = [
            rollup_input("港燈", Some("Utilities"), 100.0, Some(120.0)),
            rollup_input("香港中華煤氣", Some("Utilities\t"), 100.0, Some(80.0)),
            rollup_input("匯豐", Some("Banks - Diversified"), 200.0, Some(300.0)),
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
            rollup_input("VOO", None, 100.0, Some(150.0)),
            rollup_input("BE", Some("   "), 100.0, Some(50.0)),
        ];
        let groups = sector_rollup(&rows);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].sector, UNCATEGORIZED_SECTOR);
        assert!(approx_eq(groups[0].buy_cost, 200.0));
    }

    #[test]
    fn totals_exclude_stocks_without_a_price() {
        let rows = [
            rollup_input("港燈", Some("Utilities"), 100.0, Some(150.0)),
            rollup_input("港交所", Some("Financial Services"), 400.0, None),
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
        let rows = [rollup_input("港交所", None, 400.0, None)];
        let totals = market_totals(&rows);
        assert_eq!(totals.market_value, None);
        assert_eq!(totals.net_amount, None);
        assert_eq!(totals.net_percent, None);
    }

    // --- deposits ---

    fn deposit_input<'a>() -> DepositInput<'a> {
        DepositInput {
            label: Some("SC-9632"),
            bank: Some("SC"),
            principal: Some(110000.0),
            rate: Some(0.028),
            interest: Some(993.0),
            end_date: "2026-10-12",
        }
    }

    fn day(date: &str) -> chrono::NaiveDate {
        chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").expect("date")
    }

    fn deposit<'a>(
        bank: Option<&'a str>,
        principal: Option<f64>,
        interest: Option<f64>,
        end_date: &str,
    ) -> DepositFacts<'a> {
        DepositFacts {
            bank,
            principal,
            interest,
            end_date: day(end_date),
        }
    }

    #[test]
    fn deposit_validates_a_full_row() {
        let validated = validate_deposit(deposit_input()).expect("valid deposit");
        assert_eq!(validated.label.as_deref(), Some("SC-9632"));
        assert_eq!(validated.bank.as_deref(), Some("SC"));
        assert_eq!(validated.principal, Some(110000.0));
        assert_eq!(validated.end_date, "2026-10-12");
    }

    #[test]
    fn deposit_accepts_interest_only_and_label_only_rows() {
        let interest_only = DepositInput {
            label: None,
            bank: None,
            principal: None,
            rate: None,
            interest: Some(539.25),
            end_date: "2026-05-14",
        };
        assert!(validate_deposit(interest_only).is_ok());

        let label_only = DepositInput {
            label: Some("SC-4900"),
            bank: Some("SC"),
            principal: None,
            rate: None,
            interest: Some(0.0),
            end_date: "2026-06-30",
        };
        assert!(validate_deposit(label_only).is_ok());
    }

    #[test]
    fn deposit_rejects_malformed_end_date() {
        let errors = validate_deposit(DepositInput {
            end_date: "2026/13/45",
            ..deposit_input()
        })
        .expect_err("must fail");
        assert!(errors.iter().any(|e| e.field == "end_date"));
    }

    #[test]
    fn deposit_rejects_an_empty_record() {
        let errors = validate_deposit(DepositInput {
            label: None,
            bank: None,
            principal: None,
            rate: None,
            interest: None,
            end_date: "2026-10-12",
        })
        .expect_err("must fail");
        assert!(errors.iter().any(|e| e.field == "label"));
    }

    #[test]
    fn deposit_rejects_negative_amounts_and_whole_rates() {
        let errors = validate_deposit(DepositInput {
            principal: Some(-5000.0),
            ..deposit_input()
        })
        .expect_err("negative principal must fail");
        assert!(errors.iter().any(|e| e.field == "principal"));

        let errors = validate_deposit(DepositInput {
            rate: Some(3.0),
            ..deposit_input()
        })
        .expect_err("rate >= 1 must fail");
        assert!(errors.iter().any(|e| e.field == "rate"));
    }

    #[test]
    fn deposit_status_follows_the_sheets_today_rule() {
        let today = day("2026-09-15");
        assert!(deposit_active(day("2026-10-12"), today));
        assert!(!deposit_active(day("2026-09-15"), today));
        assert!(!deposit_active(day("2026-01-05"), today));
    }

    #[test]
    fn deposit_total_counts_blanks_as_zero() {
        assert!(approx_eq(deposit_total(Some(140000.0), None), 140000.0));
        assert!(approx_eq(deposit_total(None, Some(539.25)), 539.25));
        assert!(approx_eq(deposit_total(None, None), 0.0));
    }

    #[test]
    fn label_prefix_takes_the_text_before_the_first_dash() {
        assert_eq!(label_prefix(Some("SC-9632")).as_deref(), Some("SC"));
        assert_eq!(label_prefix(Some("HS-Irene-53")).as_deref(), Some("HS"));
        assert_eq!(label_prefix(None), None);
        assert_eq!(label_prefix(Some("  ")), None);
    }

    /// The workbook's current active set: SC-9632 + SC-7006 ending 2026-10-12,
    /// SC-0044 ending 2026-11-02, HS-88 ending 2026-11-17, SC-3615 ending
    /// 2027-01-04, plus ended and interest-only rows that must not appear.
    fn workbook_deposits<'a>() -> Vec<DepositFacts<'a>> {
        vec![
            deposit(Some("HS"), Some(60000.0), Some(362.96), "2026-01-05"),
            deposit(None, None, Some(15.27), "2026-04-20"),
            deposit(Some("SC"), Some(110000.0), Some(993.0), "2026-10-12"),
            deposit(Some("SC"), Some(35000.0), Some(285.0), "2026-10-12"),
            deposit(Some("SC"), Some(140000.0), None, "2026-11-02"),
            deposit(Some("HS"), Some(80000.0), Some(604.93), "2026-11-17"),
            deposit(None, None, Some(2097.0), "2026-08-18"),
            deposit(Some("SC"), Some(80000.0), Some(807.0), "2027-01-04"),
        ]
    }

    #[test]
    fn active_totals_match_the_workbook() {
        let totals = active_totals(&workbook_deposits(), day("2026-09-15"));
        assert!(approx_eq(totals.principal, 445000.0));
        assert!(approx_eq(totals.interest, 2689.93));
        assert!(approx_eq(totals.total, 447689.93));
    }

    #[test]
    fn active_month_rollup_matches_the_workbook() {
        let months = active_month_rollup(&workbook_deposits(), day("2026-09-15"));
        assert_eq!(months.len(), 3);
        assert_eq!((months[0].year, months[0].month), (2026, 10));
        assert!(approx_eq(months[0].principal, 145000.0));
        assert!(approx_eq(months[0].interest, 1278.0));
        assert!(approx_eq(months[0].total, 146278.0));
        assert_eq!((months[1].year, months[1].month), (2026, 11));
        assert!(approx_eq(months[1].principal, 220000.0));
        assert_eq!((months[2].year, months[2].month), (2027, 1));
        assert!(approx_eq(months[2].principal, 80000.0));
    }

    #[test]
    fn bank_rollup_matches_the_workbook() {
        let banks = bank_rollup(&workbook_deposits(), day("2026-09-15"));
        assert_eq!(banks.len(), 2);
        assert_eq!(banks[0].bank, "SC");
        assert!(approx_eq(banks[0].principal, 365000.0));
        assert!(approx_eq(banks[0].interest, 2085.0));
        assert!(approx_eq(banks[0].total, 367085.0));
        assert_eq!(banks[1].bank, "HS");
        assert!(approx_eq(banks[1].principal, 80000.0));
    }

    #[test]
    fn year_rollups_cover_every_deposit_regardless_of_status() {
        let years = year_rollups(&workbook_deposits());
        assert_eq!(years.len(), 2);
        let y2026 = &years[0];
        assert_eq!(y2026.year, 2026);
        assert_eq!(y2026.months.len(), 12);
        // August 2026 holds the unlabeled 2097 interest-only row.
        assert!(approx_eq(y2026.months[7].interest, 2097.0));
        assert!(approx_eq(y2026.months[7].payout, 2097.0));
        let y2027 = &years[1];
        // 表_2027定期 1月: 利息 807, 定期 80807, Total 81614.
        assert!(approx_eq(y2027.months[0].interest, 807.0));
        assert!(approx_eq(y2027.months[0].payout, 80807.0));
        assert!(approx_eq(y2027.months[0].total, 81614.0));
    }

    // --- 3.6 validation ---

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
}
