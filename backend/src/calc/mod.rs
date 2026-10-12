//! Pure calculation core: no database, no HTTP types.
//!
//! Money is `f64` and is never rounded here; rounding belongs to display.
//! The formulas deliberately mirror the spreadsheet being replaced, including
//! 加權平均買入單價 dividing by shares *bought* rather than shares held.

mod aia;
mod bonds;
mod deposits;
mod dividends;
mod forecast;
mod months;
mod mpf;
mod overview;
mod stocks;
mod suggestions;
mod year_review;
mod yearly;

pub use aia::*;
pub use bonds::*;
pub use deposits::*;
pub use dividends::*;
pub use forecast::*;
pub use months::*;
pub use mpf::*;
pub use overview::*;
pub use stocks::*;
pub use suggestions::*;
pub use year_review::*;
pub use yearly::*;

use chrono::Datelike;
use serde::Serialize;

/// Relative tolerance used when comparing money figures.
pub const TOLERANCE: f64 = 1e-6;

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

fn has_note(note: Option<&str>) -> bool {
    note.map(|n| !n.trim().is_empty()).unwrap_or(false)
}

/// Relative comparison used by the parity check and the tests.
pub fn approx_eq(left: f64, right: f64) -> bool {
    let scale = left.abs().max(right.abs()).max(1.0);
    (left - right).abs() <= TOLERANCE * scale
}

fn first_of_month(date: chrono::NaiveDate) -> chrono::NaiveDate {
    chrono::NaiveDate::from_ymd_opt(date.year(), date.month(), 1).expect("valid month")
}

fn parse_month(month: &str) -> Option<chrono::NaiveDate> {
    chrono::NaiveDate::parse_from_str(month, "%Y-%m-%d").ok()
}

fn in_month(date: chrono::NaiveDate, month: chrono::NaiveDate) -> bool {
    date.year() == month.year() && date.month() == month.month()
}

#[cfg(test)]
pub(crate) mod test_helpers {
    use super::*;

    use crate::models::{MonthItemCategory, TradeType};

    pub(crate) fn buy(shares: f64, total: f64) -> TradeFacts {
        TradeFacts {
            trade_type: TradeType::Buy,
            shares,
            total,
        }
    }

    pub(crate) fn sell(shares: f64) -> TradeFacts {
        TradeFacts {
            trade_type: TradeType::Sell,
            shares,
            total: 0.0,
        }
    }

    pub(crate) fn day(date: &str) -> chrono::NaiveDate {
        chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").expect("date")
    }

    pub(crate) fn dated_buy(
        date: &str,
        shares: f64,
        total: f64,
    ) -> (chrono::NaiveDate, TradeFacts) {
        (day(date), buy(shares, total))
    }

    pub(crate) fn dividend_fact<'a>(
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

    pub(crate) fn snapshot_year(
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

    pub(crate) fn month_row(
        month: &str,
        start_cash: Option<f64>,
        salary: Option<f64>,
    ) -> MonthStatRow {
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

    pub(crate) fn item(category: MonthItemCategory, amount: f64) -> MonthItemFacts {
        MonthItemFacts {
            category,
            amount,
            exclude_from_living: false,
        }
    }

    pub(crate) fn ym(month: &str) -> chrono::NaiveDate {
        chrono::NaiveDate::parse_from_str(month, "%Y-%m-%d").expect("month")
    }
}
