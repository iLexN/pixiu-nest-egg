use super::CashAccount;
use super::nullable;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// The 美股 sheet's IBKR account block (A1:B5 + B7): four manual inputs plus
/// the derived cross-checks. `now_value` is the account total as the IBKR app
/// displays it — its implied FX rate differs from `aia.usd_hkd_rate`.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct IbkrBlock {
    /// 美股!B1: cumulative bank→IBKR transfers — Σ `ibkr_transfers`.
    pub transferred_hkd: Option<f64>,
    /// 美股!B2: the account total shown in the IBKR app.
    pub now_value: Option<f64>,
    /// 美股!B4/B5: the account's cash positions.
    pub hkd_cash: Option<f64>,
    pub usd_cash: Option<f64>,
    /// US stock market value in USD (美股!E2).
    pub stock_value_usd: Option<f64>,
    /// 美股!B7 = (stock_value_usd + usd_cash) × rate + hkd_cash — the figure
    /// Overview!B9 links to.
    pub computed_total_hkd: Option<f64>,
    /// C1 = now_value − transferred_hkd; C2 = net ÷ transferred.
    pub net: Option<f64>,
    pub net_pct: Option<f64>,
    /// computed_total_hkd − now_value: the FX-gap cross-check.
    pub vs_now_value: Option<f64>,
}

/// Absent fields are left untouched; `null` clears a field. `transfer_hkd`
/// appends a dated row to `ibkr_transfers` instead — a positive value is a
/// bank→IBKR transfer, a negative one a withdrawal/correction.
#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct IbkrPatch {
    #[serde(default)]
    pub transfer_hkd: Option<f64>,
    /// Optional `YYYY-MM-DD` for the transfer; defaults to today.
    #[serde(default)]
    pub transfer_date: Option<String>,
    #[serde(default, deserialize_with = "nullable")]
    pub now_value: Option<Option<f64>>,
    #[serde(default, deserialize_with = "nullable")]
    pub hkd_cash: Option<Option<f64>>,
    #[serde(default, deserialize_with = "nullable")]
    pub usd_cash: Option<Option<f64>>,
}

/// One row of the 總覽 asset table (Overview!A3:C9): a module total or a
/// manual `asset` row, with its share of the sum (the C column).
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct OverviewAssetRow {
    pub key: String,
    pub label: String,
    /// The row's HKD amount; absent while rate-dependent and no rate is set.
    pub amount: Option<f64>,
    /// amount ÷ assets_sum.
    pub share: Option<f64>,
    /// Set when the row is a manual balance, enabling inline edits.
    pub manual_asset_id: Option<i64>,
}

/// The Overview!A14:C18 半流動資金 block plus the A13 ratio.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SemiLiquid {
    /// 已定期 (B15): Σ principal over active deposits.
    pub deposits: f64,
    /// The bank `cash` accounts (B16/B17).
    pub cash_rows: Vec<CashAccount>,
    /// 活期 (B18) = Σ cash rows.
    pub cash_sum: f64,
    /// 半流動資金 (B14) = deposits + cash_sum.
    pub total: f64,
    /// C14 = total − `overview.semi_liquid_target` × liquid_assets (25%
    /// while unset).
    pub vs_quarter_liquid: f64,
    /// A13 = total ÷ (港股 + 債券 + total + IBKR); absent while a term is missing.
    pub share: Option<f64>,
}

/// `Overview!F3:G10` (+`H6`): trailing averages over the 12 completed months
/// before the current one, the live pool balance, and the living budget with
/// its 預測 red-flag floor.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct TwelveMonthAverages {
    /// G4 總數增加.
    pub total_change: Option<f64>,
    /// G5 支出.
    pub month_spend: Option<f64>,
    /// G6 生活支出.
    pub living_spend: Option<f64>,
    /// H6 生活預算 = ROUNDUP(living_spend × 1.05, −2).
    pub living_budget: Option<f64>,
    /// The 預測 check: true while `living_budget < living_budget_floor`.
    pub living_budget_low: bool,
    /// `liquid_assets × 0.0001 × 30 + 9000` — the floor the budget is
    /// compared against.
    pub living_budget_floor: f64,
    /// G7 存.
    pub saved: Option<f64>,
    /// G8 利息.
    pub interest: Option<f64>,
    /// G10 開心Pool — the live balance.
    pub pool_balance: f64,
    /// The window's first/last months (e.g. "2025-09-01").
    pub window_start: Option<String>,
    pub window_end: Option<String>,
}

/// The 投資目標 block (Overview!J22:N27): the J22 three-year invested average
/// plus one row per year-review year under the unified target formula.
#[derive(Debug, Clone, Default, Serialize, ToSchema)]
pub struct InvestTargets {
    /// J22: mean `invested` over the last three completed years, skip-absent.
    pub avg_invested: Option<f64>,
    pub rows: Vec<InvestTargetRow>,
}

/// One 投資目標 row (J:N): invested, the effective raise, and the derived
/// target/remain/growth.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct InvestTargetRow {
    pub year: i32,
    /// K column: the year-review `invested`; absent while none.
    pub invested: Option<f64>,
    /// The year's effective 月薪增幅 (override or salary-derived).
    pub raise: Option<f64>,
    /// The year-review 利息回報 feeding the target formula (0 when none).
    pub interest: f64,
    /// The year-review 開心Pool支出 feeding the target formula (0 when none).
    pub pool_spend: f64,
    /// L column: the year's target; absent without a prior-year `invested`.
    pub target: Option<f64>,
    /// M column: target − invested, on the current year only.
    pub remain: Option<f64>,
    /// N column: invested YoY for completed years; target vs last invested
    /// for the current year.
    pub growth: Option<f64>,
}

/// The Overview `J3:K7` 策略 block: 總數 re-partitioned into liquidity tiers.
#[derive(Debug, Clone, Default, Serialize, ToSchema)]
pub struct LiquidityTiers {
    /// K4 可動用 = 半流動資金 − `cannot_use`; absent without a stored salary.
    pub can_use: Option<f64>,
    /// K5 不可動用 = salary × 6 (the sheet's `N6`); absent without a salary.
    pub cannot_use: Option<f64>,
    /// K6 短期可取回 = 港股 + 債券 + IBKR + `short` manual asset rows;
    /// absent while `aia.usd_hkd_rate` is unset.
    pub short_term: Option<f64>,
    /// K7 長期可取回 = 基金 + MPF + `long` manual asset rows; absent while
    /// `aia.usd_hkd_rate` is unset.
    pub long_term: Option<f64>,
}

/// The `Overview!J29:N35` Money Master challenge block: the bank app's
/// stored figures, the effective month/coming-save (override or derived),
/// and the sheet's progress/pace derivations.
#[derive(Debug, Clone, Default, Serialize, ToSchema)]
pub struct MoneyMaster {
    /// `money_master.start_date` — the challenge start `YYYY-MM-DD`.
    pub start_date: Option<String>,
    /// K31: override or `full months elapsed since start_date + 1` (the
    /// bank's 1-indexed month counter).
    pub month_now: Option<f64>,
    /// Months left counting the current challenge month, floored at 1.
    pub months_left: Option<f64>,
    /// L31: the bank app's saved figure.
    pub saved: Option<f64>,
    /// K32/L32: the challenge parameters.
    pub target_months: Option<f64>,
    pub target_amount: Option<f64>,
    /// K34: override or `salary + (target_amount − saved) ÷ months_left`;
    /// negative while ahead of target.
    pub coming_save: Option<f64>,
    /// M31 = saved ÷ month_now.
    pub avg_per_month: Option<f64>,
    /// N31 = avg_per_month × 12.
    pub yearly_rate: Option<f64>,
    /// K33 = month_now ÷ target_months.
    pub time_progress: Option<f64>,
    /// L33 = saved ÷ target_amount.
    pub saved_progress: Option<f64>,
    /// M33 = saved_progress − time_progress.
    pub progress_gap: Option<f64>,
    /// K35 = salary − coming_save; may exceed salary while coming_save < 0.
    pub can_use: Option<f64>,
}

/// One `J10:M20` milestone (the sheet's 1st/2nd 目標): a fixed stored target
/// and its derived remain/years/ETA. Every figure is absent while its inputs
/// are missing; `years`/`eta` are also absent once `remain ≤ 0` (已達成).
#[derive(Debug, Clone, Default, Serialize, ToSchema)]
pub struct GoalMilestone {
    /// K12/K18: the target amount (`overview.goal1_amount`/`goal2_amount`).
    pub target: Option<f64>,
    /// K14/K20 = target − current_invest.
    pub remain: Option<f64>,
    /// L14/L20 = remain ÷ saving_year.
    pub years: Option<f64>,
    /// J11/J17 = today + round(365 × years) days, `YYYY-MM-DD`.
    pub eta: Option<String>,
}

/// One `N13:P16` formula goal (the sheet's 3rd target / Final Target): the
/// target is a formula, not a stored amount.
#[derive(Debug, Clone, Default, Serialize, ToSchema)]
pub struct FormulaGoal {
    /// O14/P14: the formula target.
    pub target: Option<f64>,
    /// O15/P15 = target − 流動資產 (3rd) / 總數 (final).
    pub remain: Option<f64>,
    /// O16/P16 = remain ÷ the goal's yearly pace; absent once `remain ≤ 0`.
    pub years: Option<f64>,
    /// today + round(365 × years) days, `YYYY-MM-DD` — absent while `years`
    /// is absent, including once the goal is reached.
    pub eta: Option<String>,
}

/// The `Overview!J10:M20` + `N13:P16` 目標 block: the shared saving/return
/// figures, the two stored-amount milestones, and the two formula targets.
#[derive(Debug, Clone, Default, Serialize, ToSchema)]
pub struct OverviewGoals {
    /// N4 = `income_value_rate` of the latest completed year (today's − 1);
    /// both absent while that year's row or rate is missing.
    pub n4_rate: Option<f64>,
    pub n4_year: Option<i32>,
    /// K13 = 港股 + 債券 + IBKR; absent while the IBKR HKD total is absent.
    pub current_invest: Option<f64>,
    /// L12 = the sheet's AVERAGE over the five saving sources, absent terms
    /// skipped; M12 = L12 ÷ 12.
    pub saving_year: Option<f64>,
    pub saving_month: Option<f64>,
    /// L13 = K13 × N4; M13 = L13 ÷ 12.
    pub passive_return_year: Option<f64>,
    pub passive_return_month: Option<f64>,
    /// The 1st/2nd 目標 rows, in order.
    pub milestones: Vec<GoalMilestone>,
    /// O14:O16 3rd target = 生活預算 × inflation × 12 × years vs 流動資產.
    pub third: FormulaGoal,
    /// P14:P16 Final Target = salary × replacement × 12 ÷ N4 vs 總數.
    pub final_goal: FormulaGoal,
    /// The effective formula constants (`1.07`/`30`/`0.7` while unset) —
    /// echoed so the card can expand O14/P14 without a settings fetch.
    pub goal3_inflation: f64,
    pub goal3_years: f64,
    pub final_replacement: f64,
    /// 'Month Stat' running averages (L8/N8/C8, monthly) — the last two L12
    /// terms plus the Final goal's pace base, echoed so the card can expand
    /// 存/年 and label each goal's pace.
    pub saved_avg: Option<f64>,
    pub interest_avg: Option<f64>,
    pub total_change_avg: Option<f64>,
}

/// `GET /api/overview`: the sheet's A3:C18 block plus the B1/H1/J1 headline.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct OverviewResponse {
    pub today: String,
    /// `aia.usd_hkd_rate`; absent while unset.
    pub rate: Option<f64>,
    /// `overview.salary`; feeds J1.
    pub salary: Option<f64>,
    /// B1 總數 = assets_sum + 半流動資金.
    pub total_assets: f64,
    /// H1 流動資產 = 港股 + 半流動資金 + 債券 + IBKR − 開心Pool.
    pub liquid_assets: f64,
    /// J1 = liquid_assets ÷ (salary × 100); absent without a salary.
    pub liquid_ratio: Option<f64>,
    pub assets: Vec<OverviewAssetRow>,
    /// B10 Sum = Σ present asset rows.
    pub assets_sum: f64,
    pub semi_liquid: SemiLiquid,
    /// `overview.semi_liquid_target` — the effective buffer ratio (0.25 while
    /// unset), so the view can label the C14 percent.
    pub semi_liquid_target: f64,
    pub ibkr: IbkrBlock,
    /// F3:G10 + H6.
    pub averages: TwelveMonthAverages,
    /// J22:N27.
    pub invest_targets: InvestTargets,
    /// J3:K7.
    pub liquidity_tiers: LiquidityTiers,
    /// J29:N35.
    pub money_master: MoneyMaster,
    /// J10:M20 + N13:P16.
    pub goals: OverviewGoals,
}
