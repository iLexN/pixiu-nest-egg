use super::*;

use crate::models::CouponStatus;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calc::test_helpers::*;

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
}
