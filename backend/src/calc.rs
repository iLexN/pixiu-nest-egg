//! Pure calculation core: no database, no HTTP types.
//!
//! Money is `f64` and is never rounded here; rounding belongs to display.
//! The formulas deliberately mirror the spreadsheet being replaced, including
//! 加權平均買入單價 dividing by shares *bought* rather than shares held.

use std::collections::HashMap;

use chrono::Datelike;
use serde::Serialize;

use crate::models::{InputMode, MpfFigures, TradeType};

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
        if let Some(value) = value {
            if !value.is_finite() || value < 0.0 {
                errors.push(FieldError::new(
                    field,
                    format!("{name} must not be negative"),
                ));
            }
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
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DividendStockTotal {
    pub code: String,
    pub received: f64,
}

/// A year's received-dividend rollup, like the 回報率 sheet's per-stock
/// SUMIF block over the J–O columns.
#[derive(Debug, Clone, PartialEq, Serialize)]
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
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct YearSnapshot {
    pub invested: Option<f64>,
    pub cost: Option<f64>,
    pub market_value: Option<f64>,
    pub updated_at: String,
}

/// One row of the per-market yearly summary table: the sheet's B–M year
/// block (net invested, sold P/L, 成本, 報酬率s, year-end value, 派息, month,
/// and the two year-over-year changes).
#[derive(Debug, Clone, PartialEq, Serialize)]
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
    /// Reserved for realized sell P/L; not computed yet.
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
            sold_pl: None,
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
#[derive(Debug, Clone, Serialize)]
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
/// moments.
pub fn mpf_max(
    history: &[MpfPoint],
    current: MpfPoint,
    seed_max_rate: Option<f64>,
    seed_max_gain: Option<f64>,
) -> MpfFigures {
    let current_figures = mpf_figures(current.contributions, current.balance);
    let mut rate = seed_max_rate;
    let mut gain = seed_max_gain;
    for point in history.iter().copied().chain([current]) {
        let figures = mpf_figures(point.contributions, point.balance);
        rate = fold_max(rate, figures.rate);
        gain = fold_max(gain, Some(figures.gain));
    }
    MpfFigures {
        rate,
        gain: gain.unwrap_or(current_figures.gain),
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
        let max = mpf_max(&history, current, None, None);
        assert!(approx_eq(max.rate.unwrap(), 0.5));
        assert!(approx_eq(max.gain, 100.0));
    }

    #[test]
    fn mpf_max_is_floored_by_the_seeded_marks() {
        let current = mpf_point("2026-09-17", 100.0, 110.0);
        let max = mpf_max(&[], current, Some(0.4792), Some(278_899.91));
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
}
