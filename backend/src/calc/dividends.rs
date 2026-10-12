use super::*;

use chrono::Datelike;
use serde::Serialize;
use utoipa::ToSchema;

use crate::models::TradeType;

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
    pub withholding_tax: Option<f64>,
    /// Input-only gross 派息, checked against the tax during validation.
    pub gross_amount: Option<f64>,
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
    pub withholding_tax: Option<f64>,
}

/// Net 實收派息 from the gross 派息 and the withheld tax.
pub fn net_from_gross(gross: f64, tax: Option<f64>) -> f64 {
    gross - tax.unwrap_or(0.0)
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
        ("withholding_tax", input.withholding_tax, "Tax"),
        ("gross_amount", input.gross_amount, "派息"),
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

    if let (Some(gross), Some(tax)) = (input.gross_amount, input.withholding_tax)
        && tax > gross
    {
        errors.push(FieldError::new(
            "withholding_tax",
            "Tax must not exceed 派息",
        ));
    }
    if input.withholding_tax.is_some() && input.received_amount.is_none() {
        errors.push(FieldError::new(
            "withholding_tax",
            "Tax requires a received 實收派息",
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
        withholding_tax: input.withholding_tax,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calc::test_helpers::*;

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
            withholding_tax: None,
            gross_amount: None,
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
    fn dividend_net_from_gross_subtracts_the_tax() {
        assert!(approx_eq(net_from_gross(12.76, Some(3.83)), 8.93));
        assert!(approx_eq(net_from_gross(12.76, None), 12.76));
    }

    #[test]
    fn dividend_rejects_tax_larger_than_gross() {
        let errors = validate_dividend(DividendInput {
            received_amount: Some(0.0),
            gross_amount: Some(10.0),
            withholding_tax: Some(12.0),
            ..dividend_input()
        })
        .expect_err("must fail");
        assert!(errors.iter().any(|e| e.field == "withholding_tax"));
    }

    #[test]
    fn dividend_rejects_tax_on_a_pending_record() {
        let errors = validate_dividend(DividendInput {
            withholding_tax: Some(3.83),
            ..dividend_input()
        })
        .expect_err("must fail");
        assert!(errors.iter().any(|e| e.field == "withholding_tax"));
    }

    #[test]
    fn dividend_rejects_negative_tax() {
        let errors = validate_dividend(DividendInput {
            received_amount: Some(8.93),
            withholding_tax: Some(-3.83),
            ..dividend_input()
        })
        .expect_err("must fail");
        assert!(errors.iter().any(|e| e.field == "withholding_tax"));
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
}
