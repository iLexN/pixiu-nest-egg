use super::*;

use std::collections::HashSet;

use crate::models::{InterestComponent, MonthItemCategory, MonthSuggestion, TradeType};

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

    // Coupons and HK dividends suggest before 收訖 — like dep-end — at the
    // received amount, else the expected/estimated figure. A receipt with no
    // known amount (待定 coupon, estimate-less dividend) has nothing to
    // suggest.
    for dividend in &events.dividends {
        if in_month(dividend.pay_date, month)
            && let Some(amount) = dividend.amount
        {
            suggestions.push(MonthSuggestion {
                auto_key: format!("div:{}", dividend.id),
                category: MonthItemCategory::Adjustment,
                label: Some(dividend.label.clone()),
                amount,
                source: "dividend".to_string(),
            });
        }
    }

    for coupon in &events.coupons {
        if in_month(coupon.pay_date, month)
            && let Some(amount) = coupon.amount
        {
            suggestions.push(MonthSuggestion {
                auto_key: format!("coupon:{}", coupon.id),
                category: MonthItemCategory::Adjustment,
                label: Some(coupon.label.clone()),
                amount,
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
            amount: events.pool_input,
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
    use std::collections::HashMap;

    use super::*;
    use crate::calc::test_helpers::*;

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
        // Pending receipts suggest at their expected/estimated figure; a
        // 待定 coupon with no amount suggests nothing.
        assert!(approx_eq(by_key["div:9"].amount, 720.0));
        assert!(!by_key.contains_key("coupon:10"));
        assert_eq!(by_key["aia-pay:7"].category, MonthItemCategory::ExtraSpend);
        assert!(approx_eq(by_key["aia-pay:7"].amount, 8320.0 * 7.8));
        assert!(approx_eq(by_key["pool-input"].amount, 4142.66));
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

    // --- 策略 liquidity tiers (Overview J3:K7) ---
}
