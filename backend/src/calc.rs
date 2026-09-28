//! Pure calculation core: no database, no HTTP types.
//!
//! Money is `f64` and is never rounded here; rounding belongs to display.
//! The formulas deliberately mirror the spreadsheet being replaced, including
//! 加權平均買入單價 dividing by shares *bought* rather than shares held.

use std::collections::{BTreeMap, HashMap, HashSet};

use chrono::Datelike;
use serde::Serialize;
use utoipa::ToSchema;

use crate::models::{
    CouponStatus, InputMode, InterestComponent, MonthItemCategory, MonthSuggestion, MpfFigures,
    TradeType,
};

/// Relative tolerance used when comparing money figures.
pub const TOLERANCE: f64 = 1e-6;

pub const UNCATEGORIZED_SECTOR: &str = "未分類";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
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
    /// When the principal left the bank account; optional.
    pub start_date: Option<&'a str>,
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
    pub start_date: Option<String>,
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

    let start_date = input
        .start_date
        .map(str::trim)
        .filter(|start_date| !start_date.is_empty());
    if let Some(start_date) = start_date
        && chrono::NaiveDate::parse_from_str(start_date, "%Y-%m-%d").is_err()
    {
        errors.push(FieldError::new(
            "start_date",
            "start date must be a calendar date in YYYY-MM-DD form",
        ));
    }

    let label = input.label.map(str::trim).filter(|label| !label.is_empty());
    let bank = input.bank.map(str::trim).filter(|bank| !bank.is_empty());

    for (field, value, name) in [
        ("principal", input.principal, "input"),
        ("interest", input.interest, "利息"),
        ("rate", input.rate, "rate"),
    ] {
        if let Some(value) = value
            && (!value.is_finite() || value < 0.0)
        {
            errors.push(FieldError::new(
                field,
                format!("{name} must not be negative"),
            ));
        }
    }
    if let Some(rate) = input.rate
        && rate.is_finite()
        && rate >= 1.0
    {
        errors.push(FieldError::new(
            "rate",
            "rate is stored as a fraction (0.03 = 3%) and must be less than 1",
        ));
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
        start_date: start_date.map(str::to_string),
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
#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
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
#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
pub struct ActiveTotals {
    pub principal: f64,
    pub interest: f64,
    pub total: f64,
}

/// One bank row of the 定期 rollup (sheet labels them SC, HS).
#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
pub struct BankRollup {
    pub bank: String,
    pub principal: f64,
    pub interest: f64,
    pub total: f64,
}

/// One month row of a 定期Info year table.
#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
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
#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
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
    buckets.sort_by_key(|a| (a.year, a.month));
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

// --- dividends (派息) ---

/// A dividend as it arrives from the user, before validation. Snapshots may
/// be pre-derived by the caller or supplied explicitly.
#[derive(Debug, Clone)]
pub struct DividendInput<'a> {
    pub pay_date: &'a str,
    pub per_share: Option<f64>,
    pub shares_held: Option<f64>,
    pub buy_cost: Option<f64>,
    pub estimated_amount: Option<f64>,
    pub received_amount: Option<f64>,
    pub received_price: Option<f64>,
}

/// A validated dividend. When only `per_share` was given, `estimated_amount`
/// is filled in as `per_share × shares_held` here; when no `per_share` was
/// given, it is implied as the amount (received preferred) ÷ `shares_held`.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedDividend {
    pub pay_date: String,
    pub per_share: Option<f64>,
    pub shares_held: Option<f64>,
    pub buy_cost: Option<f64>,
    pub estimated_amount: Option<f64>,
    pub received_amount: Option<f64>,
    pub received_price: Option<f64>,
}

/// Validate user input for a dividend record.
pub fn validate_dividend(input: DividendInput<'_>) -> Result<ValidatedDividend, Vec<FieldError>> {
    let mut errors = Vec::new();

    if chrono::NaiveDate::parse_from_str(input.pay_date, "%Y-%m-%d").is_err() {
        errors.push(FieldError::new(
            "pay_date",
            "派息日 must be a calendar date in YYYY-MM-DD form",
        ));
    }
    for (field, value, name) in [
        ("per_share", input.per_share, "每股派息"),
        ("shares_held", input.shares_held, "股數"),
        ("buy_cost", input.buy_cost, "總買入成本"),
        ("estimated_amount", input.estimated_amount, "預期派息"),
        ("received_amount", input.received_amount, "實收派息"),
        ("received_price", input.received_price, "現價"),
    ] {
        if let Some(value) = value
            && (!value.is_finite() || value < 0.0)
        {
            errors.push(FieldError::new(
                field,
                format!("{name} must not be negative"),
            ));
        }
    }

    if input.per_share.is_none()
        && input.estimated_amount.is_none()
        && input.received_amount.is_none()
    {
        errors.push(FieldError::new(
            "estimated_amount",
            "at least one of 每股派息, 預期派息 or 實收派息 is required",
        ));
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    // Per-share-only input derives the estimate from the shares snapshot,
    // and estimate-only input implies the per-share figure back.
    let estimated_amount = input
        .estimated_amount
        .or(match (input.per_share, input.shares_held) {
            (Some(per_share), Some(shares)) => Some(per_share * shares),
            _ => None,
        });
    let per_share = input.per_share.or(
        match (
            input.received_amount.or(input.estimated_amount),
            input.shares_held,
        ) {
            (Some(amount), Some(shares)) if shares > 0.0 => Some(amount / shares),
            _ => None,
        },
    );

    Ok(ValidatedDividend {
        pay_date: input.pay_date.to_string(),
        per_share,
        shares_held: input.shares_held,
        buy_cost: input.buy_cost,
        estimated_amount,
        received_amount: input.received_amount,
        received_price: input.received_price,
    })
}

/// 股數 and 總買入成本 as of a date: only trades on or before it count, so a
/// recorded dividend's denominators stay frozen when more shares are bought.
pub fn holdings_snapshot(
    trades: &[(chrono::NaiveDate, TradeFacts)],
    as_of: chrono::NaiveDate,
) -> (f64, f64) {
    let mut bought_shares = 0.0;
    let mut sold_shares = 0.0;
    let mut buy_cost = 0.0;
    for (trade_date, trade) in trades {
        if *trade_date > as_of {
            continue;
        }
        match trade.trade_type {
            TradeType::Buy => {
                bought_shares += trade.shares;
                buy_cost += trade.total;
            }
            TradeType::Sell => sold_shares += trade.shares,
        }
    }
    (bought_shares - sold_shares, buy_cost)
}

/// The figure the yields divide: the final amount once received, else the estimate.
pub fn dividend_amount(received_amount: Option<f64>, estimated_amount: Option<f64>) -> Option<f64> {
    received_amount.or(estimated_amount)
}

/// The sheet's `rate` column: 派息 ÷ 總買入成本 snapshot.
pub fn yield_on_cost(amount: Option<f64>, buy_cost: Option<f64>) -> Option<f64> {
    match (amount, buy_cost) {
        (Some(amount), Some(buy_cost)) if buy_cost > 0.0 => Some(amount / buy_cost),
        _ => None,
    }
}

/// The sheet's second rate: 派息 ÷ (現價 snapshot × 股數 snapshot).
pub fn yield_on_price(
    amount: Option<f64>,
    received_price: Option<f64>,
    shares_held: Option<f64>,
) -> Option<f64> {
    match (amount, received_price, shares_held) {
        (Some(amount), Some(price), Some(shares)) if price > 0.0 && shares > 0.0 => {
            Some(amount / (price * shares))
        }
        _ => None,
    }
}

/// received − estimated, when both exist: how far the estimate was off.
pub fn dividend_variance(
    received_amount: Option<f64>,
    estimated_amount: Option<f64>,
) -> Option<f64> {
    match (received_amount, estimated_amount) {
        (Some(received), Some(estimated)) => Some(received - estimated),
        _ => None,
    }
}

/// The dividend facts the yearly rollup needs.
#[derive(Debug, Clone)]
pub struct DividendFacts<'a> {
    pub code: &'a str,
    pub pay_date: chrono::NaiveDate,
    pub received_amount: Option<f64>,
}

/// One stock's contribution inside a year bucket.
#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
pub struct DividendStockTotal {
    pub code: String,
    pub received: f64,
}

/// A year's received-dividend rollup, like the 回報率 sheet's per-stock
/// SUMIF block over the J–O columns.
#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
pub struct DividendYearRollup {
    pub year: i32,
    pub total: f64,
    pub stocks: Vec<DividendStockTotal>,
}

/// Received dividends grouped by the year of `pay_date`, one bucket per
/// year, stocks ordered by descending total.
pub fn dividend_year_rollups(dividends: &[DividendFacts<'_>]) -> Vec<DividendYearRollup> {
    let mut years: Vec<DividendYearRollup> = Vec::new();
    for dividend in dividends {
        let Some(received) = dividend.received_amount else {
            continue;
        };
        let year = dividend.pay_date.year();
        let rollup = match years.iter_mut().find(|rollup| rollup.year == year) {
            Some(existing) => existing,
            None => {
                years.push(DividendYearRollup {
                    year,
                    total: 0.0,
                    stocks: Vec::new(),
                });
                years.last_mut().expect("just pushed")
            }
        };
        rollup.total += received;
        match rollup.stocks.iter_mut().find(|s| s.code == dividend.code) {
            Some(stock) => stock.received += received,
            None => rollup.stocks.push(DividendStockTotal {
                code: dividend.code.to_string(),
                received,
            }),
        }
    }
    years.sort_by_key(|rollup| rollup.year);
    for rollup in &mut years {
        rollup.stocks.sort_by(|a, b| {
            b.received
                .partial_cmp(&a.received)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.code.cmp(&b.code))
        });
    }
    years
}

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

// ----- MPF (強積金) -----

/// A recorded MPF account state: a history row, or the standing values.
#[derive(Debug, Clone, Copy)]
pub struct MpfPoint {
    pub recorded_on: chrono::NaiveDate,
    pub contributions: f64,
    pub balance: f64,
}

/// Everything needed to derive one account's figures.
#[derive(Debug, Clone)]
pub struct MpfAccountFacts {
    /// Creation date of the account record: the as-of fallback boundary —
    /// dates at or after it fall back to the current values when no history
    /// row is old enough.
    pub created_on: chrono::NaiveDate,
    pub contributions: f64,
    pub balance: f64,
    pub seed_max_rate: Option<f64>,
    pub seed_max_gain: Option<f64>,
    pub history: Vec<MpfPoint>,
}

/// Section-level aggregates for the MPF overview header.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MpfTotals {
    /// Σ contributions (the sheet's `buy`).
    pub buy: f64,
    /// Σ balance (the sheet's `now`).
    pub now: f64,
    pub rate: Option<f64>,
    pub gain: f64,
    pub last_month: Option<MpfFigures>,
    pub max: MpfFigures,
}

/// An MPF account as it arrives from the user, before validation.
#[derive(Debug, Clone)]
pub struct MpfAccountInput<'a> {
    pub label: &'a str,
    pub contributions: f64,
    pub balance: f64,
}

pub fn validate_mpf_account(input: MpfAccountInput<'_>) -> Result<(), Vec<FieldError>> {
    let mut errors = Vec::new();
    if input.label.trim().is_empty() {
        errors.push(FieldError::new("label", "label is required"));
    }
    if input.contributions < 0.0 {
        errors.push(FieldError::new(
            "contributions",
            "contributions must not be negative",
        ));
    }
    if input.balance < 0.0 {
        errors.push(FieldError::new("balance", "balance must not be negative"));
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// rate = (balance − contributions) ÷ contributions; empty at contributions 0.
pub fn mpf_figures(contributions: f64, balance: f64) -> MpfFigures {
    MpfFigures {
        rate: (contributions > 0.0).then(|| (balance - contributions) / contributions),
        gain: balance - contributions,
    }
}

/// Recover last month-end per-account contributions from the sheet's frozen
/// figures. Two accounts' last-month rates plus the portfolio rate and gain
/// pin down the split exactly:
///   c1·r1 + c2·r2 = total_gain,  c1 + c2 = total_gain ÷ total_rate.
/// Returns None when the inputs are missing or degenerate, so the caller
/// falls back to the current contributions.
pub fn mpf_last_month_contributions(
    rates: (f64, f64),
    total_rate: f64,
    total_gain: f64,
) -> Option<(f64, f64)> {
    let (r1, r2) = rates;
    if total_rate <= 0.0 || (r1 - r2).abs() < TOLERANCE {
        return None;
    }
    let total = total_gain / total_rate;
    let c1 = (total_gain - total * r2) / (r1 - r2);
    let c2 = total - c1;
    (c1 >= 0.0 && c2 >= 0.0 && c1.is_finite() && c2.is_finite()).then_some((c1, c2))
}

fn first_of_month(date: chrono::NaiveDate) -> chrono::NaiveDate {
    chrono::NaiveDate::from_ymd_opt(date.year(), date.month(), 1).expect("valid month")
}

fn month_end(date: chrono::NaiveDate) -> chrono::NaiveDate {
    first_of_month(date) + chrono::Months::new(1) - chrono::Days::new(1)
}

/// The last day of every calendar month strictly between the months holding
/// `from` and `to` — the synthetic month-end rows a multi-month gap needs.
pub fn mpf_gap_month_ends(
    from: chrono::NaiveDate,
    to: chrono::NaiveDate,
) -> Vec<chrono::NaiveDate> {
    let mut ends = Vec::new();
    let mut cursor = first_of_month(from) + chrono::Months::new(1);
    let limit = first_of_month(to);
    while cursor < limit {
        ends.push(month_end(cursor));
        cursor = cursor + chrono::Months::new(1);
    }
    ends
}

/// Figures of the latest history row inside the previous calendar month.
pub fn mpf_last_month(history: &[MpfPoint], today: chrono::NaiveDate) -> Option<MpfFigures> {
    let prev_end = first_of_month(today) - chrono::Days::new(1);
    let (year, month) = (prev_end.year(), prev_end.month());
    history
        .iter()
        .filter(|point| point.recorded_on.year() == year && point.recorded_on.month() == month)
        .max_by_key(|point| point.recorded_on)
        .map(|point| mpf_figures(point.contributions, point.balance))
}

fn fold_max(acc: Option<f64>, value: Option<f64>) -> Option<f64> {
    match (acc, value) {
        (Some(a), Some(v)) => Some(a.max(v)),
        (None, v) => v,
        (a, None) => a,
    }
}

/// All-time maxima over the seeded marks, every history row, and the current
/// values. Rate and gain are tracked independently and may peak at different
/// moments. `current` is optional: the seeded marks and history alone still
/// define a max when no current values exist.
pub fn mpf_max(
    history: &[MpfPoint],
    current: Option<MpfPoint>,
    seed_max_rate: Option<f64>,
    seed_max_gain: Option<f64>,
) -> MpfFigures {
    let mut rate = seed_max_rate;
    let mut gain = seed_max_gain;
    for point in history.iter().copied().chain(current) {
        let figures = mpf_figures(point.contributions, point.balance);
        rate = fold_max(rate, figures.rate);
        gain = fold_max(gain, Some(figures.gain));
    }
    MpfFigures {
        rate,
        gain: gain.unwrap_or(0.0),
    }
}

/// The account's standing values on `date`: the latest history row on or
/// before it; otherwise the current values when the record already existed,
/// else nothing.
fn mpf_as_of(facts: &MpfAccountFacts, date: chrono::NaiveDate) -> Option<MpfPoint> {
    if let Some(point) = facts
        .history
        .iter()
        .filter(|point| point.recorded_on <= date)
        .max_by_key(|point| point.recorded_on)
    {
        return Some(*point);
    }
    (facts.created_on <= date).then_some(MpfPoint {
        recorded_on: date,
        contributions: facts.contributions,
        balance: facts.balance,
    })
}

/// Section aggregates via an as-of merge: at every recorded date (plus last
/// month-end and today) each account contributes its standing values. This
/// carries an untouched account forward and gives a true portfolio max rather
/// than summing per-account peaks that may never have co-occurred. The
/// portfolio seeds floor the maxima.
pub fn mpf_totals(
    accounts: &[MpfAccountFacts],
    today: chrono::NaiveDate,
    seed_max_rate: Option<f64>,
    seed_max_gain: Option<f64>,
) -> MpfTotals {
    // The standing values count as a point at `today`, so an account whose
    // today's history row was deleted (or never written) still reports its
    // current state rather than a stale row.
    let extended: Vec<MpfAccountFacts> = accounts
        .iter()
        .map(|account| {
            let mut facts = account.clone();
            facts.history.push(MpfPoint {
                recorded_on: today,
                contributions: account.contributions,
                balance: account.balance,
            });
            facts
        })
        .collect();

    let prev_end = first_of_month(today) - chrono::Days::new(1);
    let mut dates: Vec<chrono::NaiveDate> = extended
        .iter()
        .flat_map(|account| account.history.iter().map(|point| point.recorded_on))
        .collect();
    dates.push(prev_end);
    dates.sort_unstable();
    dates.dedup();

    let mut max_rate = seed_max_rate;
    let mut max_gain = seed_max_gain;
    let mut last_month = None;
    for date in dates {
        let (mut contributions, mut balance) = (0.0, 0.0);
        let mut any = false;
        for account in &extended {
            if let Some(point) = mpf_as_of(account, date) {
                contributions += point.contributions;
                balance += point.balance;
                any = true;
            }
        }
        if !any {
            continue;
        }
        let figures = mpf_figures(contributions, balance);
        if date == prev_end {
            last_month = Some(figures);
        }
        max_rate = fold_max(max_rate, figures.rate);
        max_gain = fold_max(max_gain, Some(figures.gain));
    }

    let buy: f64 = accounts.iter().map(|account| account.contributions).sum();
    let now: f64 = accounts.iter().map(|account| account.balance).sum();
    let current = mpf_figures(buy, now);
    MpfTotals {
        buy,
        now,
        rate: current.rate,
        gain: current.gain,
        last_month,
        max: MpfFigures {
            rate: max_rate,
            gain: max_gain.unwrap_or(current.gain),
        },
    }
}

/// A bond as it arrives from the user, before validation.
#[derive(Debug, Clone)]
pub struct BondInput<'a> {
    pub label: &'a str,
    pub issue_no: Option<&'a str>,
    pub principal: f64,
    pub maturity_date: &'a str,
}

/// A validated bond record.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedBond {
    pub label: String,
    pub issue_no: Option<String>,
    pub principal: f64,
    pub maturity_date: String,
}

/// Validate user input for a bond record.
pub fn validate_bond(input: BondInput<'_>) -> Result<ValidatedBond, Vec<FieldError>> {
    let mut errors = Vec::new();

    let label = input.label.trim();
    if label.is_empty() {
        errors.push(FieldError::new("label", "label is required"));
    }
    if !input.principal.is_finite() || input.principal <= 0.0 {
        errors.push(FieldError::new(
            "principal",
            "principal must be a positive number",
        ));
    }
    if chrono::NaiveDate::parse_from_str(input.maturity_date, "%Y-%m-%d").is_err() {
        errors.push(FieldError::new(
            "maturity_date",
            "maturity date must be a calendar date in YYYY-MM-DD form",
        ));
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    Ok(ValidatedBond {
        label: label.to_string(),
        issue_no: input
            .issue_no
            .map(str::trim)
            .filter(|issue_no| !issue_no.is_empty())
            .map(str::to_string),
        principal: input.principal,
        maturity_date: input.maturity_date.to_string(),
    })
}

/// A coupon as it arrives from the user, before validation.
#[derive(Debug, Clone)]
pub struct CouponInput<'a> {
    pub pay_date: &'a str,
    pub fixing_date: Option<&'a str>,
    pub annual_rate: Option<f64>,
    pub per_10k: Option<f64>,
    pub received_amount: Option<f64>,
}

/// A validated coupon record.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedCoupon {
    pub pay_date: String,
    pub fixing_date: Option<String>,
    pub annual_rate: Option<f64>,
    pub per_10k: Option<f64>,
    pub received_amount: Option<f64>,
}

/// Validate user input for a coupon record. `annual_rate`/`per_10k` may both
/// be absent — that is the sheet's 待定 (rate not yet fixed).
pub fn validate_coupon(input: CouponInput<'_>) -> Result<ValidatedCoupon, Vec<FieldError>> {
    let mut errors = Vec::new();

    if chrono::NaiveDate::parse_from_str(input.pay_date, "%Y-%m-%d").is_err() {
        errors.push(FieldError::new(
            "pay_date",
            "付息日 must be a calendar date in YYYY-MM-DD form",
        ));
    }
    if let Some(fixing_date) = input.fixing_date
        && chrono::NaiveDate::parse_from_str(fixing_date, "%Y-%m-%d").is_err()
    {
        errors.push(FieldError::new(
            "fixing_date",
            "利息釐定日 must be a calendar date in YYYY-MM-DD form",
        ));
    }
    for (field, value, name) in [
        ("per_10k", input.per_10k, "每1萬利息"),
        ("received_amount", input.received_amount, "實收利息"),
    ] {
        if let Some(value) = value
            && (!value.is_finite() || value < 0.0)
        {
            errors.push(FieldError::new(
                field,
                format!("{name} must not be negative"),
            ));
        }
    }
    if let Some(rate) = input.annual_rate
        && (!rate.is_finite() || !(0.0..1.0).contains(&rate))
    {
        errors.push(FieldError::new(
            "annual_rate",
            "年息率 is stored as a fraction (0.04 = 4%) and must be less than 1",
        ));
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    Ok(ValidatedCoupon {
        pay_date: input.pay_date.to_string(),
        fixing_date: input
            .fixing_date
            .map(str::trim)
            .filter(|fixing_date| !fixing_date.is_empty())
            .map(str::to_string),
        annual_rate: input.annual_rate,
        per_10k: input.per_10k,
        received_amount: input.received_amount,
    })
}

/// The sheet's maturity rule: the bond is active while its `end` date is in
/// the future — the same `TODAY()` predicate the deposits use.
pub fn bond_active(maturity_date: chrono::NaiveDate, today: chrono::NaiveDate) -> bool {
    maturity_date > today
}

/// A coupon's lifecycle: received once an amount is recorded; 待定
/// (`PendingFix`) while the announced figures are missing; otherwise pending.
pub fn coupon_status(
    received_amount: Option<f64>,
    annual_rate: Option<f64>,
    per_10k: Option<f64>,
) -> CouponStatus {
    if received_amount.is_some() {
        CouponStatus::Received
    } else if annual_rate.is_none() || per_10k.is_none() {
        CouponStatus::PendingFix
    } else {
        CouponStatus::Pending
    }
}

/// The sheet's interest column: 每1萬利息 × principal ÷ 10000.
pub fn coupon_expected(per_10k: Option<f64>, principal: f64) -> Option<f64> {
    per_10k.map(|per_10k| per_10k * principal / 10000.0)
}

/// received − expected, when both exist: how far the announced figure was off.
pub fn coupon_variance(received_amount: Option<f64>, expected: Option<f64>) -> Option<f64> {
    match (received_amount, expected) {
        (Some(received), Some(expected)) => Some(received - expected),
        _ => None,
    }
}

/// An AIA policy as it arrives from the user, before validation.
#[derive(Debug, Clone)]
pub struct AiaPolicyInput<'a> {
    pub label: &'a str,
    pub policy_no: Option<&'a str>,
    pub next_pay_date: Option<&'a str>,
    pub premium_usd: f64,
    pub value_usd: f64,
    pub remaining_years: Option<f64>,
    pub withdrew_usd: f64,
    pub note: Option<&'a str>,
    pub link: Option<&'a str>,
    pub excluded: bool,
    pub in_account: bool,
}

/// A validated AIA policy record.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedAiaPolicy {
    pub label: String,
    pub policy_no: Option<String>,
    pub next_pay_date: Option<String>,
    pub premium_usd: f64,
    pub value_usd: f64,
    pub remaining_years: Option<f64>,
    pub withdrew_usd: f64,
    pub note: Option<String>,
    pub link: Option<String>,
    pub excluded: bool,
    pub in_account: bool,
}

/// Validate user input for an AIA policy record.
pub fn validate_aia_policy(
    input: AiaPolicyInput<'_>,
) -> Result<ValidatedAiaPolicy, Vec<FieldError>> {
    let mut errors = Vec::new();

    let label = input.label.trim();
    if label.is_empty() {
        errors.push(FieldError::new("label", "label is required"));
    }
    for (field, value, name) in [
        ("premium_usd", input.premium_usd, "premium"),
        ("value_usd", input.value_usd, "value"),
        ("withdrew_usd", input.withdrew_usd, "withdrew"),
    ] {
        if !value.is_finite() || value < 0.0 {
            errors.push(FieldError::new(
                field,
                format!("{name} must not be negative"),
            ));
        }
    }
    if let Some(remaining) = input.remaining_years
        && (!remaining.is_finite() || remaining < 0.0)
    {
        errors.push(FieldError::new(
            "remaining_years",
            "remaining years must not be negative",
        ));
    }
    if let Some(next_pay_date) = input.next_pay_date
        && chrono::NaiveDate::parse_from_str(next_pay_date, "%Y-%m-%d").is_err()
    {
        errors.push(FieldError::new(
            "next_pay_date",
            "next pay date must be a calendar date in YYYY-MM-DD form",
        ));
    }
    if let Some(link) = input.link.map(str::trim).filter(|link| !link.is_empty())
        && !(link.starts_with("http://") || link.starts_with("https://"))
    {
        errors.push(FieldError::new(
            "link",
            "link must start with http:// or https://",
        ));
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    let trimmed = |value: Option<&str>| {
        value
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    };
    Ok(ValidatedAiaPolicy {
        label: label.to_string(),
        policy_no: trimmed(input.policy_no),
        next_pay_date: trimmed(input.next_pay_date),
        premium_usd: input.premium_usd,
        value_usd: input.value_usd,
        remaining_years: input.remaining_years,
        withdrew_usd: input.withdrew_usd,
        note: trimmed(input.note),
        link: trimmed(input.link),
        excluded: input.excluded,
        in_account: input.in_account,
    })
}

/// The sheet's `balance %%` column: withdrawals count toward the return.
/// Absent when nothing was ever paid in.
pub fn aia_balance_pct(value_usd: f64, withdrew_usd: f64, premium_usd: f64) -> Option<f64> {
    (premium_usd > 0.0).then(|| (value_usd + withdrew_usd - premium_usd) / premium_usd)
}

/// The policy fields a totals row needs.
#[derive(Debug, Clone, Copy)]
pub struct AiaPolicyFacts {
    pub premium_usd: f64,
    pub value_usd: f64,
    pub withdrew_usd: f64,
    pub excluded: bool,
    pub in_account: bool,
}

/// Portfolio-level AIA figures: the sheet's summary block.
#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
pub struct AiaTotals {
    /// Σ premium over non-excluded rows (the sheet's `buy usd`).
    pub premium: f64,
    /// Σ value over non-excluded rows (the sheet's `now usd`).
    pub value: f64,
    /// Σ withdrew over non-excluded rows (the sheet's drew column).
    pub withdrew: f64,
    /// Same formula as the per-policy figure, on the totals (the sheet's `B4`).
    pub balance_pct: Option<f64>,
    /// Σ value over in-account rows (the sheet's `AIA display value`).
    pub display_value: f64,
    /// HKD conversions via the stored rate; absent while no rate exists.
    pub premium_hkd: Option<f64>,
    pub value_hkd: Option<f64>,
    pub withdrew_hkd: Option<f64>,
    /// The sheet's unlabeled B5 scratch cell: `now − buy − drew` in HKD —
    /// net position change, counting withdrawals as outflow.
    pub net_change_hkd: Option<f64>,
}

pub fn aia_totals(policies: &[AiaPolicyFacts], rate: Option<f64>) -> AiaTotals {
    let mut premium = 0.0;
    let mut value = 0.0;
    let mut withdrew = 0.0;
    let mut display_value = 0.0;
    for policy in policies {
        if policy.in_account {
            display_value += policy.value_usd;
        }
        if policy.excluded {
            continue;
        }
        premium += policy.premium_usd;
        value += policy.value_usd;
        withdrew += policy.withdrew_usd;
    }
    let hkd = |amount: f64| rate.map(|rate| amount * rate);
    AiaTotals {
        premium,
        value,
        withdrew,
        balance_pct: aia_balance_pct(value, withdrew, premium),
        display_value,
        premium_hkd: hkd(premium),
        value_hkd: hkd(value),
        withdrew_hkd: hkd(withdrew),
        net_change_hkd: hkd(value - premium - withdrew),
    }
}

/// The earliest premium-due date still ahead of (or on) today — the premium
/// the user still owes.
pub fn next_premium_due(
    dates: impl Iterator<Item = chrono::NaiveDate>,
    today: chrono::NaiveDate,
) -> Option<chrono::NaiveDate> {
    dates.filter(|date| *date >= today).min()
}

/// The fields a recorded premium payment rewrites on its policy.
#[derive(Debug, Clone, PartialEq)]
pub struct AiaPaymentOutcome {
    pub premium_usd: f64,
    pub remaining_years: Option<f64>,
    pub next_pay_date: Option<chrono::NaiveDate>,
}

/// Recording a premium payment: the amount joins the cumulative premium, one
/// remaining year is used up (never below zero), and the next due date moves —
/// to the submitted date when given, else one year on from the current one.
/// Collapses the sheet's three manual edits into one.
pub fn apply_aia_payment(
    premium_usd: f64,
    remaining_years: Option<f64>,
    next_pay_date: Option<chrono::NaiveDate>,
    amount_usd: f64,
    submitted_next_pay: Option<chrono::NaiveDate>,
) -> AiaPaymentOutcome {
    AiaPaymentOutcome {
        premium_usd: premium_usd + amount_usd,
        remaining_years: remaining_years.map(|years| (years - 1.0).max(0.0)),
        next_pay_date: submitted_next_pay.or_else(|| {
            next_pay_date.and_then(|date| date.checked_add_months(chrono::Months::new(12)))
        }),
    }
}

/// Recording a withdrawal: the amount joins the cumulative withdrew figure.
pub fn apply_aia_withdrawal(withdrew_usd: f64, amount_usd: f64) -> f64 {
    withdrew_usd + amount_usd
}

/// Reversing a recorded event's amount on its cumulative field.
pub fn undo_aia_event_amount(current: f64, amount_usd: f64) -> f64 {
    current - amount_usd
}

/// An event as it arrives from the user, before validation.
#[derive(Debug, Clone)]
pub struct AiaEventInput<'a> {
    pub kind: crate::models::AiaEventKind,
    pub event_date: &'a str,
    pub amount_usd: f64,
    pub next_pay_date: Option<&'a str>,
}

/// A validated event record.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedAiaEvent {
    pub kind: crate::models::AiaEventKind,
    pub event_date: String,
    pub amount_usd: f64,
    pub next_pay_date: Option<String>,
}

/// Validate user input for a payment/withdrawal event.
pub fn validate_aia_event(input: AiaEventInput<'_>) -> Result<ValidatedAiaEvent, Vec<FieldError>> {
    let mut errors = Vec::new();

    if chrono::NaiveDate::parse_from_str(input.event_date, "%Y-%m-%d").is_err() {
        errors.push(FieldError::new(
            "event_date",
            "event date must be a calendar date in YYYY-MM-DD form",
        ));
    }
    if !input.amount_usd.is_finite() || input.amount_usd <= 0.0 {
        errors.push(FieldError::new(
            "amount_usd",
            "amount must be a positive number",
        ));
    }
    if let Some(next_pay_date) = input.next_pay_date
        && chrono::NaiveDate::parse_from_str(next_pay_date, "%Y-%m-%d").is_err()
    {
        errors.push(FieldError::new(
            "next_pay_date",
            "next pay date must be a calendar date in YYYY-MM-DD form",
        ));
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    Ok(ValidatedAiaEvent {
        kind: input.kind,
        event_date: input.event_date.to_string(),
        amount_usd: input.amount_usd,
        next_pay_date: input
            .next_pay_date
            .map(str::trim)
            .filter(|date| !date.is_empty())
            .map(str::to_string),
    })
}

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

/// The components of `Overview!B1`/`H1`, already resolved by the caller.
#[derive(Debug, Clone, Copy)]
pub struct LiveTotalsInput {
    /// 港股 market value.
    pub hk_market_value: f64,
    /// 美股 market value in USD.
    pub us_market_value: f64,
    /// USD→HKD rate (`aia.usd_hkd_rate`); rate-dependent terms drop out when absent.
    pub usd_hkd_rate: Option<f64>,
    /// 定期!B1: Σ principal over active deposits.
    pub deposits_active_principal: f64,
    /// 債券!B1: Σ principal over active bonds.
    pub bonds_active_principal: f64,
    /// AIA 總 value in USD (non-excluded rows).
    pub aia_value_usd: f64,
    /// MPF 總結存.
    pub mpf_balance: f64,
    /// Σ manual_assets where kind = 'asset'.
    pub manual_assets_sum: f64,
    /// Σ manual_assets where kind = 'cash'.
    pub cash_sum: f64,
    /// 美股!B4/B5: the IBKR account's cash positions, part of Overview!B9.
    pub ibkr_hkd_cash: f64,
    pub ibkr_usd_cash: f64,
    /// Current 開心Pool balance.
    pub pool_balance: f64,
}

/// `Overview!B1` (總數) and `H1` (流動資產) computed inside the backend.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, ToSchema)]
pub struct LiveTotals {
    pub total_assets: f64,
    pub liquid_assets: f64,
}

pub fn live_totals(input: &LiveTotalsInput) -> LiveTotals {
    // The US side is the sheet's 美股!B7 (Overview!B9): stocks plus the IBKR
    // account's own cash positions, with the HKD part needing no rate.
    let us_hkd = input
        .usd_hkd_rate
        .map(|rate| (input.us_market_value + input.ibkr_usd_cash) * rate);
    let aia_hkd = input.usd_hkd_rate.map(|rate| input.aia_value_usd * rate);
    LiveTotals {
        total_assets: input.hk_market_value
            + us_hkd.unwrap_or(0.0)
            + input.ibkr_hkd_cash
            + input.deposits_active_principal
            + input.bonds_active_principal
            + aia_hkd.unwrap_or(0.0)
            + input.mpf_balance
            + input.manual_assets_sum
            + input.cash_sum,
        liquid_assets: input.hk_market_value
            + input.deposits_active_principal
            + input.cash_sum
            + input.bonds_active_principal
            + us_hkd.unwrap_or(0.0)
            + input.ibkr_hkd_cash
            - input.pool_balance,
    }
}

/// A deposit as a suggestion source: 定期 start/end events.
#[derive(Debug, Clone)]
pub struct SuggestionDeposit {
    pub id: i64,
    pub start_date: Option<chrono::NaiveDate>,
    pub end_date: chrono::NaiveDate,
    pub principal: f64,
    pub interest: f64,
    pub label: Option<String>,
    /// 收訖: the user confirmed the principal + interest came back. Interest
    /// counts in the end month only once received.
    pub received: bool,
}

/// An HK trade as a suggestion source (US trades settle inside IBKR and stay
/// manual items).
#[derive(Debug, Clone)]
pub struct SuggestionTrade {
    pub id: i64,
    pub date: chrono::NaiveDate,
    pub kind: TradeType,
    pub total: f64,
    pub code: String,
}

/// A received dividend or bond coupon as a suggestion source.
#[derive(Debug, Clone)]
pub struct SuggestionReceipt {
    pub id: i64,
    pub pay_date: chrono::NaiveDate,
    /// `received_amount` once received, else the expected/estimated figure;
    /// None while nothing is known (待定 coupon, estimate-less dividend).
    pub amount: Option<f64>,
    /// Stock code or bond label.
    pub label: String,
    /// Whether the payment was 收訖 — pending receipts preview in the 利息
    /// breakdown but never count and never become suggestions.
    pub received: bool,
}

/// A recorded AIA premium payment as a suggestion source.
#[derive(Debug, Clone)]
pub struct SuggestionAiaPayment {
    pub id: i64,
    pub date: chrono::NaiveDate,
    pub amount_usd: f64,
    pub policy: String,
}

/// Every dated event the suggestion builder reads, plus the month's own
/// pool_input and the USD→HKD rate.
#[derive(Debug, Default)]
pub struct SuggestionEvents {
    pub deposits: Vec<SuggestionDeposit>,
    pub trades: Vec<SuggestionTrade>,
    pub dividends: Vec<SuggestionReceipt>,
    pub coupons: Vec<SuggestionReceipt>,
    pub aia_payments: Vec<SuggestionAiaPayment>,
    /// The month row's own pool_input.
    pub pool_input: f64,
    pub usd_hkd_rate: Option<f64>,
}

fn parse_month(month: &str) -> Option<chrono::NaiveDate> {
    chrono::NaiveDate::parse_from_str(month, "%Y-%m-%d").ok()
}

fn in_month(date: chrono::NaiveDate, month: chrono::NaiveDate) -> bool {
    date.year() == month.year() && date.month() == month.month()
}

/// Suggestions exist only for the current month and later: imported history
/// already carries its 調整 as single items, so suggesting auto items there
/// would double-count.
pub fn suggestions_enabled(month: &str, today: chrono::NaiveDate) -> bool {
    match parse_month(month) {
        Some(month) => month >= first_of_month(today),
        None => false,
    }
}

/// The candidate items for one month, minus keys already stored or dismissed.
/// `month` is 'YYYY-MM-01'.
pub fn build_suggestions(
    month: &str,
    events: &SuggestionEvents,
    stored_keys: &HashSet<String>,
    dismissed_keys: &HashSet<String>,
) -> Vec<MonthSuggestion> {
    let Some(month) = parse_month(month) else {
        return Vec::new();
    };
    let mut suggestions = Vec::new();

    for deposit in &events.deposits {
        if let Some(start_date) = deposit.start_date
            && in_month(start_date, month)
        {
            suggestions.push(MonthSuggestion {
                auto_key: format!("dep-start:{}", deposit.id),
                category: MonthItemCategory::Adjustment,
                label: deposit.label.clone(),
                amount: -deposit.principal,
                source: "deposit".to_string(),
            });
        }
        if in_month(deposit.end_date, month) {
            suggestions.push(MonthSuggestion {
                auto_key: format!("dep-end:{}", deposit.id),
                category: MonthItemCategory::Adjustment,
                label: deposit.label.clone(),
                amount: deposit.principal + deposit.interest,
                source: "deposit".to_string(),
            });
        }
    }

    for trade in &events.trades {
        if in_month(trade.date, month) {
            suggestions.push(MonthSuggestion {
                auto_key: format!("trade:{}", trade.id),
                category: MonthItemCategory::Adjustment,
                label: Some(format!("{} {}", trade.code, trade.kind.as_str())),
                amount: match trade.kind {
                    TradeType::Buy => -trade.total,
                    TradeType::Sell => trade.total,
                },
                source: "trade".to_string(),
            });
        }
    }

    for dividend in &events.dividends {
        if in_month(dividend.pay_date, month) && dividend.received {
            suggestions.push(MonthSuggestion {
                auto_key: format!("div:{}", dividend.id),
                category: MonthItemCategory::Adjustment,
                label: Some(dividend.label.clone()),
                amount: dividend.amount.unwrap_or(0.0),
                source: "dividend".to_string(),
            });
        }
    }

    for coupon in &events.coupons {
        if in_month(coupon.pay_date, month) && coupon.received {
            suggestions.push(MonthSuggestion {
                auto_key: format!("coupon:{}", coupon.id),
                category: MonthItemCategory::Adjustment,
                label: Some(coupon.label.clone()),
                amount: coupon.amount.unwrap_or(0.0),
                source: "coupon".to_string(),
            });
        }
    }

    for payment in &events.aia_payments {
        if in_month(payment.date, month)
            && let Some(rate) = events.usd_hkd_rate
        {
            suggestions.push(MonthSuggestion {
                auto_key: format!("aia-pay:{}", payment.id),
                category: MonthItemCategory::ExtraSpend,
                label: Some(payment.policy.clone()),
                amount: payment.amount_usd * rate,
                source: "aia".to_string(),
            });
        }
    }

    if events.pool_input > 0.0 {
        suggestions.push(MonthSuggestion {
            auto_key: "pool-input".to_string(),
            category: MonthItemCategory::Adjustment,
            label: Some("Irene + 開心 Pool".to_string()),
            amount: -events.pool_input,
            source: "pool".to_string(),
        });
    }

    suggestions
        .into_iter()
        .filter(|suggestion| {
            !stored_keys.contains(&suggestion.auto_key)
                && !dismissed_keys.contains(&suggestion.auto_key)
        })
        .collect()
}

/// The auto 利息 components of a month: every deposit whose `end_date` falls
/// in the month, every coupon paid in the month, and every HK dividend paid
/// in the month. Unreceived entries carry `received: false` and preview with
/// their expected/estimated amount (None when unknown); only received ones
/// count via `auto_interest`. Bank 活期 interest has no source — it stays
/// manual as `interest` items on top.
pub fn interest_components(month: &str, events: &SuggestionEvents) -> Vec<InterestComponent> {
    let Some(month) = parse_month(month) else {
        return Vec::new();
    };
    let mut components = Vec::new();
    for deposit in &events.deposits {
        if in_month(deposit.end_date, month) && deposit.interest != 0.0 {
            components.push(InterestComponent {
                source: "deposit".to_string(),
                label: deposit.label.clone(),
                amount: Some(deposit.interest),
                received: deposit.received,
            });
        }
    }
    for coupon in &events.coupons {
        if in_month(coupon.pay_date, month) {
            components.push(InterestComponent {
                source: "coupon".to_string(),
                label: Some(coupon.label.clone()),
                amount: coupon.amount,
                received: coupon.received,
            });
        }
    }
    for dividend in &events.dividends {
        if in_month(dividend.pay_date, month) {
            components.push(InterestComponent {
                source: "dividend".to_string(),
                label: Some(dividend.label.clone()),
                amount: dividend.amount,
                received: dividend.received,
            });
        }
    }
    components
}

/// Σ `interest_components` where `received` — the auto part of a month's 利息.
/// Unreceived events preview in the breakdown but do not count.
pub fn auto_interest(month: chrono::NaiveDate, events: &SuggestionEvents) -> f64 {
    interest_components(&month.to_string(), events)
        .iter()
        .filter(|component| component.received)
        .map(|component| component.amount.unwrap_or(0.0))
        .sum()
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

    fn deposit_input<'a>() -> DepositInput<'a> {
        DepositInput {
            label: Some("SC-9632"),
            bank: Some("SC"),
            principal: Some(110000.0),
            rate: Some(0.028),
            interest: Some(993.0),
            start_date: None,
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
            start_date: None,
            end_date: "2026-05-14",
        };
        assert!(validate_deposit(interest_only).is_ok());

        let label_only = DepositInput {
            label: Some("SC-4900"),
            bank: Some("SC"),
            principal: None,
            rate: None,
            interest: Some(0.0),
            start_date: None,
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
            start_date: None,
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

    // --- dividends ---

    fn dated_buy(date: &str, shares: f64, total: f64) -> (chrono::NaiveDate, TradeFacts) {
        (day(date), buy(shares, total))
    }

    #[test]
    fn snapshot_counts_only_trades_on_or_before_the_date() {
        // 中國銀行: buys on 2023-05-23, 2024-02-15 and 2024-03-04.
        let trades = [
            dated_buy("2023-05-23", 30000.0, 96523.15),
            dated_buy("2024-02-15", 18000.0, 53731.5),
            dated_buy("2024-03-04", 20000.0, 58491.99),
        ];
        // The 2023-08-18 dividend saw only the first buy (sheet L denominator).
        let (shares, cost) = holdings_snapshot(&trades, day("2023-08-18"));
        assert!(approx_eq(shares, 30000.0));
        assert!(approx_eq(cost, 96523.15));
        // Later rows see the full position.
        let (shares, cost) = holdings_snapshot(&trades, day("2025-12-16"));
        assert!(approx_eq(shares, 68000.0));
        assert!(approx_eq(cost, 208746.64));
    }

    #[test]
    fn snapshot_subtracts_sells_from_shares_but_not_cost() {
        let trades = [
            dated_buy("2025-01-10", 1000.0, 10030.0),
            (day("2025-03-01"), sell(400.0)),
        ];
        let (shares, cost) = holdings_snapshot(&trades, day("2025-06-01"));
        assert!(approx_eq(shares, 600.0));
        assert!(approx_eq(cost, 10030.0));
    }

    fn dividend_input<'a>() -> DividendInput<'a> {
        DividendInput {
            pay_date: "2026-09-30",
            per_share: Some(0.25),
            shares_held: Some(68000.0),
            buy_cost: Some(208746.64),
            estimated_amount: None,
            received_amount: None,
            received_price: None,
        }
    }

    #[test]
    fn dividend_per_share_only_derives_the_estimate() {
        let validated = validate_dividend(dividend_input()).expect("valid dividend");
        assert_eq!(validated.pay_date, "2026-09-30");
        assert!(approx_eq(
            validated.estimated_amount.expect("derived"),
            17000.0
        ));
    }

    #[test]
    fn dividend_estimate_only_derives_the_per_share() {
        let validated = validate_dividend(DividendInput {
            per_share: None,
            estimated_amount: Some(17000.0),
            ..dividend_input()
        })
        .expect("valid dividend");
        assert!(approx_eq(validated.per_share.expect("derived"), 0.25));
    }

    #[test]
    fn dividend_received_only_derives_the_per_share() {
        let validated = validate_dividend(DividendInput {
            per_share: None,
            estimated_amount: None,
            received_amount: Some(17000.0),
            ..dividend_input()
        })
        .expect("valid dividend");
        assert!(approx_eq(validated.per_share.expect("derived"), 0.25));
    }

    #[test]
    fn dividend_per_share_stays_empty_when_no_shares() {
        let validated = validate_dividend(DividendInput {
            per_share: None,
            shares_held: Some(0.0),
            estimated_amount: Some(17000.0),
            ..dividend_input()
        })
        .expect("valid dividend");
        assert_eq!(validated.per_share, None);
    }

    #[test]
    fn dividend_rejects_malformed_dates() {
        let errors = validate_dividend(DividendInput {
            pay_date: "2026/13/45",
            ..dividend_input()
        })
        .expect_err("must fail");
        assert!(errors.iter().any(|e| e.field == "pay_date"));
    }

    #[test]
    fn dividend_rejects_negative_amounts() {
        let errors = validate_dividend(DividendInput {
            per_share: Some(-0.1),
            ..dividend_input()
        })
        .expect_err("must fail");
        assert!(errors.iter().any(|e| e.field == "per_share"));
    }

    #[test]
    fn dividend_rejects_a_record_with_no_amount_info() {
        let errors = validate_dividend(DividendInput {
            per_share: None,
            estimated_amount: None,
            received_amount: None,
            ..dividend_input()
        })
        .expect_err("must fail");
        assert!(errors.iter().any(|e| e.field == "estimated_amount"));
    }

    #[test]
    fn dividend_yields_match_the_sheet_formulas() {
        // 中國銀行 2025-12-16 row: L = M/208746.64, N = M/(4.46*68000).
        let amount = Some(8219.65);
        assert!(approx_eq(
            yield_on_cost(amount, Some(208746.64)).unwrap(),
            0.03937620265
        ));
        assert!(approx_eq(
            yield_on_price(amount, Some(4.46), Some(68000.0)).unwrap(),
            8219.65 / (4.46 * 68000.0)
        ));
    }

    #[test]
    fn dividend_yields_are_empty_without_denominators() {
        assert_eq!(yield_on_cost(Some(100.0), None), None);
        assert_eq!(yield_on_cost(Some(100.0), Some(0.0)), None);
        assert_eq!(yield_on_price(Some(100.0), None, Some(10.0)), None);
        assert_eq!(yield_on_price(Some(100.0), Some(5.0), None), None);
        assert_eq!(yield_on_cost(None, Some(100.0)), None);
    }

    #[test]
    fn dividend_amount_prefers_received_and_variance_compares() {
        assert_eq!(dividend_amount(Some(16500.0), Some(17000.0)), Some(16500.0));
        assert_eq!(dividend_amount(None, Some(17000.0)), Some(17000.0));
        assert_eq!(dividend_amount(None, None), None);
        assert_eq!(
            dividend_variance(Some(16500.0), Some(17000.0)),
            Some(-500.0)
        );
        assert_eq!(dividend_variance(Some(16500.0), None), None);
    }

    fn dividend_fact<'a>(
        code: &'a str,
        pay_date: &str,
        received: Option<f64>,
    ) -> DividendFacts<'a> {
        DividendFacts {
            code,
            pay_date: day(pay_date),
            received_amount: received,
        }
    }

    #[test]
    fn dividend_year_rollups_group_received_by_pay_year() {
        let facts = [
            dividend_fact("中國銀行", "2025-12-16", Some(7342.08)),
            dividend_fact("中國銀行", "2026-08-29", Some(8219.65)),
            dividend_fact("匯豐", "2026-09-26", Some(909.83)),
            // Pending rows never enter the rollup.
            dividend_fact("港交所", "2026-10-01", None),
        ];
        let years = dividend_year_rollups(&facts);
        assert_eq!(years.len(), 2);
        assert_eq!(years[0].year, 2025);
        assert!(approx_eq(years[0].total, 7342.08));
        assert_eq!(years[1].year, 2026);
        assert!(approx_eq(years[1].total, 9129.48));
        assert_eq!(years[1].stocks[0].code, "中國銀行");
        assert!(approx_eq(years[1].stocks[0].received, 8219.65));
    }

    fn snapshot_year(
        invested: Option<f64>,
        cost: Option<f64>,
        market_value: Option<f64>,
    ) -> YearSnapshot {
        YearSnapshot {
            invested,
            cost,
            market_value,
            sold_pl: None,
            updated_at: "2026-01-01T00:00:00+08:00".to_string(),
        }
    }

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

    fn mpf_point(date: &str, contributions: f64, balance: f64) -> MpfPoint {
        MpfPoint {
            recorded_on: day(date),
            contributions,
            balance,
        }
    }

    #[test]
    fn mpf_rate_is_gain_over_contributions() {
        let figures = mpf_figures(100_000.0, 125_000.0);
        assert!(approx_eq(figures.rate.unwrap(), 0.25));
        assert!(approx_eq(figures.gain, 25_000.0));
    }

    #[test]
    fn mpf_zero_contributions_leave_rate_empty() {
        let figures = mpf_figures(0.0, 500.0);
        assert!(figures.rate.is_none());
        assert!(approx_eq(figures.gain, 500.0));
    }

    #[test]
    fn mpf_gap_month_ends_cover_only_months_strictly_between() {
        assert_eq!(
            mpf_gap_month_ends(day("2026-08-28"), day("2026-10-05")),
            vec![day("2026-09-30")]
        );
        assert!(mpf_gap_month_ends(day("2026-08-28"), day("2026-09-05")).is_empty());
        assert!(mpf_gap_month_ends(day("2026-09-01"), day("2026-09-20")).is_empty());
        assert_eq!(
            mpf_gap_month_ends(day("2025-12-15"), day("2026-03-02")),
            vec![day("2026-01-31"), day("2026-02-28")]
        );
    }

    #[test]
    fn mpf_last_month_reads_the_latest_row_in_the_previous_month() {
        let history = [
            mpf_point("2026-08-10", 100.0, 100.0),
            mpf_point("2026-08-30", 100.0, 112.0),
            mpf_point("2026-09-05", 100.0, 115.0),
        ];
        let last = mpf_last_month(&history, day("2026-09-17")).unwrap();
        assert!(approx_eq(last.rate.unwrap(), 0.12));
        assert!(approx_eq(last.gain, 12.0));
        // Only a September row: August has nothing to report.
        assert!(mpf_last_month(&history[2..], day("2026-09-17")).is_none());
    }

    #[test]
    fn mpf_max_tracks_rate_and_gain_independently() {
        let history = [
            mpf_point("2026-03-10", 100.0, 148.0), // rate 48%, gain 48
            mpf_point("2026-08-10", 120.0, 180.0), // rate 50%, gain 60
        ];
        let current = mpf_point("2026-10-01", 200.0, 300.0); // rate 50%, gain 100
        let max = mpf_max(&history, Some(current), None, None);
        assert!(approx_eq(max.rate.unwrap(), 0.5));
        assert!(approx_eq(max.gain, 100.0));
    }

    #[test]
    fn mpf_max_is_floored_by_the_seeded_marks() {
        let current = mpf_point("2026-09-17", 100.0, 110.0);
        let max = mpf_max(&[], Some(current), Some(0.4792), Some(278_899.91));
        assert!(approx_eq(max.rate.unwrap(), 0.4792));
        assert!(approx_eq(max.gain, 278_899.91));
    }

    #[test]
    fn mpf_totals_carry_an_untouched_account_forward() {
        // Account A updated in Aug and Oct; B only ever recorded in Aug.
        let a = MpfAccountFacts {
            created_on: day("2026-08-01"),
            contributions: 100.0,
            balance: 150.0,
            seed_max_rate: None,
            seed_max_gain: None,
            history: vec![
                mpf_point("2026-08-28", 100.0, 110.0),
                mpf_point("2026-10-05", 100.0, 150.0),
            ],
        };
        let b = MpfAccountFacts {
            created_on: day("2026-08-01"),
            contributions: 200.0,
            balance: 260.0,
            seed_max_rate: None,
            seed_max_gain: None,
            history: vec![mpf_point("2026-08-28", 200.0, 240.0)],
        };
        let totals = mpf_totals(&[a, b], day("2026-10-17"), None, None);

        // Current: buy 300, now 410, rate 110/300.
        assert!(approx_eq(totals.buy, 300.0));
        assert!(approx_eq(totals.now, 410.0));
        assert!(approx_eq(totals.rate.unwrap(), 110.0 / 300.0));
        // Last month-end (Sep 30): A as-of = Aug 28 row, B = its Aug row —
        // carried forward even though neither updated in September.
        let last = totals.last_month.unwrap();
        assert!(approx_eq(last.rate.unwrap(), 50.0 / 300.0));
        // Both maxima sit at today: gain 110, rate 110/300.
        assert!(approx_eq(totals.max.gain, 110.0));
        assert!(approx_eq(totals.max.rate.unwrap(), 110.0 / 300.0));
    }

    #[test]
    fn mpf_totals_exclude_an_account_before_it_existed() {
        let old = MpfAccountFacts {
            created_on: day("2026-01-01"),
            contributions: 100.0,
            balance: 150.0,
            seed_max_rate: None,
            seed_max_gain: None,
            history: vec![mpf_point("2026-01-31", 100.0, 120.0)],
        };
        let new = MpfAccountFacts {
            created_on: day("2026-09-20"),
            contributions: 50.0,
            balance: 50.0,
            seed_max_rate: None,
            seed_max_gain: None,
            history: vec![],
        };
        let totals = mpf_totals(&[old, new], day("2026-10-17"), None, None);
        // At Jan 31 only `old` existed: rate 0.2. Had `new`'s current values
        // leaked into that date the portfolio rate would have been 0.4667 and
        // become the max — instead the max is today's 50/150.
        assert!(approx_eq(totals.max.rate.unwrap(), 50.0 / 150.0));
        assert!(approx_eq(totals.max.gain, 50.0));
        // Last month-end (Sep 30): `new` existed (created Sep 20) with no
        // rows yet, so its current values stand in; `old` carries Jan's row.
        let last = totals.last_month.unwrap();
        assert!(approx_eq(last.rate.unwrap(), 20.0 / 150.0));
        assert!(approx_eq(last.gain, 20.0));
    }

    #[test]
    fn bond_active_until_maturity_date() {
        let maturity = day("2027-10-23");
        assert!(bond_active(maturity, day("2027-10-22")));
        // The deposit rule: maturity day itself already counts as ended.
        assert!(!bond_active(maturity, maturity));
        assert!(!bond_active(maturity, day("2027-10-24")));
    }

    #[test]
    fn coupon_status_unfixed_pending_received() {
        // 待定: announced figures still missing.
        assert_eq!(coupon_status(None, None, None), CouponStatus::PendingFix);
        assert_eq!(
            coupon_status(None, Some(0.04), None),
            CouponStatus::PendingFix
        );
        assert_eq!(
            coupon_status(None, Some(0.04), Some(200.55)),
            CouponStatus::Pending
        );
        // Received wins over every other state.
        assert_eq!(
            coupon_status(Some(1000.0), None, None),
            CouponStatus::Received
        );
        assert_eq!(
            coupon_status(Some(1000.0), Some(0.04), Some(200.55)),
            CouponStatus::Received
        );
    }

    #[test]
    fn coupon_expected_scales_per_10k_by_principal() {
        // The sheet's interest column: D × principal/10000 (×5 for 50000).
        assert!(approx_eq(
            coupon_expected(Some(199.45), 50000.0).unwrap(),
            997.25
        ));
        assert!(approx_eq(
            coupon_expected(Some(200.55), 50000.0).unwrap(),
            1002.75
        ));
        assert_eq!(coupon_expected(None, 50000.0), None);
    }

    #[test]
    fn coupon_variance_is_received_minus_expected() {
        assert!(approx_eq(
            coupon_variance(Some(1000.0), Some(997.25)).unwrap(),
            2.75
        ));
        assert!(approx_eq(
            coupon_variance(Some(997.25), Some(1000.0)).unwrap(),
            -2.75
        ));
        assert_eq!(coupon_variance(Some(1000.0), None), None);
        assert_eq!(coupon_variance(None, Some(997.25)), None);
    }

    #[test]
    fn validate_bond_rejects_bad_input() {
        let good = validate_bond(BondInput {
            label: "silver bond",
            issue_no: Some("03GB2710R"),
            principal: 50000.0,
            maturity_date: "2027-10-23",
        })
        .expect("valid bond");
        assert_eq!(good.issue_no.as_deref(), Some("03GB2710R"));

        for (label, principal, maturity_date) in [
            ("", 50000.0, "2027-10-23"),
            ("silver bond", 0.0, "2027-10-23"),
            ("silver bond", -1.0, "2027-10-23"),
            ("silver bond", 50000.0, "23/10/2027"),
        ] {
            assert!(
                validate_bond(BondInput {
                    label,
                    issue_no: None,
                    principal,
                    maturity_date,
                })
                .is_err(),
                "expected rejection for {label}/{principal}/{maturity_date}"
            );
        }
    }

    #[test]
    fn validate_coupon_allows_unfixed_and_rejects_bad_input() {
        // 待定 coupon: no rate, no per-10k — still valid.
        let unfixed = validate_coupon(CouponInput {
            pay_date: "2027-10-23",
            fixing_date: None,
            annual_rate: None,
            per_10k: None,
            received_amount: None,
        })
        .expect("unfixed coupon is valid");
        assert_eq!(unfixed.annual_rate, None);

        assert!(
            validate_coupon(CouponInput {
                pay_date: "not-a-date",
                fixing_date: None,
                annual_rate: None,
                per_10k: None,
                received_amount: None,
            })
            .is_err()
        );
        assert!(
            validate_coupon(CouponInput {
                pay_date: "2027-10-23",
                fixing_date: Some("bad"),
                annual_rate: None,
                per_10k: None,
                received_amount: None,
            })
            .is_err()
        );
        assert!(
            validate_coupon(CouponInput {
                pay_date: "2027-10-23",
                fixing_date: None,
                annual_rate: Some(1.5),
                per_10k: None,
                received_amount: None,
            })
            .is_err()
        );
        assert!(
            validate_coupon(CouponInput {
                pay_date: "2027-10-23",
                fixing_date: None,
                annual_rate: None,
                per_10k: Some(-5.0),
                received_amount: None,
            })
            .is_err()
        );
    }

    fn aia_facts(
        premium: f64,
        value: f64,
        withdrew: f64,
        excluded: bool,
        in_account: bool,
    ) -> AiaPolicyFacts {
        AiaPolicyFacts {
            premium_usd: premium,
            value_usd: value,
            withdrew_usd: withdrew,
            excluded,
            in_account,
        }
    }

    #[test]
    fn aia_balance_pct_counts_withdrawals_and_skips_zero_premium() {
        let pct = aia_balance_pct(7845.67, 127.86, 6000.0).expect("pct");
        assert!((pct - 0.3289216667).abs() < 1e-9);
        assert_eq!(aia_balance_pct(500.0, 0.0, 0.0), None);
    }

    #[test]
    fn aia_totals_respect_the_flags() {
        // The sheet's rows: irene 20% excluded from totals, irene 年金 out of
        // display value but counted.
        let policies = [
            aia_facts(24960.0, 14284.35, 0.0, false, true),
            aia_facts(6000.0, 7845.67, 127.86, false, true),
            aia_facts(5000.0, 245.47, 0.0, true, true), // irene 20%
            aia_facts(3823.98, 2231.93, 0.0, false, false), // irene 年金
        ];
        let totals = aia_totals(&policies, Some(7.84522932));
        assert_eq!(totals.premium, 34783.98);
        assert_eq!(totals.value, 24361.95);
        assert_eq!(totals.withdrew, 127.86);
        // display value covers in-account rows including the excluded one.
        assert_eq!(totals.display_value, 22375.49);
        let pct = totals.balance_pct.expect("balance pct");
        assert!((pct - (24361.95 + 127.86 - 34783.98) / 34783.98).abs() < 1e-9);
        assert_eq!(totals.premium_hkd, Some(34783.98 * 7.84522932));
    }

    #[test]
    fn aia_totals_overall_return_matches_the_sheet() {
        let policies = [aia_facts(124781.98, 89260.78, 379.07, false, true)];
        let totals = aia_totals(&policies, None);
        let pct = totals.balance_pct.expect("balance pct");
        assert!((pct - -0.2816282).abs() < 1e-6);
        assert_eq!(totals.premium_hkd, None, "no rate -> no HKD figures");
    }

    #[test]
    fn next_premium_due_picks_the_earliest_future_date() {
        let today = day("2026-09-18");
        let dates = [day("2028-01-15"), day("2027-07-01"), day("2025-03-01")];
        assert_eq!(
            next_premium_due(dates.into_iter(), today),
            Some(day("2027-07-01"))
        );
        let past = [day("2025-03-01")];
        assert_eq!(next_premium_due(past.into_iter(), today), None);
    }

    #[test]
    fn aia_payment_applies_all_three_updates() {
        let outcome = apply_aia_payment(16640.0, Some(3.0), Some(day("2026-07-01")), 8320.0, None);
        assert_eq!(outcome.premium_usd, 24960.0);
        assert_eq!(outcome.remaining_years, Some(2.0));
        assert_eq!(outcome.next_pay_date, Some(day("2027-07-01")));
    }

    #[test]
    fn aia_payment_uses_the_submitted_date_and_handles_unset_fields() {
        let outcome = apply_aia_payment(
            100.0,
            None,
            Some(day("2026-07-01")),
            50.0,
            Some(day("2027-01-15")),
        );
        assert_eq!(outcome.next_pay_date, Some(day("2027-01-15")));
        assert_eq!(outcome.remaining_years, None, "unset stays unset");

        let no_date = apply_aia_payment(0.0, Some(0.5), None, 10.0, None);
        assert_eq!(no_date.next_pay_date, None);
        assert_eq!(no_date.remaining_years, Some(0.0), "floors at zero");
    }

    #[test]
    fn aia_withdrawal_and_undo_adjust_the_cumulative_figures() {
        assert_eq!(apply_aia_withdrawal(127.86, 7032.0), 7159.86);
        assert_eq!(undo_aia_event_amount(24960.0, 8320.0), 16640.0);
        assert!((undo_aia_event_amount(7159.86, 7032.0) - 127.86).abs() < 1e-9);
    }

    #[test]
    fn validate_aia_policy_rejects_bad_input() {
        let base = AiaPolicyInput {
            label: "年金 - 2024 - 2029",
            policy_no: Some("B632611401"),
            next_pay_date: Some("2027-07-01"),
            premium_usd: 24960.0,
            value_usd: 14284.35,
            remaining_years: Some(2.0),
            withdrew_usd: 0.0,
            note: None,
            link: None,
            excluded: false,
            in_account: true,
        };
        assert!(validate_aia_policy(base.clone()).is_ok());
        assert!(
            validate_aia_policy(AiaPolicyInput {
                label: "  ",
                ..base.clone()
            })
            .is_err()
        );
        assert!(
            validate_aia_policy(AiaPolicyInput {
                premium_usd: -1.0,
                ..base.clone()
            })
            .is_err()
        );
        assert!(
            validate_aia_policy(AiaPolicyInput {
                next_pay_date: Some("not-a-date"),
                ..base.clone()
            })
            .is_err()
        );
        assert!(
            validate_aia_policy(AiaPolicyInput {
                link: Some("https://example.com/policy.pdf"),
                ..base.clone()
            })
            .is_ok()
        );
        assert!(
            validate_aia_policy(AiaPolicyInput {
                link: Some("javascript:alert(1)"),
                ..base
            })
            .is_err()
        );
    }

    #[test]
    fn validate_aia_event_rejects_bad_input() {
        use crate::models::AiaEventKind;
        let valid = validate_aia_event(AiaEventInput {
            kind: AiaEventKind::Payment,
            event_date: "2026-07-01",
            amount_usd: 8320.0,
            next_pay_date: None,
        });
        assert!(valid.is_ok());
        assert!(
            validate_aia_event(AiaEventInput {
                kind: AiaEventKind::Withdrawal,
                event_date: "2026-07-01",
                amount_usd: 0.0,
                next_pay_date: None,
            })
            .is_err()
        );
        assert!(
            validate_aia_event(AiaEventInput {
                kind: AiaEventKind::Payment,
                event_date: "bad",
                amount_usd: 10.0,
                next_pay_date: None,
            })
            .is_err()
        );
    }

    // --- month stat (月結) ---

    fn month_row(month: &str, start_cash: Option<f64>, salary: Option<f64>) -> MonthStatRow {
        MonthStatRow {
            month: chrono::NaiveDate::parse_from_str(month, "%Y-%m-%d").expect("month"),
            start_cash,
            salary,
            total_assets: None,
            liquid_assets: None,
            end_cash_override: None,
            interest: 0.0,
            pool_input: 0.0,
        }
    }

    fn item(category: MonthItemCategory, amount: f64) -> MonthItemFacts {
        MonthItemFacts {
            category,
            amount,
            exclude_from_living: false,
        }
    }

    fn ym(month: &str) -> chrono::NaiveDate {
        chrono::NaiveDate::parse_from_str(month, "%Y-%m-%d").expect("month")
    }

    #[test]
    fn validate_deposit_start_date() {
        let base = DepositInput {
            label: Some("SC-9632"),
            bank: None,
            principal: Some(110000.0),
            rate: Some(0.028),
            interest: Some(993.0),
            start_date: None,
            end_date: "2026-10-12",
        };
        assert_eq!(
            validate_deposit(base.clone()).expect("valid").start_date,
            None
        );
        // A blank string counts as absent.
        assert_eq!(
            validate_deposit(DepositInput {
                start_date: Some("  "),
                ..base.clone()
            })
            .expect("blank start date")
            .start_date,
            None
        );
        assert_eq!(
            validate_deposit(DepositInput {
                start_date: Some("2026-06-17"),
                ..base.clone()
            })
            .expect("valid start date")
            .start_date
            .as_deref(),
            Some("2026-06-17")
        );
        let errors = validate_deposit(DepositInput {
            start_date: Some("12/10/2026"),
            ..base
        })
        .expect_err("malformed start date");
        assert!(errors.iter().any(|error| error.field == "start_date"));
    }

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

    #[test]
    fn live_totals_omit_rate_dependent_terms_without_a_rate() {
        let input = LiveTotalsInput {
            hk_market_value: 100.0,
            us_market_value: 10.0,
            usd_hkd_rate: Some(7.8),
            deposits_active_principal: 200.0,
            bonds_active_principal: 30.0,
            aia_value_usd: 5.0,
            mpf_balance: 40.0,
            manual_assets_sum: 6.0,
            cash_sum: 7.0,
            ibkr_hkd_cash: 2.0,
            ibkr_usd_cash: 1.0,
            pool_balance: 8.0,
        };
        let totals = live_totals(&input);
        // total = 100 + (10+1)×7.8 + 2 + 200 + 30 + 39 + 40 + 6 + 7
        assert!(approx_eq(totals.total_assets, 509.8));
        // liquid = 100 + 200 + 7 + 30 + (10+1)×7.8 + 2 − 8
        assert!(approx_eq(totals.liquid_assets, 416.8));

        let no_rate = live_totals(&LiveTotalsInput {
            usd_hkd_rate: None,
            ..input
        });
        // Without a rate the US and AIA terms drop out; HKD cash stays.
        assert!(approx_eq(no_rate.total_assets, 385.0));
        assert!(approx_eq(no_rate.liquid_assets, 331.0));
    }

    fn suggestion_events() -> SuggestionEvents {
        SuggestionEvents {
            deposits: vec![
                SuggestionDeposit {
                    id: 1,
                    start_date: Some(ym("2026-10-05")),
                    end_date: ym("2027-04-05"),
                    principal: 100000.0,
                    interest: 1200.0,
                    label: Some("SC-9632".to_string()),
                    received: false,
                },
                SuggestionDeposit {
                    id: 2,
                    start_date: None,
                    end_date: ym("2026-10-20"),
                    principal: 50000.0,
                    interest: 600.0,
                    label: None,
                    received: true,
                },
                // Ended in the month but not yet 收訖: previews in the
                // breakdown, does not count in the derived 利息.
                SuggestionDeposit {
                    id: 8,
                    start_date: None,
                    end_date: ym("2026-10-25"),
                    principal: 0.0,
                    interest: 999.0,
                    label: Some("HS-99".to_string()),
                    received: false,
                },
            ],
            trades: vec![
                SuggestionTrade {
                    id: 3,
                    date: ym("2026-10-08"),
                    kind: TradeType::Buy,
                    total: 96523.15,
                    code: "中國銀行".to_string(),
                },
                SuggestionTrade {
                    id: 4,
                    date: ym("2026-10-09"),
                    kind: TradeType::Sell,
                    total: 20000.0,
                    code: "恒生".to_string(),
                },
            ],
            dividends: vec![
                SuggestionReceipt {
                    id: 5,
                    pay_date: ym("2026-10-12"),
                    amount: Some(1500.0),
                    label: "中國銀行".to_string(),
                    received: true,
                },
                // Pending dividend with an estimate: previews, doesn't count.
                SuggestionReceipt {
                    id: 9,
                    pay_date: ym("2026-10-20"),
                    amount: Some(720.0),
                    label: "長江基建".to_string(),
                    received: false,
                },
            ],
            coupons: vec![
                SuggestionReceipt {
                    id: 6,
                    pay_date: ym("2026-10-15"),
                    amount: Some(2000.0),
                    label: "silver bond".to_string(),
                    received: true,
                },
                // 待定 coupon: previews with no amount.
                SuggestionReceipt {
                    id: 10,
                    pay_date: ym("2026-10-23"),
                    amount: None,
                    label: "silver bond".to_string(),
                    received: false,
                },
            ],
            aia_payments: vec![SuggestionAiaPayment {
                id: 7,
                date: ym("2026-10-01"),
                amount_usd: 8320.0,
                policy: "年金".to_string(),
            }],
            pool_input: 4142.66,
            usd_hkd_rate: Some(7.8),
        }
    }

    #[test]
    fn suggestions_cover_each_event_kind() {
        let events = suggestion_events();
        let suggestions =
            build_suggestions("2026-10-01", &events, &HashSet::new(), &HashSet::new());
        let by_key: HashMap<&str, &MonthSuggestion> = suggestions
            .iter()
            .map(|suggestion| (suggestion.auto_key.as_str(), suggestion))
            .collect();
        // dep-start only exists for the deposit carrying a start_date.
        assert!(approx_eq(by_key["dep-start:1"].amount, -100000.0));
        assert!(!by_key.contains_key("dep-start:2"));
        assert!(approx_eq(by_key["dep-end:2"].amount, 50600.0));
        assert!(!by_key.contains_key("dep-end:1")); // ends in 2027-04
        assert!(approx_eq(by_key["trade:3"].amount, -96523.15));
        assert!(approx_eq(by_key["trade:4"].amount, 20000.0));
        assert!(approx_eq(by_key["div:5"].amount, 1500.0));
        assert!(approx_eq(by_key["coupon:6"].amount, 2000.0));
        // Pending receipts preview in the breakdown but never suggest items.
        assert!(!by_key.contains_key("div:9"));
        assert!(!by_key.contains_key("coupon:10"));
        assert_eq!(by_key["aia-pay:7"].category, MonthItemCategory::ExtraSpend);
        assert!(approx_eq(by_key["aia-pay:7"].amount, 8320.0 * 7.8));
        assert!(approx_eq(by_key["pool-input"].amount, -4142.66));
    }

    #[test]
    fn suggestions_exclude_stored_and_dismissed_keys() {
        let events = suggestion_events();
        let stored = HashSet::from(["dep-end:2".to_string(), "pool-input".to_string()]);
        let dismissed = HashSet::from(["trade:3".to_string()]);
        let suggestions = build_suggestions("2026-10-01", &events, &stored, &dismissed);
        let keys: HashSet<&str> = suggestions
            .iter()
            .map(|suggestion| suggestion.auto_key.as_str())
            .collect();
        assert!(!keys.contains("dep-end:2"));
        assert!(!keys.contains("pool-input"));
        assert!(!keys.contains("trade:3"));
        assert!(keys.contains("dep-start:1"));
    }

    #[test]
    fn aia_suggestion_needs_a_rate_and_pool_input_needs_a_positive_amount() {
        let mut events = suggestion_events();
        events.usd_hkd_rate = None;
        let suggestions =
            build_suggestions("2026-10-01", &events, &HashSet::new(), &HashSet::new());
        assert!(
            !suggestions
                .iter()
                .any(|suggestion| suggestion.auto_key == "aia-pay:7")
        );

        events.pool_input = 0.0;
        let suggestions =
            build_suggestions("2026-10-01", &events, &HashSet::new(), &HashSet::new());
        assert!(
            !suggestions
                .iter()
                .any(|suggestion| suggestion.auto_key == "pool-input")
        );
    }

    #[test]
    fn suggestions_are_cut_off_before_the_current_month() {
        let today = ym("2026-10-23");
        assert!(suggestions_enabled("2026-10-01", today));
        assert!(suggestions_enabled("2026-11-01", today));
        assert!(!suggestions_enabled("2026-09-01", today));
        assert!(!suggestions_enabled("not-a-month", today));
    }

    #[test]
    fn interest_components_list_deposit_coupon_and_dividend_receipts() {
        let events = suggestion_events();
        let components = interest_components("2026-10-01", &events);
        // Received: deposit 2 + coupon 6 + dividend 5; pending previews:
        // deposit 8, dividend 9, 待定 coupon 10.
        assert_eq!(components.len(), 6);
        let received: Vec<&InterestComponent> = components
            .iter()
            .filter(|component| component.received)
            .collect();
        let by_source: HashMap<&str, f64> = received
            .iter()
            .map(|component| (component.source.as_str(), component.amount.unwrap_or(0.0)))
            .collect();
        assert!(approx_eq(by_source["deposit"], 600.0));
        assert!(approx_eq(by_source["coupon"], 2000.0));
        assert!(approx_eq(by_source["dividend"], 1500.0));
        let pending: Vec<&InterestComponent> = components
            .iter()
            .filter(|component| !component.received)
            .collect();
        assert_eq!(pending.len(), 3);
        assert_eq!(pending[0].source, "deposit");
        assert!(approx_eq(pending[0].amount.unwrap_or(0.0), 999.0));
        // The 待定 coupon previews with no figure; the dividend uses its estimate.
        assert!(pending[1].amount.is_none());
        assert_eq!(pending[2].source, "dividend");
        assert!(approx_eq(pending[2].amount.unwrap_or(0.0), 720.0));
        // auto_interest sums received components only.
        assert!(approx_eq(auto_interest(ym("2026-10-01"), &events), 4100.0));
        assert!(interest_components("2026-11-01", &events).is_empty());
    }
}
