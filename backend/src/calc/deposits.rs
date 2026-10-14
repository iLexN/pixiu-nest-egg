use super::*;

use chrono::Datelike;
use serde::Serialize;
use utoipa::ToSchema;

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
    /// Receipt flag: live callers pass `received_at.is_some()`. The retired
    /// parity checker passed `end_date <= today` to reproduce the sheet's
    /// end-date rule.
    pub received: bool,
}

/// The sheet's total column: `利息 + input`, blanks counting as 0.
pub fn deposit_total(principal: Option<f64>, interest: Option<f64>) -> f64 {
    principal.unwrap_or(0.0) + interest.unwrap_or(0.0)
}

/// The receipt rule: a deposit counts until 收訖, whatever its `end_date`.
pub fn deposit_active(deposit: &DepositFacts<'_>) -> bool {
    !deposit.received
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
    /// 定期 column: Σ principal (the sheet sums its `input` column here, not
    /// principal + interest).
    pub payout: f64,
    /// The sheet's Total column: 利息 + 定期 = principal + interest.
    pub total: f64,
}

/// A year's month table: always all 12 months, like the sheet.
#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
pub struct YearRollup {
    pub year: i32,
    pub months: Vec<YearMonthRow>,
}

/// Σ principal/interest/total over the unreceived deposits — the same set
/// the 未到期定期 list shows.
pub fn active_totals(deposits: &[DepositFacts<'_>]) -> ActiveTotals {
    let mut totals = ActiveTotals {
        principal: 0.0,
        interest: 0.0,
        total: 0.0,
    };
    for deposit in deposits.iter().filter(|deposit| deposit_active(deposit)) {
        totals.principal += deposit.principal.unwrap_or(0.0);
        totals.interest += deposit.interest.unwrap_or(0.0);
        totals.total += deposit_total(deposit.principal, deposit.interest);
    }
    totals
}

/// Unreceived deposits grouped by end (year, month), in chronological order.
/// The sheet buckets by month number only; grouping by (year, month) gives the
/// same result while the active span is under a year.
pub fn active_month_rollup(deposits: &[DepositFacts<'_>]) -> Vec<ActiveMonthBucket> {
    let mut buckets: Vec<ActiveMonthBucket> = Vec::new();
    for deposit in deposits.iter().filter(|deposit| deposit_active(deposit)) {
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

/// Unreceived deposits grouped by bank, ordered by descending principal.
/// Deposits without a bank land in their own bucket, matching how the sheet's
/// `SUMIF` rows only count labelled deposits.
pub fn bank_rollup(deposits: &[DepositFacts<'_>]) -> Vec<BankRollup> {
    let mut groups: Vec<BankRollup> = Vec::new();
    for deposit in deposits.iter().filter(|deposit| deposit_active(deposit)) {
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
        month.payout += deposit.principal.unwrap_or(0.0);
        month.total = month.interest + month.payout;
    }
    years.sort_by_key(|rollup| rollup.year);
    years
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calc::test_helpers::*;

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

    fn deposit<'a>(
        bank: Option<&'a str>,
        principal: Option<f64>,
        interest: Option<f64>,
        end_date: &str,
        received: bool,
    ) -> DepositFacts<'a> {
        DepositFacts {
            bank,
            principal,
            interest,
            end_date: day(end_date),
            received,
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
    fn deposit_status_follows_receipt_not_the_end_date() {
        assert!(deposit_active(&deposit(
            None,
            Some(110000.0),
            None,
            "2026-01-05",
            false
        )));
        assert!(!deposit_active(&deposit(
            None,
            Some(110000.0),
            None,
            "2027-01-05",
            true
        )));
    }

    #[test]
    fn deposit_total_counts_blanks_as_zero() {
        assert!(approx_eq(deposit_total(Some(140000.0), None), 140000.0));
        assert!(approx_eq(deposit_total(None, Some(539.25)), 539.25));
        assert!(approx_eq(deposit_total(None, None), 0.0));
    }

    /// The workbook's current active set: SC-9632 + SC-7006 ending 2026-10-12,
    /// SC-0044 ending 2026-11-02, HS-88 ending 2026-11-17, SC-3615 ending
    /// 2027-01-04, plus ended and interest-only rows that must not appear.
    /// The fixture marks rows ended by the workbook's 2026-09-15 snapshot as
    /// received, reproducing the sheet's end-date filter.
    fn workbook_deposits<'a>() -> Vec<DepositFacts<'a>> {
        let snapshot = day("2026-09-15");
        let row = |bank, principal, interest, end_date: &str| {
            deposit(
                bank,
                principal,
                interest,
                end_date,
                day(end_date) <= snapshot,
            )
        };
        vec![
            row(Some("HS"), Some(60000.0), Some(362.96), "2026-01-05"),
            row(None, None, Some(15.27), "2026-04-20"),
            row(Some("SC"), Some(110000.0), Some(993.0), "2026-10-12"),
            row(Some("SC"), Some(35000.0), Some(285.0), "2026-10-12"),
            row(Some("SC"), Some(140000.0), None, "2026-11-02"),
            row(Some("HS"), Some(80000.0), Some(604.93), "2026-11-17"),
            row(None, None, Some(2097.0), "2026-08-18"),
            row(Some("SC"), Some(80000.0), Some(807.0), "2027-01-04"),
        ]
    }

    #[test]
    fn matured_unreceived_deposit_counts_in_every_aggregate() {
        let deposits = vec![
            deposit(Some("SC"), Some(110000.0), Some(993.0), "2026-01-05", false),
            deposit(Some("SC"), Some(80000.0), Some(807.0), "2027-01-04", false),
        ];
        let totals = active_totals(&deposits);
        assert!(approx_eq(totals.principal, 190000.0));
        let months = active_month_rollup(&deposits);
        assert_eq!(months.len(), 2);
        assert!(approx_eq(months[0].principal, 110000.0));
        let banks = bank_rollup(&deposits);
        assert_eq!(banks.len(), 1);
        assert!(approx_eq(banks[0].principal, 190000.0));
    }

    #[test]
    fn received_deposit_with_a_future_end_date_counts_in_no_aggregate() {
        let deposits = vec![deposit(
            Some("SC"),
            Some(110000.0),
            Some(993.0),
            "2027-01-04",
            true,
        )];
        assert_eq!(active_totals(&deposits).principal, 0.0);
        assert!(active_month_rollup(&deposits).is_empty());
        assert!(bank_rollup(&deposits).is_empty());
    }

    #[test]
    fn active_totals_match_the_workbook() {
        let totals = active_totals(&workbook_deposits());
        assert!(approx_eq(totals.principal, 445000.0));
        assert!(approx_eq(totals.interest, 2689.93));
        assert!(approx_eq(totals.total, 447689.93));
    }

    #[test]
    fn active_month_rollup_matches_the_workbook() {
        let months = active_month_rollup(&workbook_deposits());
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
        let banks = bank_rollup(&workbook_deposits());
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
        // August 2026 holds the unlabeled 2097 interest-only row — no
        // principal, so the 定期 column sums to zero like the sheet's.
        assert!(approx_eq(y2026.months[7].interest, 2097.0));
        assert!(approx_eq(y2026.months[7].payout, 0.0));
        let y2027 = &years[1];
        // 表_2027定期 1月: 利息 807, 定期 80000, Total 80807.
        assert!(approx_eq(y2027.months[0].interest, 807.0));
        assert!(approx_eq(y2027.months[0].payout, 80000.0));
        assert!(approx_eq(y2027.months[0].total, 80807.0));
    }

    // --- 3.6 validation ---

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
}
