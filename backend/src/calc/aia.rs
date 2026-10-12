use super::*;

use serde::Serialize;
use utoipa::ToSchema;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calc::test_helpers::*;

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
}
