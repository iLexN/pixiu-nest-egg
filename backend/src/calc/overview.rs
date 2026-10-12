use chrono::Datelike;
use serde::Serialize;
use utoipa::ToSchema;

/// The components of `Overview!B1`/`H1`, already resolved by the caller.
#[derive(Debug, Clone, Copy)]
pub struct LiveTotalsInput {
    /// 港股 market value.
    pub hk_market_value: f64,
    /// 美股 market value in USD.
    pub us_market_value: f64,
    /// USD→HKD rate (`aia.usd_hkd_rate`); rate-dependent terms drop out when absent.
    pub usd_hkd_rate: Option<f64>,
    /// 定期!B1: Σ principal over active deposits.
    pub deposits_active_principal: f64,
    /// 債券!B1: Σ principal over active bonds.
    pub bonds_active_principal: f64,
    /// AIA 總 value in USD (non-excluded rows).
    pub aia_value_usd: f64,
    /// MPF 總結存.
    pub mpf_balance: f64,
    /// Σ manual_assets where kind = 'asset'.
    pub manual_assets_sum: f64,
    /// Σ manual_assets where kind = 'cash'.
    pub cash_sum: f64,
    /// 美股!B4/B5: the IBKR account's cash positions, part of Overview!B9.
    pub ibkr_hkd_cash: f64,
    pub ibkr_usd_cash: f64,
    /// Current 開心Pool balance.
    pub pool_balance: f64,
}

/// `Overview!B1` (總數) and `H1` (流動資產) computed inside the backend.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, ToSchema)]
pub struct LiveTotals {
    pub total_assets: f64,
    pub liquid_assets: f64,
}

pub fn live_totals(input: &LiveTotalsInput) -> LiveTotals {
    // The US side is the sheet's 美股!B7 (Overview!B9): stocks plus the IBKR
    // account's own cash positions, with the HKD part needing no rate.
    let us_hkd = input
        .usd_hkd_rate
        .map(|rate| (input.us_market_value + input.ibkr_usd_cash) * rate);
    let aia_hkd = input.usd_hkd_rate.map(|rate| input.aia_value_usd * rate);
    LiveTotals {
        total_assets: input.hk_market_value
            + us_hkd.unwrap_or(0.0)
            + input.ibkr_hkd_cash
            + input.deposits_active_principal
            + input.bonds_active_principal
            + aia_hkd.unwrap_or(0.0)
            + input.mpf_balance
            + input.manual_assets_sum
            + input.cash_sum,
        liquid_assets: input.hk_market_value
            + input.deposits_active_principal
            + input.cash_sum
            + input.bonds_active_principal
            + us_hkd.unwrap_or(0.0)
            + input.ibkr_hkd_cash
            - input.pool_balance,
    }
}

/// The Overview `J3:K7` 策略 block: 總數 re-partitioned into liquidity tiers.
///
/// `cannot_use = salary × 6` (the sheet's `N6`); `can_use = semi_total −
/// cannot_use`, negative allowed — both absent without a stored salary.
/// `short_term` (K6 = 港股 + 債券 + IBKR + `manual_short`) hinges on `ibkr`
/// and `long_term` (K7 = 基金 + MPF + `manual_long`) on `aia`: both are
/// absent while the USD→HKD rate is unset rather than reporting partial
/// sums, matching the module's rule for rate-dependent figures. While every
/// figure is present the four tiers sum to `total_assets`.
#[allow(clippy::too_many_arguments)]
pub fn liquidity_tiers(
    salary: Option<f64>,
    semi_total: f64,
    hk: f64,
    bonds: f64,
    ibkr: Option<f64>,
    aia: Option<f64>,
    mpf: f64,
    manual_short: f64,
    manual_long: f64,
) -> crate::models::LiquidityTiers {
    let cannot_use = salary.map(|salary| salary * 6.0);
    crate::models::LiquidityTiers {
        can_use: cannot_use.map(|cannot_use| semi_total - cannot_use),
        cannot_use,
        short_term: ibkr.map(|ibkr| hk + bonds + ibkr + manual_short),
        long_term: aia.map(|aia| aia + mpf + manual_long),
    }
}

/// The Money Master challenge inputs (Overview!J29:N35): the stored
/// `money_master.*` settings plus the salary and the day `month_now` is
/// computed for.
#[derive(Debug, Clone)]
pub struct MoneyMasterInput {
    /// `money_master.start_date`; `month_now` derives from it.
    pub start_date: Option<chrono::NaiveDate>,
    /// `money_master.month_now` override — wins while set.
    pub month_now_override: Option<f64>,
    /// `money_master.saved` — the bank app's figure (L31).
    pub saved: Option<f64>,
    /// `money_master.coming_save` override — wins while set.
    pub coming_save_override: Option<f64>,
    /// `money_master.target_months`/`target_amount` (K32/L32).
    pub target_months: Option<f64>,
    pub target_amount: Option<f64>,
    /// `overview.salary` — feeds K34's derivation and K35.
    pub salary: Option<f64>,
    /// The day `month_now` is computed for.
    pub today: chrono::NaiveDate,
}

/// The derived Money Master block, each figure absent while its inputs are
/// missing. `month_now` is the bank's 1-indexed challenge month — override or
/// `full months elapsed since start_date + 1`. `months_left` counts the
/// current month and floors at 1 so a completed challenge still divides the
/// whole gap. `coming_save` is the bank's `salary + (target − saved) ÷
/// months_left` — negative while ahead of target — or its override; `can_use`
/// = `salary − coming_save` then equals the over-target excess.
pub fn money_master(input: &MoneyMasterInput) -> crate::models::MoneyMaster {
    let month_now = input.month_now_override.or_else(|| {
        input
            .start_date
            .map(|start| full_months_since(start, input.today) as f64 + 1.0)
    });
    let months_left = match (input.target_months, month_now) {
        (Some(target), Some(now)) => Some((target - now + 1.0).max(1.0)),
        _ => None,
    };
    let coming_save = input.coming_save_override.or_else(|| {
        match (input.salary, input.target_amount, input.saved, months_left) {
            (Some(salary), Some(target), Some(saved), Some(left)) => {
                Some(salary + (target - saved) / left)
            }
            _ => None,
        }
    });
    let avg_per_month = match (input.saved, month_now) {
        (Some(saved), Some(now)) if now != 0.0 => Some(saved / now),
        _ => None,
    };
    let time_progress = match (month_now, input.target_months) {
        (Some(now), Some(target)) if target != 0.0 => Some(now / target),
        _ => None,
    };
    let saved_progress = match (input.saved, input.target_amount) {
        (Some(saved), Some(target)) if target != 0.0 => Some(saved / target),
        _ => None,
    };
    crate::models::MoneyMaster {
        start_date: input.start_date.map(|date| date.to_string()),
        month_now,
        months_left,
        saved: input.saved,
        target_months: input.target_months,
        target_amount: input.target_amount,
        coming_save,
        avg_per_month,
        yearly_rate: avg_per_month.map(|avg| avg * 12.0),
        time_progress,
        saved_progress,
        progress_gap: match (saved_progress, time_progress) {
            (Some(saved), Some(time)) => Some(saved - time),
            _ => None,
        },
        can_use: match (input.salary, coming_save) {
            (Some(salary), Some(coming)) => Some(salary - coming),
            _ => None,
        },
    }
}

/// Whole months from `start` to `end` by the monthly anniversary — negative
/// while `end` precedes `start` (a challenge configured before it begins).
fn full_months_since(start: chrono::NaiveDate, end: chrono::NaiveDate) -> i64 {
    let mut months =
        (end.year() - start.year()) as i64 * 12 + (end.month() as i64 - start.month() as i64);
    if end.day() < start.day() {
        months -= 1;
    }
    months
}

/// The `Overview!J10:M20` + `N13:P16` goals inputs, resolved by the caller:
/// the stored settings, derived totals and averages, and the N4 rate with
/// the year it belongs to.
#[derive(Debug, Clone)]
pub struct OverviewGoalsInput {
    /// K13 = 港股 + 債券 + IBKR — absent while the IBKR HKD figure is missing.
    pub current_invest: Option<f64>,
    /// G7: the trailing 12-month 存 average (monthly); an L12 term ×12.
    pub avg_saved: Option<f64>,
    /// J22: `invest_targets.avg_invested` (already yearly).
    pub avg_invested: Option<f64>,
    /// M31: `money_master.avg_per_month` (monthly); an L12 term ×12.
    pub mm_avg_per_month: Option<f64>,
    /// 'Month Stat'!L8/N8/C8 running averages (monthly).
    pub saved_avg: Option<f64>,
    pub interest_avg: Option<f64>,
    pub total_change_avg: Option<f64>,
    /// N4 = (year, `income_value_rate`) of the latest completed year.
    pub n4: Option<(i32, f64)>,
    /// H1 流動資產 (O15) and B1 總數 (P15).
    pub liquid_assets: f64,
    pub total_assets: f64,
    /// H6 生活預算 (monthly): O14's base.
    pub living_budget: Option<f64>,
    /// E1 salary: P14's base.
    pub salary: Option<f64>,
    /// `overview.goal1_amount`/`overview.goal2_amount` — K12/K18.
    pub goal1_amount: Option<f64>,
    pub goal2_amount: Option<f64>,
    /// The effective formula constants (`1.07`/`30`/`0.7` while unset).
    pub goal3_inflation: f64,
    pub goal3_years: f64,
    pub final_replacement: f64,
    /// The day the ETAs are computed for.
    pub today: chrono::NaiveDate,
}

/// A goal's projected date: `today + round(365 × years)` days, `YYYY-MM-DD`;
/// absent while `years` is absent (missing inputs, zero divisor, reached).
fn goal_eta(today: chrono::NaiveDate, years: Option<f64>) -> Option<String> {
    let days = chrono::TimeDelta::try_days((365.0 * years?).round() as i64)?;
    today.checked_add_signed(days).map(|date| date.to_string())
}

/// A formula goal's remain/years/eta: `target − assets`, then `remain ÷ pace`.
/// `years` stays absent while `target`/`pace` is missing or `pace` is zero,
/// and once `remain ≤ 0` — the goal is reached (已達成), where the sheet
/// would show a negative year count.
fn formula_goal(
    target: Option<f64>,
    assets: f64,
    pace: Option<f64>,
    today: chrono::NaiveDate,
) -> crate::models::FormulaGoal {
    let remain = target.map(|target| target - assets);
    let years = match (remain, pace) {
        (Some(remain), Some(pace)) if pace != 0.0 && remain > 0.0 => Some(remain / pace),
        _ => None,
    };
    crate::models::FormulaGoal {
        target,
        remain,
        years,
        eta: goal_eta(today, years),
    }
}

/// The `Overview!J10:M20` + `N13:P16` 目標 block, derived on read:
///
/// `L12` = the sheet's `AVERAGE(G7*12, J22, M31*12, L8*12, (L8+N8)*12)` —
/// the mean over the present terms; `M12` = `L12/12`. `L13` = `K13 × N4`.
/// Each milestone: `remain = target − K13`, `years = remain ÷ L12`,
/// `eta = today + round(365 × years)` days. 3rd target `O14` =
/// `living_budget × inflation × 12 × years`, remain vs 流動資產, paced by
/// `(L8 + N8) × 12`. Final `P14` = `salary × replacement × 12 ÷ N4`, remain
/// vs 總數, paced by `C8 × 12`.
pub fn overview_goals(input: &OverviewGoalsInput) -> crate::models::OverviewGoals {
    use crate::models::{GoalMilestone, OverviewGoals};

    // L12: the sheet's AVERAGE skips absent terms.
    let combined_running = match (input.saved_avg, input.interest_avg) {
        (Some(saved), Some(interest)) => Some((saved + interest) * 12.0),
        _ => None,
    };
    let terms: Vec<f64> = [
        input.avg_saved.map(|avg| avg * 12.0),
        input.avg_invested,
        input.mm_avg_per_month.map(|avg| avg * 12.0),
        input.saved_avg.map(|avg| avg * 12.0),
        combined_running,
    ]
    .into_iter()
    .flatten()
    .collect();
    let saving_year = (!terms.is_empty()).then(|| terms.iter().sum::<f64>() / terms.len() as f64);

    let (n4_year, n4_rate) = match input.n4 {
        Some((year, rate)) => (Some(year), Some(rate)),
        None => (None, None),
    };
    let passive_return_year = match (input.current_invest, n4_rate) {
        (Some(invest), Some(rate)) => Some(invest * rate),
        _ => None,
    };

    // K14/L14/J11 for each milestone: the years stay absent once reached.
    let milestone = |target: Option<f64>| {
        let remain = match (target, input.current_invest) {
            (Some(target), Some(invest)) => Some(target - invest),
            _ => None,
        };
        let years = match (remain, saving_year) {
            (Some(remain), Some(saving)) if saving != 0.0 && remain > 0.0 => Some(remain / saving),
            _ => None,
        };
        GoalMilestone {
            target,
            remain,
            years,
            eta: goal_eta(input.today, years),
        }
    };

    let third_target = input
        .living_budget
        .map(|budget| budget * input.goal3_inflation * 12.0 * input.goal3_years);
    let third = formula_goal(
        third_target,
        input.liquid_assets,
        combined_running,
        input.today,
    );

    let final_target = match (input.salary, n4_rate) {
        (Some(salary), Some(rate)) if rate != 0.0 => {
            Some(salary * input.final_replacement * 12.0 / rate)
        }
        _ => None,
    };
    let final_goal = formula_goal(
        final_target,
        input.total_assets,
        input.total_change_avg.map(|avg| avg * 12.0),
        input.today,
    );

    OverviewGoals {
        n4_rate,
        n4_year,
        current_invest: input.current_invest,
        saving_year,
        saving_month: saving_year.map(|year| year / 12.0),
        passive_return_year,
        passive_return_month: passive_return_year.map(|year| year / 12.0),
        milestones: vec![milestone(input.goal1_amount), milestone(input.goal2_amount)],
        third,
        final_goal,
        goal3_inflation: input.goal3_inflation,
        goal3_years: input.goal3_years,
        final_replacement: input.final_replacement,
        saved_avg: input.saved_avg,
        interest_avg: input.interest_avg,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calc::approx_eq;

    #[test]
    fn live_totals_omit_rate_dependent_terms_without_a_rate() {
        let input = LiveTotalsInput {
            hk_market_value: 100.0,
            us_market_value: 10.0,
            usd_hkd_rate: Some(7.8),
            deposits_active_principal: 200.0,
            bonds_active_principal: 30.0,
            aia_value_usd: 5.0,
            mpf_balance: 40.0,
            manual_assets_sum: 6.0,
            cash_sum: 7.0,
            ibkr_hkd_cash: 2.0,
            ibkr_usd_cash: 1.0,
            pool_balance: 8.0,
        };
        let totals = live_totals(&input);
        // total = 100 + (10+1)×7.8 + 2 + 200 + 30 + 39 + 40 + 6 + 7
        assert!(approx_eq(totals.total_assets, 509.8));
        // liquid = 100 + 200 + 7 + 30 + (10+1)×7.8 + 2 − 8
        assert!(approx_eq(totals.liquid_assets, 416.8));

        let no_rate = live_totals(&LiveTotalsInput {
            usd_hkd_rate: None,
            ..input
        });
        // Without a rate the US and AIA terms drop out; HKD cash stays.
        assert!(approx_eq(no_rate.total_assets, 385.0));
        assert!(approx_eq(no_rate.liquid_assets, 331.0));
    }

    #[test]
    fn liquidity_tiers_reproduce_the_sheet_cells() {
        // The workbook's cached K4:K7 figures.
        let tiers = liquidity_tiers(
            Some(52700.0),
            477023.67,
            1_000_000.0,
            300_000.0,
            Some(184_503.865),
            Some(1_400_000.0),
            136_362.444,
            20_000.0,
            69_440.47,
        );
        assert!(approx_eq(tiers.cannot_use.unwrap(), 316200.0));
        assert!(approx_eq(tiers.can_use.unwrap(), 160823.67));
        assert!(approx_eq(tiers.short_term.unwrap(), 1504503.87));
        assert!(approx_eq(tiers.long_term.unwrap(), 1605802.91));

        // K4+K5 = B14 and K6+K7 = B10, so ΣK4:K7 = B1.
        let total_assets = 477023.67 + 1504503.865 + 1605802.914;
        let tiers_sum = tiers.can_use.unwrap()
            + tiers.cannot_use.unwrap()
            + tiers.short_term.unwrap()
            + tiers.long_term.unwrap();
        assert!(approx_eq(tiers_sum, total_assets));
    }

    #[test]
    fn liquidity_tiers_absences() {
        let tiers = liquidity_tiers(
            None,
            477023.67,
            1_000_000.0,
            300_000.0,
            Some(184_503.865),
            Some(1_400_000.0),
            136_362.444,
            20_000.0,
            69_440.47,
        );
        assert_eq!(tiers.can_use, None);
        assert_eq!(tiers.cannot_use, None);
        assert!(tiers.short_term.is_some());
        assert!(tiers.long_term.is_some());

        // No rate: the recoverable tiers stay absent rather than partial.
        let tiers = liquidity_tiers(
            Some(52700.0),
            477023.67,
            1_000_000.0,
            300_000.0,
            None,
            None,
            136_362.444,
            20_000.0,
            69_440.47,
        );
        assert!(tiers.can_use.is_some());
        assert!(tiers.cannot_use.is_some());
        assert_eq!(tiers.short_term, None);
        assert_eq!(tiers.long_term, None);
    }

    #[test]
    fn liquidity_tiers_can_use_goes_negative() {
        let tiers = liquidity_tiers(
            Some(52700.0),
            300_000.0,
            0.0,
            0.0,
            Some(0.0),
            Some(0.0),
            0.0,
            0.0,
            0.0,
        );
        assert!(approx_eq(tiers.can_use.unwrap(), -16200.0));
    }

    // ----- 預測 grid -----

    fn mm_input() -> MoneyMasterInput {
        MoneyMasterInput {
            start_date: Some(chrono::NaiveDate::from_ymd_opt(2023, 10, 27).unwrap()),
            month_now_override: None,
            saved: Some(1_094_405.06),
            coming_save_override: None,
            target_months: Some(36.0),
            target_amount: Some(1_000_000.0),
            salary: Some(52_700.0),
            today: chrono::NaiveDate::from_ymd_opt(2026, 9, 29).unwrap(),
        }
    }

    #[test]
    fn money_master_matches_the_bank_display() {
        // The bank app's live figures on 2026-09-29: month 36, saved
        // 1,094,405.06, coming save −41,705.06, can use 94,405.06.
        let mm = money_master(&mm_input());
        assert_eq!(mm.month_now, Some(36.0));
        assert_eq!(mm.months_left, Some(1.0));
        assert!(approx_eq(mm.coming_save.unwrap(), -41_705.06));
        assert!(approx_eq(mm.can_use.unwrap(), 94_405.06));
        assert!(approx_eq(mm.avg_per_month.unwrap(), 30_400.14));
        assert!(approx_eq(mm.saved_progress.unwrap(), 1.09440506));
        assert_eq!(mm.time_progress, Some(1.0));
        assert!(approx_eq(mm.progress_gap.unwrap(), 0.09440506));
    }

    #[test]
    fn money_master_month_counts_anniversaries_one_indexed() {
        let mut input = mm_input();
        // The anniversary day itself begins a new month.
        input.today = chrono::NaiveDate::from_ymd_opt(2026, 9, 26).unwrap();
        assert_eq!(money_master(&input).month_now, Some(35.0));
        input.today = chrono::NaiveDate::from_ymd_opt(2026, 9, 27).unwrap();
        assert_eq!(money_master(&input).month_now, Some(36.0));
        // The start day is month 1; before the start it counts down to 0.
        input.today = chrono::NaiveDate::from_ymd_opt(2023, 10, 27).unwrap();
        assert_eq!(money_master(&input).month_now, Some(1.0));
        input.today = chrono::NaiveDate::from_ymd_opt(2023, 10, 26).unwrap();
        assert_eq!(money_master(&input).month_now, Some(0.0));
    }

    #[test]
    fn money_master_overrides_pin_their_figures() {
        let mut input = mm_input();
        input.month_now_override = Some(35.0);
        input.coming_save_override = Some(2_726.57);
        let mm = money_master(&input);
        assert_eq!(mm.month_now, Some(35.0));
        assert_eq!(mm.months_left, Some(2.0));
        assert_eq!(mm.coming_save, Some(2_726.57));
        // Downstream derivations follow the overrides.
        assert!(approx_eq(mm.avg_per_month.unwrap(), 1_094_405.06 / 35.0));
        assert!(approx_eq(mm.can_use.unwrap(), 52_700.0 - 2_726.57));
    }

    #[test]
    fn money_master_months_left_floors_at_one_past_the_target() {
        let mut input = mm_input();
        input.today = chrono::NaiveDate::from_ymd_opt(2027, 3, 10).unwrap();
        let mm = money_master(&input);
        // Month 41: five months past the target, still dividing the gap by 1.
        assert_eq!(mm.month_now, Some(41.0));
        assert_eq!(mm.months_left, Some(1.0));
        assert!(mm.coming_save.unwrap() < 0.0);
    }

    #[test]
    fn money_master_figures_absent_without_inputs() {
        let mm = money_master(&MoneyMasterInput {
            start_date: None,
            month_now_override: None,
            saved: None,
            coming_save_override: None,
            target_months: None,
            target_amount: None,
            salary: None,
            today: chrono::NaiveDate::from_ymd_opt(2026, 9, 29).unwrap(),
        });
        assert!(mm.month_now.is_none());
        assert!(mm.coming_save.is_none());
        assert!(mm.can_use.is_none());
        assert!(mm.avg_per_month.is_none());
        assert!(mm.progress_gap.is_none());
    }

    // --- Goals (Overview!J10:M20 + N13:P16) ---

    /// The workbook's cached inputs/outputs (recomputed offline).
    fn goals_input() -> OverviewGoalsInput {
        OverviewGoalsInput {
            current_invest: Some(1495563.431),
            avg_saved: Some(30013.27),
            avg_invested: Some(326841.2233),
            mm_avg_per_month: Some(30400.14056),
            saved_avg: Some(26362.84273),
            interest_avg: Some(4480.899118),
            total_change_avg: Some(59918.15472),
            n4: Some((2025, 0.03851965006)),
            liquid_assets: 1951776.034,
            total_assets: 3579321.076,
            living_budget: Some(14900.0),
            salary: Some(52_700.0),
            goal1_amount: Some(1_550_000.0),
            goal2_amount: Some(2_500_000.0),
            goal3_inflation: 1.07,
            goal3_years: 30.0,
            final_replacement: 0.7,
            today: chrono::NaiveDate::from_ymd_opt(2026, 9, 29).unwrap(),
        }
    }

    #[test]
    fn overview_goals_reproduce_the_sheet_cells() {
        let input = goals_input();
        let goals = overview_goals(&input);
        // The formula goals' ETAs follow the same today + round(365 × years)
        // rule as the milestones' J11/J17.
        let expected_eta = |years: f64| {
            input
                .today
                .checked_add_signed(chrono::TimeDelta::days((365.0 * years).round() as i64))
                .map(|date| date.to_string())
        };
        assert_eq!(goals.n4_year, Some(2025));
        assert!(approx_eq(goals.n4_rate.unwrap(), 0.03851965006));
        // The running averages echo so the card can expand 存/年.
        assert_eq!(goals.saved_avg, input.saved_avg);
        assert_eq!(goals.interest_avg, input.interest_avg);
        assert!(approx_eq(goals.saving_year.unwrap(), 347656.233));
        assert!(approx_eq(goals.saving_month.unwrap(), 28971.35275));
        assert!(approx_eq(goals.passive_return_year.unwrap(), 57608.58002));
        assert!(approx_eq(
            goals.passive_return_month.unwrap(),
            57608.58002 / 12.0
        ));

        let first = &goals.milestones[0];
        assert_eq!(first.target, Some(1_550_000.0));
        assert!(approx_eq(first.remain.unwrap(), 54436.56857));
        assert!(approx_eq(first.years.unwrap(), 0.1565815982));
        // J11 = today + round(365 × 0.1565815982) days.
        assert_eq!(first.eta.as_deref(), Some("2026-11-25"));
        let second = &goals.milestones[1];
        assert_eq!(second.target, Some(2_500_000.0));
        assert!(approx_eq(second.remain.unwrap(), 1004436.569));
        assert!(approx_eq(second.years.unwrap(), 2.889166002));
        // round(365 × 2.889166002) = 1055 days — one day later than the
        // sheet's truncating DATEVALUE result.
        assert_eq!(second.eta.as_deref(), Some("2029-08-19"));

        assert!(approx_eq(goals.third.target.unwrap(), 5739480.0));
        assert!(approx_eq(goals.third.remain.unwrap(), 3787703.966));
        assert!(approx_eq(goals.third.years.unwrap(), 10.23358316));
        assert_eq!(goals.third.eta, expected_eta(10.23358316));
        assert!(approx_eq(goals.final_goal.target.unwrap(), 11492316.24));
        assert!(approx_eq(goals.final_goal.remain.unwrap(), 7912995.166));
        assert!(approx_eq(goals.final_goal.years.unwrap(), 11.00528324));
        assert_eq!(goals.final_goal.eta, expected_eta(11.00528324));
    }

    #[test]
    fn overview_goals_skip_absent_terms_and_missing_inputs() {
        let mut input = goals_input();
        // AVERAGE over the four remaining terms.
        input.avg_invested = None;
        let goals = overview_goals(&input);
        let expected = (30013.27 * 12.0
            + 30400.14056 * 12.0
            + 26362.84273 * 12.0
            + (26362.84273 + 4480.899118) * 12.0)
            / 4.0;
        assert!(approx_eq(goals.saving_year.unwrap(), expected));

        // Without a rate the IBKR HKD figure is absent, so K13 and every
        // figure derived from it are absent; the formula goals still work.
        input.current_invest = None;
        input.n4 = None;
        let goals = overview_goals(&input);
        assert!(goals.n4_rate.is_none() && goals.n4_year.is_none());
        assert!(goals.current_invest.is_none());
        assert!(goals.passive_return_year.is_none());
        for milestone in &goals.milestones {
            assert!(milestone.remain.is_none() && milestone.years.is_none());
            assert!(milestone.eta.is_none());
        }
        assert!(goals.final_goal.target.is_none());
        assert!(goals.final_goal.eta.is_none());
        assert!(goals.third.target.is_some());
        assert!(goals.third.eta.is_some());
    }

    #[test]
    fn overview_goals_mark_reached_goals() {
        let mut input = goals_input();
        input.goal1_amount = Some(1_000_000.0); // below current_invest
        input.total_assets = 20_000_000.0; // above the Final Target
        let goals = overview_goals(&input);
        assert!(goals.milestones[0].remain.unwrap() < 0.0);
        assert!(goals.milestones[0].years.is_none());
        assert!(goals.milestones[0].eta.is_none());
        assert!(goals.final_goal.remain.unwrap() < 0.0);
        assert!(goals.final_goal.years.is_none());
        assert!(goals.final_goal.eta.is_none());
    }

    #[test]
    fn overview_goals_divisors_and_cleared_amounts() {
        let mut input = goals_input();
        input.goal1_amount = None;
        input.salary = None;
        input.total_change_avg = Some(0.0);
        let goals = overview_goals(&input);
        assert_eq!(goals.milestones[0].target, None);
        assert!(goals.milestones[0].remain.is_none());
        // salary missing and the pace divisor is zero — both absent.
        assert!(goals.final_goal.target.is_none());
        assert!(goals.final_goal.years.is_none());
        // No saving terms at all → saving_year absent → milestone years too.
        input.avg_saved = None;
        input.avg_invested = None;
        input.mm_avg_per_month = None;
        input.saved_avg = None;
        input.interest_avg = None;
        let goals = overview_goals(&input);
        assert!(goals.saving_year.is_none());
        assert!(goals.saved_avg.is_none() && goals.interest_avg.is_none());
        assert!(goals.milestones[1].years.is_none());
        // The combined running term is absent too, so 3rd years go absent.
        assert!(goals.third.years.is_none());
        assert!(goals.third.eta.is_none());
        assert!(goals.third.remain.is_some());
    }

    #[test]
    fn overview_goals_constants_override_the_defaults() {
        let mut input = goals_input();
        input.goal3_inflation = 1.05;
        input.goal3_years = 20.0;
        input.final_replacement = 0.6;
        let goals = overview_goals(&input);
        assert!(approx_eq(
            goals.third.target.unwrap(),
            14900.0 * 1.05 * 12.0 * 20.0
        ));
        assert!(approx_eq(
            goals.final_goal.target.unwrap(),
            52_700.0 * 0.6 * 12.0 / 0.03851965006
        ));
        // The effective constants echo so the card can expand the formulas.
        assert_eq!(goals.goal3_inflation, 1.05);
        assert_eq!(goals.goal3_years, 20.0);
        assert_eq!(goals.final_replacement, 0.6);
    }
}
