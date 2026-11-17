use super::*;

use std::collections::HashSet;

use crate::models::{InterestComponent, MonthSuggestion, TradeType};

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
    /// The deposit's bank — maps its placement to a grid row (HS → 定期 HS,
    /// anything else → SC高息馬拉松).
    pub bank: Option<String>,
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
    /// breakdown and suggest at their expected figure, but count in the
    /// derived 利息 only once received.
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

/// An IBKR transfer as a suggestion source — cash-moving rows only
/// (`from_account_id` recorded); log-only legacy rows never reach this.
/// The forecast reuses the shape for its `ibkr_lines`, where legacy rows do
/// count.
#[derive(Debug, Clone)]
pub struct SuggestionIbkrTransfer {
    pub id: i64,
    pub date: chrono::NaiveDate,
    /// The transferred HKD amount (positive).
    pub amount: f64,
}

/// Every dated event the suggestion builder reads, plus the month's own
/// pool_input and the USD→HKD rate.
#[derive(Debug, Default)]
pub struct SuggestionEvents {
    pub deposits: Vec<SuggestionDeposit>,
    pub trades: Vec<SuggestionTrade>,
    pub dividends: Vec<SuggestionReceipt>,
    pub coupons: Vec<SuggestionReceipt>,
    /// Bonds whose maturity date falls in the month — the `bond-end:<id>`
    /// suggestion source.
    pub bonds: Vec<SuggestionBond>,
    pub aia_payments: Vec<SuggestionAiaPayment>,
    /// Cash-moving IBKR transfers (the `ibkr:<id>` suggestion source).
    pub ibkr_transfers: Vec<SuggestionIbkrTransfer>,
    /// The month row's own pool_input.
    pub pool_input: f64,
    pub usd_hkd_rate: Option<f64>,
}

/// Suggestion actions exist only from the previous month on: candidates are
/// listed for every month, but accept/dismiss/undismiss applies only while
/// the month is recent enough to still be settled. Older months' ghost rows
/// are informational — imported history already carries its 調整 as single
/// items, so acting on a candidate there would double-count.
pub fn suggestion_actions_enabled(month: &str, today: chrono::NaiveDate) -> bool {
    let previous_month = first_of_month(today) - chrono::Months::new(1);
    parse_month(month).is_some_and(|month| month >= previous_month)
}

/// The candidate items for one month, minus keys already stored (an accepted
/// candidate is a real item, not a ghost). Dismissed keys stay listed,
/// flagged `dismissed` — an ignored, never-counted row. Every event kind
/// builds its Candidate through `calc::candidates` — the same definitions
/// the write paths record through `record_candidate`.
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
    let mut built = Vec::new();

    for deposit in &events.deposits {
        built.push(dep_start(deposit));
        built.push(dep_end(deposit));
    }
    for event in &events.trades {
        built.push(trade(event));
    }
    // Coupons and HK dividends suggest before 收訖 — like dep-end and
    // bond-end — at the received amount, else the expected/estimated figure.
    // A receipt with no known amount (待定 coupon, estimate-less dividend)
    // has nothing to suggest.
    for receipt in &events.dividends {
        built.push(dividend(receipt));
    }
    for receipt in &events.coupons {
        built.push(coupon(receipt));
    }
    for bond in &events.bonds {
        built.push(bond_end(bond));
    }
    // A cash-moving IBKR transfer suggests only while its month row missed
    // the auto item — normally `record_candidate` stores `ibkr:<id>` at
    // transfer time and the stored-key filter suppresses this candidate.
    for transfer in &events.ibkr_transfers {
        built.push(ibkr(transfer));
    }
    for payment in &events.aia_payments {
        built.push(aia_pay(payment, events.usd_hkd_rate));
    }
    built.push(pool_input(month, events.pool_input));

    built
        .into_iter()
        .flatten()
        .filter(|candidate| candidate.month == month)
        .map(|candidate| {
            let auto_key = candidate.key.to_string();
            MonthSuggestion {
                dismissed: dismissed_keys.contains(&auto_key),
                auto_key,
                category: candidate.category,
                label: Some(candidate.label),
                amount: candidate.amount,
                source: candidate.key.source().to_string(),
            }
        })
        .filter(|suggestion| !stored_keys.contains(&suggestion.auto_key))
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
    use std::collections::HashMap;

    use super::*;
    use crate::calc::test_helpers::*;
    use crate::models::MonthItemCategory;

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
                    bank: None,
                },
                SuggestionDeposit {
                    id: 2,
                    start_date: None,
                    end_date: ym("2026-10-20"),
                    principal: 50000.0,
                    interest: 600.0,
                    label: None,
                    received: true,
                    bank: None,
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
                    bank: None,
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
            bonds: vec![
                // Maturing in the month, not yet 收訖: suggests at principal.
                SuggestionBond {
                    id: 13,
                    maturity_date: ym("2026-10-20"),
                    principal: 50000.0,
                    credited_amount: None,
                    label: "iBond".to_string(),
                },
                // Matures out of the month: never suggests.
                SuggestionBond {
                    id: 14,
                    maturity_date: ym("2026-11-20"),
                    principal: 70000.0,
                    credited_amount: None,
                    label: "green bond".to_string(),
                },
            ],
            aia_payments: vec![SuggestionAiaPayment {
                id: 7,
                date: ym("2026-10-01"),
                amount_usd: 8320.0,
                policy: "年金".to_string(),
            }],
            ibkr_transfers: vec![
                SuggestionIbkrTransfer {
                    id: 11,
                    date: ym("2026-10-17"),
                    amount: 50000.0,
                },
                // Dated outside the month: never suggests.
                SuggestionIbkrTransfer {
                    id: 12,
                    date: ym("2026-11-17"),
                    amount: 90000.0,
                },
            ],
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
        // Pending receipts suggest at their expected/estimated figure; a
        // 待定 coupon with no amount suggests nothing.
        assert!(approx_eq(by_key["div:9"].amount, 720.0));
        assert!(!by_key.contains_key("coupon:10"));
        assert_eq!(by_key["aia-pay:7"].category, MonthItemCategory::ExtraSpend);
        assert!(approx_eq(by_key["aia-pay:7"].amount, 8320.0 * 7.8));
        // A cash-moving IBKR transfer suggests `IBKR 轉入` of −amount, keyed
        // `ibkr:<id>`; the out-of-month one does not.
        assert!(approx_eq(by_key["ibkr:11"].amount, -50000.0));
        assert_eq!(by_key["ibkr:11"].label.as_deref(), Some("IBKR 轉入"));
        assert_eq!(by_key["ibkr:11"].category, MonthItemCategory::Adjustment);
        assert!(!by_key.contains_key("ibkr:12"));
        assert!(approx_eq(by_key["pool-input"].amount, 4142.66));
    }

    /// A maturing bond suggests `bond-end:<id>` at its principal before
    /// 收訖, then at the credited amount once one was banked.
    #[test]
    fn bond_end_suggests_at_principal_then_at_the_credited_amount() {
        let events = suggestion_events();
        let suggestions =
            build_suggestions("2026-10-01", &events, &HashSet::new(), &HashSet::new());
        let by_key: HashMap<&str, &MonthSuggestion> = suggestions
            .iter()
            .map(|suggestion| (suggestion.auto_key.as_str(), suggestion))
            .collect();
        let bond = by_key["bond-end:13"];
        assert_eq!(bond.source, "bond");
        assert_eq!(bond.category, MonthItemCategory::Adjustment);
        assert_eq!(bond.label.as_deref(), Some("iBond"));
        assert!(approx_eq(bond.amount, 50000.0));
        assert!(!by_key.contains_key("bond-end:14"));

        let mut events = suggestion_events();
        events.bonds[0].credited_amount = Some(49500.0);
        let suggestions =
            build_suggestions("2026-10-01", &events, &HashSet::new(), &HashSet::new());
        let bond = suggestions
            .iter()
            .find(|suggestion| suggestion.auto_key == "bond-end:13")
            .expect("bond-end suggestion");
        assert!(approx_eq(bond.amount, 49500.0));
    }

    /// Stored keys drop out (the accepted candidate is a real item); dismissed
    /// keys stay listed, flagged — the ignored row the editor renders.
    #[test]
    fn suggestions_exclude_stored_keys_and_flag_dismissed_ones() {
        let events = suggestion_events();
        let stored = HashSet::from([
            "dep-end:2".to_string(),
            "pool-input".to_string(),
            "ibkr:11".to_string(),
        ]);
        let dismissed = HashSet::from(["trade:3".to_string()]);
        let suggestions = build_suggestions("2026-10-01", &events, &stored, &dismissed);
        let by_key: HashMap<&str, &MonthSuggestion> = suggestions
            .iter()
            .map(|suggestion| (suggestion.auto_key.as_str(), suggestion))
            .collect();
        assert!(!by_key.contains_key("dep-end:2"));
        assert!(!by_key.contains_key("pool-input"));
        assert!(!by_key.contains_key("ibkr:11"));
        assert!(by_key["trade:3"].dismissed);
        assert!(!by_key["dep-start:1"].dismissed);
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
    fn suggestion_actions_open_at_the_previous_month() {
        let today = ym("2026-10-23");
        assert!(suggestion_actions_enabled("2026-10-01", today));
        assert!(suggestion_actions_enabled("2026-09-01", today));
        assert!(suggestion_actions_enabled("2026-11-01", today));
        assert!(!suggestion_actions_enabled("2026-08-01", today));
        assert!(!suggestion_actions_enabled("not-a-month", today));
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

    // --- 策略 liquidity tiers (Overview J3:K7) ---
}
