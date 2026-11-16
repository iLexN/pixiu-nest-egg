use super::*;

use crate::models::MonthItemCategory;

/// The stable `auto_key` of a Candidate — one variant per event source. Its
/// `Display` is the only place a stored key is rendered; the strings are
/// byte-identical to what the pre-catalogue code wrote.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateKey {
    DepStart(i64),
    DepEnd(i64),
    BondEnd(i64),
    Coupon(i64),
    Div(i64),
    Ibkr(i64),
    Trade(i64),
    AiaPay(i64),
    PoolInput,
}

impl std::fmt::Display for CandidateKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CandidateKey::DepStart(id) => write!(f, "dep-start:{id}"),
            CandidateKey::DepEnd(id) => write!(f, "dep-end:{id}"),
            CandidateKey::BondEnd(id) => write!(f, "bond-end:{id}"),
            CandidateKey::Coupon(id) => write!(f, "coupon:{id}"),
            CandidateKey::Div(id) => write!(f, "div:{id}"),
            CandidateKey::Ibkr(id) => write!(f, "ibkr:{id}"),
            CandidateKey::Trade(id) => write!(f, "trade:{id}"),
            CandidateKey::AiaPay(id) => write!(f, "aia-pay:{id}"),
            CandidateKey::PoolInput => write!(f, "pool-input"),
        }
    }
}

impl CandidateKey {
    /// The `MonthSuggestion.source` bucket the candidate reports.
    pub fn source(&self) -> &'static str {
        match self {
            CandidateKey::DepStart(_) | CandidateKey::DepEnd(_) => "deposit",
            CandidateKey::BondEnd(_) => "bond",
            CandidateKey::Coupon(_) => "coupon",
            CandidateKey::Div(_) => "dividend",
            CandidateKey::Ibkr(_) => "ibkr",
            CandidateKey::Trade(_) => "trade",
            CandidateKey::AiaPay(_) => "aia",
            CandidateKey::PoolInput => "pool",
        }
    }
}

/// The single definition of an event-derived month item: the suggestion the
/// month read shows and the item the write path records both come from this
/// one value. `month` is the first of the month.
#[derive(Debug, Clone)]
pub struct Candidate {
    pub key: CandidateKey,
    pub month: chrono::NaiveDate,
    pub category: MonthItemCategory,
    pub label: String,
    pub amount: f64,
}

/// A bond as a suggestion source: the `bond-end:<id>` candidate in its
/// maturity month. `credited_amount` is set once 收訖 banked a return —
/// before that the candidate carries the principal.
#[derive(Debug, Clone)]
pub struct SuggestionBond {
    pub id: i64,
    pub maturity_date: chrono::NaiveDate,
    pub principal: f64,
    pub credited_amount: Option<f64>,
    pub label: String,
}

fn deposit_label(deposit: &SuggestionDeposit) -> String {
    deposit.label.clone().unwrap_or_else(|| "定期".to_string())
}

/// 定期 placement: −principal in the start month; `None` while the deposit
/// carries no `start_date`.
pub fn dep_start(deposit: &SuggestionDeposit) -> Option<Candidate> {
    Some(Candidate {
        key: CandidateKey::DepStart(deposit.id),
        month: first_of_month(deposit.start_date?),
        category: MonthItemCategory::Adjustment,
        label: deposit_label(deposit),
        amount: -deposit.principal,
    })
}

/// 定期 ending: +principal+interest in the end month.
pub fn dep_end(deposit: &SuggestionDeposit) -> Option<Candidate> {
    Some(Candidate {
        key: CandidateKey::DepEnd(deposit.id),
        month: first_of_month(deposit.end_date),
        category: MonthItemCategory::Adjustment,
        label: deposit_label(deposit),
        amount: deposit.principal + deposit.interest,
    })
}

/// Bond maturity: +principal in the maturity month, or + the credited amount
/// once 收訖 banked one.
pub fn bond_end(bond: &SuggestionBond) -> Option<Candidate> {
    Some(Candidate {
        key: CandidateKey::BondEnd(bond.id),
        month: first_of_month(bond.maturity_date),
        category: MonthItemCategory::Adjustment,
        label: bond.label.clone(),
        amount: bond.credited_amount.unwrap_or(bond.principal),
    })
}

fn receipt_candidate(key: CandidateKey, receipt: &SuggestionReceipt) -> Option<Candidate> {
    Some(Candidate {
        key,
        month: first_of_month(receipt.pay_date),
        category: MonthItemCategory::Adjustment,
        label: receipt.label.clone(),
        amount: receipt.amount?,
    })
}

/// A bond coupon at its known amount (received, else expected); `None` while
/// 待定.
pub fn coupon(receipt: &SuggestionReceipt) -> Option<Candidate> {
    receipt_candidate(CandidateKey::Coupon(receipt.id), receipt)
}

/// An HK dividend at its known amount (received, else estimated); `None`
/// while nothing is known.
pub fn dividend(receipt: &SuggestionReceipt) -> Option<Candidate> {
    receipt_candidate(CandidateKey::Div(receipt.id), receipt)
}

/// A cash-moving IBKR 轉入: −amount in the transfer month.
pub fn ibkr(transfer: &SuggestionIbkrTransfer) -> Option<Candidate> {
    Some(Candidate {
        key: CandidateKey::Ibkr(transfer.id),
        month: first_of_month(transfer.date),
        category: MonthItemCategory::Adjustment,
        label: "IBKR 轉入".to_string(),
        amount: -transfer.amount,
    })
}

/// An HK trade: −BUY / +SELL total, labelled `{code} {BUY|SELL}`.
pub fn trade(trade: &SuggestionTrade) -> Option<Candidate> {
    Some(Candidate {
        key: CandidateKey::Trade(trade.id),
        month: first_of_month(trade.date),
        category: MonthItemCategory::Adjustment,
        label: format!("{} {}", trade.code, trade.kind.as_str()),
        amount: trade_amount(trade),
    })
}

/// An AIA premium: amount_usd × the USD→HKD rate as 額外支出; `None` while no
/// rate is stored.
pub fn aia_pay(payment: &SuggestionAiaPayment, usd_hkd_rate: Option<f64>) -> Option<Candidate> {
    Some(Candidate {
        key: CandidateKey::AiaPay(payment.id),
        month: first_of_month(payment.date),
        category: MonthItemCategory::ExtraSpend,
        label: payment.policy.clone(),
        amount: payment.amount_usd * usd_hkd_rate?,
    })
}

/// The month's own 開心Pool input; `None` while it is 0.
pub fn pool_input(month: chrono::NaiveDate, pool_input: f64) -> Option<Candidate> {
    (pool_input > 0.0).then(|| Candidate {
        key: CandidateKey::PoolInput,
        month,
        category: MonthItemCategory::Adjustment,
        label: "Irene + 開心 Pool".to_string(),
        amount: pool_input,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calc::test_helpers::*;
    use crate::models::TradeType;

    /// The stored key strings, pinned against what the live DB already holds
    /// — the Display impl is the only place a key renders.
    #[test]
    fn candidate_keys_render_the_stored_strings() {
        assert_eq!(CandidateKey::DepStart(7).to_string(), "dep-start:7");
        assert_eq!(CandidateKey::DepEnd(7).to_string(), "dep-end:7");
        assert_eq!(CandidateKey::BondEnd(7).to_string(), "bond-end:7");
        assert_eq!(CandidateKey::Coupon(7).to_string(), "coupon:7");
        assert_eq!(CandidateKey::Div(7).to_string(), "div:7");
        assert_eq!(CandidateKey::Ibkr(7).to_string(), "ibkr:7");
        assert_eq!(CandidateKey::Trade(7).to_string(), "trade:7");
        assert_eq!(CandidateKey::AiaPay(7).to_string(), "aia-pay:7");
        assert_eq!(CandidateKey::PoolInput.to_string(), "pool-input");
    }

    #[test]
    fn candidate_keys_report_their_source() {
        assert_eq!(CandidateKey::DepStart(7).source(), "deposit");
        assert_eq!(CandidateKey::DepEnd(7).source(), "deposit");
        assert_eq!(CandidateKey::BondEnd(7).source(), "bond");
        assert_eq!(CandidateKey::Coupon(7).source(), "coupon");
        assert_eq!(CandidateKey::Div(7).source(), "dividend");
        assert_eq!(CandidateKey::Ibkr(7).source(), "ibkr");
        assert_eq!(CandidateKey::Trade(7).source(), "trade");
        assert_eq!(CandidateKey::AiaPay(7).source(), "aia");
        assert_eq!(CandidateKey::PoolInput.source(), "pool");
    }

    fn deposit(label: Option<&str>) -> SuggestionDeposit {
        SuggestionDeposit {
            id: 1,
            start_date: Some(day("2026-10-13")),
            end_date: day("2027-04-13"),
            principal: 100000.0,
            interest: 1200.0,
            label: label.map(str::to_string),
            received: false,
            bank: None,
        }
    }

    #[test]
    fn dep_start_is_minus_principal_in_the_start_month() {
        let candidate = dep_start(&deposit(Some("SC-9632"))).expect("candidate");
        assert_eq!(candidate.key, CandidateKey::DepStart(1));
        assert_eq!(candidate.month, day("2026-10-01"));
        assert_eq!(candidate.category, MonthItemCategory::Adjustment);
        assert_eq!(candidate.label, "SC-9632");
        assert_eq!(candidate.amount, -100000.0);
    }

    /// No start_date means no candidate; an unlabeled deposit falls back to 定期.
    #[test]
    fn dep_start_needs_a_start_date_and_falls_back_to_the_default_label() {
        let mut without_start = deposit(None);
        without_start.start_date = None;
        assert!(dep_start(&without_start).is_none());

        let unlabeled = dep_start(&deposit(None)).expect("candidate");
        assert_eq!(unlabeled.label, "定期");

        let unlabeled_end = dep_end(&deposit(None)).expect("candidate");
        assert_eq!(unlabeled_end.label, "定期");
    }

    #[test]
    fn dep_end_is_principal_plus_interest_in_the_end_month() {
        let candidate = dep_end(&deposit(Some("SC-9632"))).expect("candidate");
        assert_eq!(candidate.key, CandidateKey::DepEnd(1));
        assert_eq!(candidate.month, day("2027-04-01"));
        assert_eq!(candidate.amount, 101200.0);
    }

    #[test]
    fn bond_end_is_the_credited_amount_else_the_principal() {
        let pending = bond_end(&SuggestionBond {
            id: 3,
            maturity_date: day("2026-10-20"),
            principal: 100000.0,
            credited_amount: None,
            label: "silver bond".to_string(),
        })
        .expect("candidate");
        assert_eq!(pending.key, CandidateKey::BondEnd(3));
        assert_eq!(pending.month, day("2026-10-01"));
        assert_eq!(pending.category, MonthItemCategory::Adjustment);
        assert_eq!(pending.label, "silver bond");
        assert_eq!(pending.amount, 100000.0);

        let credited = bond_end(&SuggestionBond {
            id: 3,
            maturity_date: day("2026-10-20"),
            principal: 100000.0,
            credited_amount: Some(99500.0),
            label: "silver bond".to_string(),
        })
        .expect("candidate");
        assert_eq!(credited.amount, 99500.0);
    }

    fn receipt(amount: Option<f64>) -> SuggestionReceipt {
        SuggestionReceipt {
            id: 5,
            pay_date: day("2026-10-15"),
            amount,
            label: "silver bond".to_string(),
            received: amount.is_some(),
        }
    }

    #[test]
    fn coupon_and_dividend_need_a_known_amount() {
        let candidate = coupon(&receipt(Some(2000.0))).expect("candidate");
        assert_eq!(candidate.key, CandidateKey::Coupon(5));
        assert_eq!(candidate.month, day("2026-10-01"));
        assert_eq!(candidate.category, MonthItemCategory::Adjustment);
        assert_eq!(candidate.label, "silver bond");
        assert_eq!(candidate.amount, 2000.0);
        assert!(coupon(&receipt(None)).is_none());

        let candidate = dividend(&receipt(Some(1500.0))).expect("candidate");
        assert_eq!(candidate.key, CandidateKey::Div(5));
        assert_eq!(candidate.amount, 1500.0);
        assert!(dividend(&receipt(None)).is_none());
    }

    #[test]
    fn ibkr_is_minus_amount_labelled_ibkr_transfer() {
        let candidate = ibkr(&SuggestionIbkrTransfer {
            id: 11,
            date: day("2026-10-17"),
            amount: 50000.0,
        })
        .expect("candidate");
        assert_eq!(candidate.key, CandidateKey::Ibkr(11));
        assert_eq!(candidate.month, day("2026-10-01"));
        assert_eq!(candidate.category, MonthItemCategory::Adjustment);
        assert_eq!(candidate.label, "IBKR 轉入");
        assert_eq!(candidate.amount, -50000.0);
    }

    #[test]
    fn trade_signs_by_kind_and_labels_code_kind() {
        let buy = trade(&SuggestionTrade {
            id: 3,
            date: day("2026-10-08"),
            kind: TradeType::Buy,
            total: 96523.15,
            code: "中國銀行".to_string(),
        })
        .expect("candidate");
        assert_eq!(buy.key, CandidateKey::Trade(3));
        assert_eq!(buy.month, day("2026-10-01"));
        assert_eq!(buy.label, "中國銀行 BUY");
        assert_eq!(buy.amount, -96523.15);

        let sell = trade(&SuggestionTrade {
            id: 4,
            date: day("2026-10-09"),
            kind: TradeType::Sell,
            total: 20000.0,
            code: "恒生".to_string(),
        })
        .expect("candidate");
        assert_eq!(sell.label, "恒生 SELL");
        assert_eq!(sell.amount, 20000.0);
    }

    #[test]
    fn aia_pay_needs_a_rate_and_is_extra_spend() {
        let payment = SuggestionAiaPayment {
            id: 7,
            date: day("2026-10-01"),
            amount_usd: 8320.0,
            policy: "年金".to_string(),
        };
        assert!(aia_pay(&payment, None).is_none());
        let candidate = aia_pay(&payment, Some(7.8)).expect("candidate");
        assert_eq!(candidate.key, CandidateKey::AiaPay(7));
        assert_eq!(candidate.category, MonthItemCategory::ExtraSpend);
        assert_eq!(candidate.label, "年金");
        assert!(approx_eq(candidate.amount, 64896.0));
    }

    #[test]
    fn pool_input_needs_a_positive_amount() {
        assert!(pool_input(day("2026-10-01"), 0.0).is_none());
        let candidate = pool_input(day("2026-10-01"), 4142.66).expect("candidate");
        assert_eq!(candidate.key, CandidateKey::PoolInput);
        assert_eq!(candidate.month, day("2026-10-01"));
        assert_eq!(candidate.category, MonthItemCategory::Adjustment);
        assert_eq!(candidate.label, "Irene + 開心 Pool");
        assert_eq!(candidate.amount, 4142.66);
    }
}
