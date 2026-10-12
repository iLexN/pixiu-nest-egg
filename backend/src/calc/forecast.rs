use super::*;

use chrono::Datelike;

use crate::models::ForecastItemKind;

/// The forecast window: the current month plus six ahead (sheet columns B:H).
pub const FORECAST_MONTHS: u32 = 7;

/// The sheet's 差餉 quarter months — the 繳費 row's auto line.
pub const BILL_QUARTER_MONTHS: [u32; 4] = [1, 4, 7, 10];

/// A stored `forecast_items` row reduced to what the derivation needs.
#[derive(Debug, Clone, Copy)]
pub struct ForecastItemFacts {
    /// The month the cash line lands in (`YYYY-MM-01`).
    pub month: chrono::NaiveDate,
    pub kind: ForecastItemKind,
    /// Signed amount (planned lockups are negative).
    pub amount: f64,
    /// Return-month override (`YYYY-MM-01`); NULL uses the kind's lag.
    pub return_month: Option<chrono::NaiveDate>,
}

/// The inputs `forecast_months` needs, resolved by the caller.
pub struct ForecastInput<'a> {
    /// The window's first column — the current month.
    pub first_month: chrono::NaiveDate,
    /// The first column's start: the current month's `start_cash`, falling
    /// back to the live 活期 sum while it is unset.
    pub start: Option<f64>,
    /// `overview.salary`.
    pub salary: Option<f64>,
    /// −生活預算; absent while the trailing window is empty.
    pub spend: Option<f64>,
    /// Live 流動資產 — the ref-check baseline (`Overview!N7`).
    pub liquid_assets: f64,
    /// The configured 半流動資金 target share of 流動資產
    /// (`overview.semi_liquid_target`, 0.25 while unset) — the ref-check ratio;
    /// the sheet's `N7` hardcodes ÷ 4.
    pub semi_liquid_target: f64,
    /// The locked anchor: Σ principal of deposits held on the window's first
    /// day (`forecast_locked_anchor`), not the live "end date not passed" sum.
    pub locked_base: f64,
    /// The effective quarterly 差餉 amount.
    pub bill_amount: f64,
    /// Every deposit (the finish row buckets by `end_date`, like the sheet's
    /// `SUMIF`, regardless of received status).
    pub deposits: &'a [SuggestionDeposit],
    /// The dated receipts: `dividends` (HK) and `coupons` feed 利息; pending
    /// ones count at their expected/estimated figure, None amounts skip.
    pub events: &'a SuggestionEvents,
    /// All forecast items; only items whose month is inside the window have
    /// effects (a stale pre-window plan fires nothing — its cell left the
    /// sheet when the window slid past, and its return must not double-count
    /// a deposit that already exists).
    pub items: &'a [ForecastItemFacts],
}

/// One derived month column of the 預測 grid.
#[derive(Debug, Clone, PartialEq)]
pub struct ForecastMonthDerived {
    pub month: chrono::NaiveDate,
    pub start: Option<f64>,
    /// Σ principal of deposits ending in the month.
    pub deposit_finish: f64,
    /// Auto receipts (deposit interest + dividends + coupons) + `interest` items.
    pub interest: f64,
    /// `bill` items summed when any exist, else −`bill_amount` on the quarter
    /// months; `None` on other months.
    pub bill: Option<f64>,
    /// Σ `tax` + `stock` + `other` items.
    pub plans: f64,
    /// Σ `hs_deposit` + `sc_deposit` items (negative = cash out into lockup).
    pub deposit_plans: f64,
    /// Σ principal of real deposits placed in the month (`start_date` in it).
    pub deposit_start: f64,
    /// Σ principal returning from in-window plans this month.
    pub deposit_return: f64,
    /// start + salary + spend + finish + interest + bill + plans + deposit
    /// plans − deposit placements + deposit return; absent while `start` is.
    pub cash: Option<f64>,
    /// Prior `locked` − finish − deposit plans + placements − returns,
    /// anchored on `locked_base`.
    pub locked: Option<f64>,
    /// `cash` + `locked`.
    pub semi_liquid: Option<f64>,
    /// `semi_liquid` − `semi_liquid_target` × `liquid_assets`.
    pub ref_check: Option<f64>,
}

/// The month a deposit-plan item's principal returns: its `return_month`
/// override, else the kind's default lag from the plan month.
fn item_return_month(item: &ForecastItemFacts) -> Option<chrono::NaiveDate> {
    item.return_month.or_else(|| {
        item.kind
            .deposit_lag()
            .and_then(|lag| item.month.checked_add_months(chrono::Months::new(lag)))
    })
}

/// The locked anchor: Σ principal of deposits held on the window's first day —
/// `start_date` absent or before the first month's 1st, `end_date` on or after
/// it. `end_date >= first_month` keeps a deposit that matured earlier in the
/// month in the anchor, so `deposit_finish` removes it exactly once.
pub fn forecast_locked_anchor(
    deposits: &[SuggestionDeposit],
    first_month: chrono::NaiveDate,
) -> f64 {
    deposits
        .iter()
        .filter(|deposit| {
            deposit.start_date.is_none_or(|d| d < first_month) && deposit.end_date >= first_month
        })
        .map(|deposit| deposit.principal)
        .sum::<f64>()
        + 0.0
}

/// The seven-month rolling 預測 grid. Every figure derives from the inputs;
/// nothing is stored except the items themselves.
pub fn forecast_months(input: &ForecastInput) -> Vec<ForecastMonthDerived> {
    let window: Vec<chrono::NaiveDate> = (0..FORECAST_MONTHS)
        .map(|offset| {
            input
                .first_month
                .checked_add_months(chrono::Months::new(offset))
                .expect("forecast month overflow")
        })
        .collect();
    let in_window = |month: chrono::NaiveDate| {
        month >= window[0] && month <= window[FORECAST_MONTHS as usize - 1]
    };

    let mut derived = Vec::with_capacity(FORECAST_MONTHS as usize);
    let mut cash = input.start;
    let mut locked = Some(input.locked_base);
    for month in &window {
        let items: Vec<&ForecastItemFacts> = input
            .items
            .iter()
            .filter(|item| item.month == *month)
            .collect();
        let mut interest_items = 0.0;
        let mut bill_items = 0.0;
        let mut plans = 0.0;
        let mut deposit_plans = 0.0;
        for item in &items {
            match item.kind {
                ForecastItemKind::Interest => interest_items += item.amount,
                ForecastItemKind::Bill => bill_items += item.amount,
                ForecastItemKind::HsDeposit | ForecastItemKind::ScDeposit => {
                    deposit_plans += item.amount
                }
                ForecastItemKind::Tax | ForecastItemKind::Stock | ForecastItemKind::Other => {
                    plans += item.amount
                }
            }
        }
        // Deposits placed inside the window return at their effective month.
        // (sum() of an empty iter is -0.0 — the +0.0 collapses it so the UI
        // never shows "-0.00".)
        let deposit_return: f64 = input
            .items
            .iter()
            .filter(|item| {
                item.kind.deposit_lag().is_some()
                    && in_window(item.month)
                    && item_return_month(item) == Some(*month)
            })
            .map(|item| -item.amount)
            .sum::<f64>()
            + 0.0;

        // Sheet row 28: every deposit ending in the month, received or not.
        let deposit_finish: f64 = input
            .deposits
            .iter()
            .filter(|deposit| in_month(deposit.end_date, *month))
            .map(|deposit| deposit.principal)
            .sum::<f64>()
            + 0.0;
        // Sheet row 31: deposit interest ending + dividends + coupons at their
        // known (received or expected) figures, plus `interest` items.
        let receipts: f64 = input
            .deposits
            .iter()
            .filter(|deposit| in_month(deposit.end_date, *month))
            .map(|deposit| deposit.interest)
            .sum::<f64>()
            + input
                .events
                .dividends
                .iter()
                .chain(input.events.coupons.iter())
                .filter(|receipt| in_month(receipt.pay_date, *month))
                .filter_map(|receipt| receipt.amount)
                .sum::<f64>();
        let interest = receipts + interest_items;

        let bill = if items.iter().any(|item| item.kind == ForecastItemKind::Bill) {
            Some(bill_items)
        } else if BILL_QUARTER_MONTHS.contains(&month.month()) {
            Some(-input.bill_amount)
        } else {
            None
        };

        // A real deposit placed in the month moves principal out of 活期 into
        // lockup — the same flow a hs_deposit/sc_deposit plan would model.
        let deposit_start: f64 = input
            .deposits
            .iter()
            .filter(|deposit| deposit.start_date.is_some_and(|d| in_month(d, *month)))
            .map(|deposit| deposit.principal)
            .sum::<f64>()
            + 0.0;

        // This column's start is the chained cash before its own flows.
        let start = cash;
        cash = start.map(|start| {
            start
                + input.salary.unwrap_or(0.0)
                + input.spend.unwrap_or(0.0)
                + deposit_finish
                + interest
                + bill.unwrap_or(0.0)
                + plans
                + deposit_plans
                - deposit_start
                + deposit_return
        });
        locked = locked
            .map(|locked| locked - deposit_finish - deposit_plans + deposit_start - deposit_return);
        let semi_liquid = cash.and_then(|cash| locked.map(|locked| cash + locked));
        derived.push(ForecastMonthDerived {
            month: *month,
            start,
            deposit_finish,
            interest,
            bill,
            plans,
            deposit_plans,
            deposit_start,
            deposit_return,
            cash,
            locked,
            semi_liquid,
            ref_check: semi_liquid
                .map(|semi| semi - input.semi_liquid_target * input.liquid_assets),
        });
    }
    derived
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calc::test_helpers::*;

    fn forecast_input<'a>(
        deposits: &'a [SuggestionDeposit],
        events: &'a SuggestionEvents,
        items: &'a [ForecastItemFacts],
    ) -> ForecastInput<'a> {
        ForecastInput {
            first_month: ym("2026-09-01"),
            start: Some(23_746.72),
            salary: Some(52_700.0),
            spend: Some(-8_500.0),
            liquid_assets: 1_200_000.0,
            semi_liquid_target: 0.25,
            locked_base: 445_000.0,
            bill_amount: 2_158.0,
            deposits,
            events,
            items,
        }
    }

    fn plan(
        month: &str,
        kind: ForecastItemKind,
        amount: f64,
        return_month: Option<&str>,
    ) -> ForecastItemFacts {
        ForecastItemFacts {
            month: ym(month),
            kind,
            amount,
            return_month: return_month.map(ym),
        }
    }

    fn dep_event(month_end: &str, principal: f64, interest: f64) -> SuggestionDeposit {
        SuggestionDeposit {
            id: 1,
            start_date: None,
            end_date: day(month_end),
            principal,
            interest,
            label: None,
            received: false,
            bank: None,
        }
    }

    #[test]
    fn forecast_anchors_on_start_cash_and_chains() {
        let events = SuggestionEvents::default();
        let months = forecast_months(&forecast_input(&[], &events, &[]));
        assert_eq!(months.len(), FORECAST_MONTHS as usize);
        assert_eq!(months[0].month, ym("2026-09-01"));
        assert_eq!(months[6].month, ym("2027-03-01"));

        assert_eq!(months[0].start, Some(23_746.72));
        // Sep cash = 23746.72 + 52700 − 8500 (+ Oct is a bill quarter? no — Oct
        // bills, not Sep; Sep has no bill) — wait: Oct IS a bill month.
        assert!(approx_eq(months[0].cash.unwrap(), 67_946.72));
        assert_eq!(months[1].start, months[0].cash);
        // Oct: start − 8500 + 52700 − 2158 (quarter bill).
        assert!(approx_eq(
            months[1].cash.unwrap(),
            67_946.72 + 52_700.0 - 8_500.0 - 2_158.0
        ));
    }

    #[test]
    fn forecast_start_falls_back_to_live_cash() {
        let events = SuggestionEvents::default();
        let mut input = forecast_input(&[], &events, &[]);
        input.start = Some(32_934.62); // live cash_sum when start_cash unset
        let months = forecast_months(&input);
        assert_eq!(months[0].start, Some(32_934.62));
    }

    #[test]
    fn forecast_deposit_finish_and_interest_count_the_end_month() {
        let deposits = vec![dep_event("2026-10-20", 110_000.0, 3_025.0)];
        let events = SuggestionEvents::default();
        let months = forecast_months(&forecast_input(&deposits, &events, &[]));
        let oct = &months[1];
        assert!(approx_eq(oct.deposit_finish, 110_000.0));
        assert!(approx_eq(oct.interest, 3_025.0));
        // Cash gains the return; locked drops by the same principal.
        assert!(approx_eq(
            oct.cash.unwrap() - oct.start.unwrap(),
            52_700.0 - 8_500.0 - 2_158.0 + 110_000.0 + 3_025.0
        ));
        assert!(approx_eq(oct.locked.unwrap(), 445_000.0 - 110_000.0));
    }

    #[test]
    fn forecast_interest_counts_pending_dividend_and_coupon() {
        let events = SuggestionEvents {
            dividends: vec![SuggestionReceipt {
                id: 1,
                pay_date: day("2026-11-10"),
                amount: Some(500.0),
                label: "中銀".to_string(),
                received: false,
            }],
            coupons: vec![SuggestionReceipt {
                id: 2,
                pay_date: day("2026-11-20"),
                amount: Some(1_234.5),
                label: "iBond".to_string(),
                received: false,
            }],
            ..Default::default()
        };
        let items = vec![plan("2026-11-01", ForecastItemKind::Interest, 88.0, None)];
        let months = forecast_months(&forecast_input(&[], &events, &items));
        assert!(approx_eq(months[2].interest, 500.0 + 1_234.5 + 88.0));
        // Unknown amounts (待定) are skipped.
        let events = SuggestionEvents {
            coupons: vec![SuggestionReceipt {
                id: 3,
                pay_date: day("2026-11-20"),
                amount: None,
                label: "iBond".to_string(),
                received: false,
            }],
            ..Default::default()
        };
        let months = forecast_months(&forecast_input(&[], &events, &[]));
        assert!(approx_eq(months[2].interest, 0.0));
    }

    #[test]
    fn forecast_bill_items_override_the_quarterly_default() {
        let events = SuggestionEvents::default();
        // Oct is a quarter month: default −2158.
        let months = forecast_months(&forecast_input(&[], &events, &[]));
        assert_eq!(months[1].bill, Some(-2_158.0));
        assert_eq!(months[0].bill, None);
        // A bill item replaces (not stacks on) the default.
        let items = vec![plan("2026-10-01", ForecastItemKind::Bill, -2_260.5, None)];
        let months = forecast_months(&forecast_input(&[], &events, &items));
        assert_eq!(months[1].bill, Some(-2_260.5));
        // A bill item on a non-quarter month is a plain line.
        let items = vec![plan("2026-11-01", ForecastItemKind::Bill, -500.0, None)];
        let months = forecast_months(&forecast_input(&[], &events, &items));
        assert_eq!(months[2].bill, Some(-500.0));
    }

    #[test]
    fn forecast_return_lags_and_locked_chain() {
        let events = SuggestionEvents::default();
        let items = vec![
            plan("2026-10-01", ForecastItemKind::ScDeposit, -180_000.0, None),
            plan("2026-09-01", ForecastItemKind::HsDeposit, -50_000.0, None),
        ];
        let months = forecast_months(&forecast_input(&[], &events, &items));

        // Sep: −50000 out, locked +50000.
        assert!(approx_eq(months[0].deposit_plans, -50_000.0));
        assert!(approx_eq(months[0].locked.unwrap(), 445_000.0 + 50_000.0));
        // Oct: −180000 out.
        assert!(approx_eq(months[1].deposit_plans, -180_000.0));
        // Dec: the HS plan returns (+3) — +50000 cash, locked −50000.
        assert!(approx_eq(months[3].deposit_return, 50_000.0));
        // Feb 2027: the SC plan returns (+4 months from Oct).
        assert!(approx_eq(months[5].deposit_return, 180_000.0));
        assert!(approx_eq(
            months[5].locked.unwrap(),
            445_000.0 + 50_000.0 + 180_000.0 - 50_000.0 - 180_000.0
        ));
    }

    #[test]
    fn forecast_return_month_override_wins() {
        let events = SuggestionEvents::default();
        let items = vec![plan(
            "2026-09-01",
            ForecastItemKind::HsDeposit,
            -50_000.0,
            Some("2027-02-01"),
        )];
        let months = forecast_months(&forecast_input(&[], &events, &items));
        assert_eq!(months[3].deposit_return, 0.0); // Dec: no default return
        assert!(approx_eq(months[5].deposit_return, 50_000.0)); // Feb 2027
    }

    #[test]
    fn forecast_pre_window_plan_fires_nothing() {
        let events = SuggestionEvents::default();
        // A plan left behind in Aug (before the Sep window) is stale: no
        // placement, no return — matching the shifted-out sheet cell.
        let items = vec![plan(
            "2026-08-01",
            ForecastItemKind::ScDeposit,
            -60_000.0,
            None,
        )];
        let months = forecast_months(&forecast_input(&[], &events, &items));
        assert!(months.iter().all(|m| m.deposit_plans == 0.0));
        assert!(months.iter().all(|m| m.deposit_return == 0.0));
    }

    #[test]
    fn forecast_empty_cells_stay_positive_zero() {
        let events = SuggestionEvents::default();
        let months = forecast_months(&forecast_input(&[], &events, &[]));
        for m in &months {
            assert!(
                m.deposit_finish.is_sign_positive() && m.deposit_return.is_sign_positive(),
                "{} finish={} return={}",
                m.month,
                m.deposit_finish,
                m.deposit_return
            );
        }
    }

    #[test]
    fn forecast_ref_check_diffs_against_quarter_liquid() {
        let events = SuggestionEvents::default();
        let months = forecast_months(&forecast_input(&[], &events, &[]));
        let semi = months[0].cash.unwrap() + months[0].locked.unwrap();
        assert_eq!(months[0].semi_liquid, Some(semi));
        assert!(approx_eq(
            months[0].ref_check.unwrap(),
            semi - 1_200_000.0 / 4.0
        ));
    }

    #[test]
    fn forecast_ref_check_uses_the_configured_ratio() {
        let events = SuggestionEvents::default();
        let mut input = forecast_input(&[], &events, &[]);
        input.semi_liquid_target = 0.3;
        let months = forecast_months(&input);
        let semi = months[0].semi_liquid.unwrap();
        assert!(approx_eq(
            months[0].ref_check.unwrap(),
            semi - 0.3 * 1_200_000.0
        ));
    }

    #[test]
    fn forecast_locked_anchor_counts_month_start_holdings() {
        let first = ym("2026-09-01");
        // NULL start and starts before the 1st count; an end inside the
        // month stays in (deposit_finish removes it exactly once).
        let held = vec![
            SuggestionDeposit {
                start_date: None,
                ..dep_event("2026-11-20", 80_000.0, 0.0)
            },
            SuggestionDeposit {
                start_date: Some(day("2026-08-31")),
                ..dep_event("2027-01-05", 80_000.0, 0.0)
            },
            SuggestionDeposit {
                start_date: Some(day("2026-06-01")),
                ..dep_event("2026-09-05", 10_000.0, 0.0)
            },
        ];
        assert!(approx_eq(forecast_locked_anchor(&held, first), 170_000.0));
        // A start on/after the 1st and an end before it stay out.
        let out = vec![
            SuggestionDeposit {
                start_date: Some(day("2026-09-01")),
                ..dep_event("2027-01-05", 5_000.0, 0.0)
            },
            SuggestionDeposit {
                start_date: Some(day("2026-09-30")),
                ..dep_event("2027-01-05", 55_000.0, 0.0)
            },
            SuggestionDeposit {
                start_date: Some(day("2026-08-01")),
                ..dep_event("2026-08-31", 7_000.0, 0.0)
            },
        ];
        assert_eq!(forecast_locked_anchor(&out, first), 0.0);
    }

    #[test]
    fn forecast_locked_anchor_matches_the_spec_scenario() {
        let first = ym("2026-09-01");
        let deposits = vec![
            SuggestionDeposit {
                start_date: Some(day("2026-07-01")),
                ..dep_event("2026-11-03", 140_000.0, 0.0)
            },
            SuggestionDeposit {
                start_date: None,
                ..dep_event("2026-11-20", 80_000.0, 0.0)
            },
            SuggestionDeposit {
                start_date: Some(day("2026-08-31")),
                ..dep_event("2027-01-05", 80_000.0, 0.0)
            },
            SuggestionDeposit {
                start_date: Some(day("2026-09-30")),
                ..dep_event("2027-01-05", 55_000.0, 0.0)
            },
        ];
        assert!(approx_eq(
            forecast_locked_anchor(&deposits, first),
            300_000.0
        ));
        // The in-window start enters through its placement month instead.
        let events = SuggestionEvents::default();
        let mut input = forecast_input(&deposits, &events, &[]);
        input.locked_base = 300_000.0;
        let months = forecast_months(&input);
        assert!(approx_eq(months[0].locked.unwrap(), 355_000.0));
    }

    #[test]
    fn forecast_deposit_start_places_principal_in_the_month() {
        let deposits = vec![SuggestionDeposit {
            start_date: Some(day("2026-09-30")),
            ..dep_event("2027-01-05", 55_000.0, 0.0)
        }];
        let events = SuggestionEvents::default();
        let base = forecast_months(&forecast_input(&[], &events, &[]));
        let months = forecast_months(&forecast_input(&deposits, &events, &[]));
        // Sep 活期 drops by the principal, 定期+SC gains it.
        assert!(approx_eq(
            months[0].cash.unwrap(),
            base[0].cash.unwrap() - 55_000.0
        ));
        assert!(approx_eq(months[0].locked.unwrap(), 445_000.0 + 55_000.0));
        // Jan 2027: the principal comes back as 定期 finish.
        assert!(approx_eq(months[4].deposit_finish, 55_000.0));
    }

    #[test]
    fn forecast_conversion_leaves_the_grid_totals() {
        let events = SuggestionEvents::default();
        // A: the sc_deposit plan with its return override.
        let items = vec![plan(
            "2026-09-01",
            ForecastItemKind::ScDeposit,
            -55_000.0,
            Some("2027-01-01"),
        )];
        let planned = forecast_months(&forecast_input(&[], &events, &items));
        // B: the converted deposit — same flows, so every column matches.
        let deposits = vec![SuggestionDeposit {
            start_date: Some(day("2026-09-30")),
            ..dep_event("2027-01-05", 55_000.0, 0.0)
        }];
        let converted = forecast_months(&forecast_input(&deposits, &events, &[]));
        for (plan_col, dep_col) in planned.iter().zip(&converted) {
            assert!(approx_eq(plan_col.cash.unwrap(), dep_col.cash.unwrap()));
            assert!(approx_eq(plan_col.locked.unwrap(), dep_col.locked.unwrap()));
            assert!(approx_eq(
                plan_col.semi_liquid.unwrap(),
                dep_col.semi_liquid.unwrap()
            ));
            assert!(approx_eq(
                plan_col.ref_check.unwrap(),
                dep_col.ref_check.unwrap()
            ));
        }
    }

    // --- Money Master (Overview!J29:N35) ---
}
