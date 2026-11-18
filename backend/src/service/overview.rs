use chrono::Datelike;
use serde::Serialize;
use sqlx::SqlitePool;
use utoipa::ToSchema;

use crate::calc::{FieldError, liquidity_tiers};
use crate::models::{CashAccount, Currency, IbkrPatch, ManualAssetLiquidity};
use crate::service::ServiceError;
use crate::store::{StoreError, cash_accounts, ibkr as ibkr_store, settings, today};

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
    /// The manual row's note, shown as a hover tooltip on 總覽.
    pub note: Option<String>,
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
    pub invest_targets: crate::calc::InvestTargets,
    /// J3:K7.
    pub liquidity_tiers: crate::calc::LiquidityTiers,
    /// J29:N35.
    pub money_master: crate::calc::MoneyMaster,
    /// J10:M20 + N13:P16.
    pub goals: crate::calc::OverviewGoals,
}

/// The IBKR block: stored inputs plus the derived 美股!B7/C1/C2 figures.
/// The cash figures are the `broker` cash_accounts rows; the reported total
/// is the `ibkr.reported_value` setting; the rate is the fx_rates USD row.
pub async fn ibkr_block(pool: &SqlitePool) -> Result<IbkrBlock, StoreError> {
    let reads = super::wealth::ibkr_reads(pool).await?;
    let transferred_hkd = ibkr_store::transferred_hkd(pool).await?;
    let now_value = settings::Settings::load(pool).await?.ibkr_reported_value;
    Ok(compose_ibkr_block(
        transferred_hkd,
        now_value,
        reads.hkd_cash,
        reads.usd_cash,
        reads.us_market_value,
        crate::store::fx::usd_rate(pool).await?,
    ))
}

/// `ibkr_block` for a caller already holding the `LiveContext` — only the
/// transferred sum is loaded; every other read comes from the context.
async fn ibkr_block_from(
    pool: &SqlitePool,
    ctx: &super::wealth::LiveContext,
) -> Result<IbkrBlock, StoreError> {
    Ok(compose_ibkr_block(
        ibkr_store::transferred_hkd(pool).await?,
        ctx.stored.ibkr_reported_value,
        ctx.reads.hkd_cash,
        ctx.reads.usd_cash,
        ctx.reads.us_market_value,
        ctx.input.usd_hkd_rate,
    ))
}

fn compose_ibkr_block(
    transferred_hkd: Option<f64>,
    now_value: Option<f64>,
    hkd_cash: Option<f64>,
    usd_cash: Option<f64>,
    stock_value_usd: Option<f64>,
    rate: Option<f64>,
) -> IbkrBlock {
    // 美股!B7 `manual cal now`; the sheet cannot produce it without N3 either.
    let computed_total_hkd = rate.map(|rate| {
        (stock_value_usd.unwrap_or(0.0) + usd_cash.unwrap_or(0.0)) * rate + hkd_cash.unwrap_or(0.0)
    });
    let net = match (now_value, transferred_hkd) {
        (Some(now), Some(transferred)) => Some(now - transferred),
        _ => None,
    };
    let net_pct = match (net, transferred_hkd) {
        (Some(net), Some(transferred)) if transferred != 0.0 => Some(net / transferred),
        _ => None,
    };
    let vs_now_value = match (computed_total_hkd, now_value) {
        (Some(computed), Some(now)) => Some(computed - now),
        _ => None,
    };
    IbkrBlock {
        transferred_hkd,
        now_value,
        hkd_cash,
        usd_cash,
        stock_value_usd,
        computed_total_hkd,
        net,
        net_pct,
        vs_now_value,
    }
}

/// `PATCH /api/ibkr`: validate the patch, then one transaction stores the
/// reported total and sets the broker account amounts (a `null` cash value
/// sets 0). Transfers live on `POST /api/ibkr/transfers` — a patch never
/// appends a log row.
pub async fn update_ibkr(pool: &SqlitePool, patch: &IbkrPatch) -> Result<IbkrBlock, ServiceError> {
    let mut errors = Vec::new();
    for (field, value) in [
        ("now_value", patch.now_value),
        ("hkd_cash", patch.hkd_cash),
        ("usd_cash", patch.usd_cash),
    ] {
        if let Some(Some(value)) = value
            && (!value.is_finite() || value < 0.0)
        {
            errors.push(FieldError::new(field, "value must not be negative"));
        }
    }
    if !errors.is_empty() {
        return Err(ServiceError::Validation(errors));
    }

    let now = crate::store::now_timestamp();
    let mut tx = pool.begin().await.map_err(StoreError::from)?;
    if let Some(value) = patch.now_value {
        settings::SettingsPatch {
            ibkr_reported_value: Some(value),
            ..Default::default()
        }
        .apply(&mut tx)
        .await?;
    }
    for (currency, value) in [
        (Currency::Hkd, patch.hkd_cash),
        (Currency::Usd, patch.usd_cash),
    ] {
        if let Some(value) = value {
            let account = cash_accounts::broker(&mut *tx, currency)
                .await?
                .ok_or_else(|| {
                    ServiceError::Conflict(format!("no IBKR {} account", currency.as_str()))
                })?;
            // A `null` cash cell records 0, not "unset".
            cash_accounts::set_amount(&mut tx, account.id, value.unwrap_or(0.0), &now).await?;
        }
    }
    tx.commit().await?;
    ibkr_block(pool).await.map_err(ServiceError::from)
}

/// `Overview!A3:C18` plus the B1/H1/J1 headline, derived on read.
pub async fn build(pool: &SqlitePool) -> Result<OverviewResponse, StoreError> {
    let ctx = super::wealth::LiveContext::open(pool).await?;
    let input = &ctx.input;
    let totals = ctx.totals;
    let ibkr = ibkr_block_from(pool, &ctx).await?;
    let stored = &ctx.stored;
    let salary = stored.salary;
    let semi_target = ctx.semi_liquid_target();
    let manual = crate::store::manual_assets::load_all(pool).await?;
    let bank_accounts = cash_accounts::load_all(pool).await?;

    let aia_hkd = input.usd_hkd_rate.map(|rate| input.aia_value_usd * rate);
    let mut assets = vec![
        asset_row("hk", "港股", Some(input.hk_market_value)),
        asset_row("bonds", "債券", Some(input.bonds_active_principal)),
        asset_row("aia", "基金", aia_hkd),
        asset_row("mpf", "MPF", Some(input.mpf_balance)),
    ];
    // Manual `asset` rows also feed the 策略 tiers by their liquidity.
    let mut manual_short = 0.0;
    let mut manual_long = 0.0;
    for asset in manual.iter() {
        match asset.liquidity {
            ManualAssetLiquidity::Short => manual_short += asset.amount,
            ManualAssetLiquidity::Long => manual_long += asset.amount,
        }
        assets.push(OverviewAssetRow {
            key: "manual".to_string(),
            label: asset.label.clone(),
            amount: Some(asset.amount),
            share: None,
            manual_asset_id: Some(asset.id),
            note: asset.note.clone(),
        });
    }
    assets.push(asset_row("ibkr", "IBKR", ibkr.computed_total_hkd));

    let assets_sum: f64 = assets.iter().filter_map(|row| row.amount).sum();
    for row in &mut assets {
        row.share = row
            .amount
            .filter(|_| assets_sum != 0.0)
            .map(|amount| amount / assets_sum);
    }

    // The bank accounts only — broker cash lives in the IBKR block.
    let cash_rows: Vec<CashAccount> = bank_accounts
        .into_iter()
        .filter(|account| account.role == crate::models::CashRole::Bank)
        .collect();
    let semi_total = input.deposits_active_principal + input.cash_sum;
    let semi_liquid = SemiLiquid {
        deposits: input.deposits_active_principal,
        cash_rows,
        cash_sum: input.cash_sum,
        total: semi_total,
        vs_quarter_liquid: semi_total - semi_target * totals.liquid_assets,
        // A13 = B14 ÷ (港股 + 債券 + 半流動資金 + IBKR).
        share: ibkr.computed_total_hkd.and_then(|ibkr_total| {
            let divisor =
                input.hk_market_value + input.bonds_active_principal + semi_total + ibkr_total;
            (divisor != 0.0).then_some(semi_total / divisor)
        }),
    };

    // F3:G10 + H6 — live-filled totals feed the window's edge (e.g. the last
    // completed month's Changed needs the live current row's 總數).
    let trailing = ctx.book.trailing_averages();
    // The 預測 floor: budget is red below 流動資產 × 0.01% × 30 + 9000.
    let living_budget_floor = totals.liquid_assets * 0.0001 * 30.0 + 9000.0;
    let averages = TwelveMonthAverages {
        total_change: trailing.total_change,
        month_spend: trailing.month_spend,
        living_spend: trailing.living_spend,
        living_budget: trailing.living_budget,
        living_budget_low: trailing
            .living_budget
            .is_some_and(|budget| budget < living_budget_floor),
        living_budget_floor,
        saved: trailing.saved,
        interest: trailing.interest,
        pool_balance: input.pool_balance,
        window_start: trailing.window_start.map(|month| month.to_string()),
        window_end: trailing.window_end.map(|month| month.to_string()),
    };

    // J22:N27 投資目標 — a pure projection of the year-review rows.
    let review = super::year_review::build(pool).await?;
    let invest_targets = crate::calc::invest_targets(&review.years, today().year());

    // J29:N35 Money Master — the bank challenge's stored figures plus the
    // derived month/coming-save (each with an optional meta override).
    let money_master = crate::calc::money_master(&crate::calc::MoneyMasterInput {
        start_date: stored
            .money_master_start_date
            .as_deref()
            .and_then(|date| chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()),
        month_now_override: stored.money_master_month_now,
        saved: stored.money_master_saved,
        coming_save_override: stored.money_master_coming_save,
        target_months: stored.money_master_target_months,
        target_amount: stored.money_master_target_amount,
        salary,
        today: today(),
    });

    // J10:M20 + N13:P16 目標 — the milestone amounts are stored settings;
    // everything else derives. N4 = the latest completed year's
    // income_value_rate from the year-review rows.
    let running = ctx.book.running_averages();
    let n4_year = today().year() - 1;
    let n4 = review
        .years
        .iter()
        .find(|row| row.year == n4_year)
        .and_then(|row| row.assets.income_value_rate.map(|rate| (row.year, rate)));
    let (goal3_inflation, goal3_years, final_replacement) =
        super::settings::goal_constants_of(stored);
    let goals = crate::calc::overview_goals(&crate::calc::OverviewGoalsInput {
        // K13 = 港股 + 債券 + IBKR — absent while the rate-derived B9 is.
        current_invest: ibkr
            .computed_total_hkd
            .map(|total| input.hk_market_value + input.bonds_active_principal + total),
        avg_saved: trailing.saved,
        avg_invested: invest_targets.avg_invested,
        mm_avg_per_month: money_master.avg_per_month,
        saved_avg: running.saved_avg,
        interest_avg: running.interest_avg,
        total_change_avg: running.total_change_avg,
        n4,
        liquid_assets: totals.liquid_assets,
        total_assets: totals.total_assets,
        living_budget: averages.living_budget,
        salary,
        goal1_amount: stored.goal1_amount,
        goal2_amount: stored.goal2_amount,
        goal3_inflation,
        goal3_years,
        final_replacement,
        today: today(),
    });

    // J3:K7 策略 — 總數 re-partitioned into liquidity tiers.
    let liquidity_tiers = liquidity_tiers(
        salary,
        semi_total,
        input.hk_market_value,
        input.bonds_active_principal,
        ibkr.computed_total_hkd,
        aia_hkd,
        input.mpf_balance,
        manual_short,
        manual_long,
    );

    Ok(OverviewResponse {
        today: today().to_string(),
        rate: input.usd_hkd_rate,
        salary,
        total_assets: totals.total_assets,
        liquid_assets: totals.liquid_assets,
        liquid_ratio: salary.map(|salary| totals.liquid_assets / (salary * 100.0)),
        assets,
        assets_sum,
        semi_liquid,
        semi_liquid_target: semi_target,
        ibkr,
        averages,
        invest_targets,
        liquidity_tiers,
        money_master,
        goals,
    })
}

fn asset_row(key: &str, label: &str, amount: Option<f64>) -> OverviewAssetRow {
    OverviewAssetRow {
        key: key.to_string(),
        label: label.to_string(),
        amount,
        share: None,
        manual_asset_id: None,
        note: None,
    }
}
