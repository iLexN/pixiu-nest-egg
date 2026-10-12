use chrono::Datelike;
use sqlx::SqlitePool;

use crate::calc::{liquidity_tiers, live_totals, month_running_averages, trailing_averages};
use crate::models::{
    IbkrBlock, ManualAsset, ManualAssetKind, ManualAssetLiquidity, OverviewAssetRow,
    OverviewResponse, SemiLiquid, TwelveMonthAverages,
};
use crate::store::{StoreError, today};

/// The IBKR block: stored inputs plus the derived 美股!B7/C1/C2 figures.
pub async fn ibkr_block(pool: &SqlitePool) -> Result<IbkrBlock, StoreError> {
    let transferred_hkd = crate::store::overview::transferred_hkd(pool).await?;
    let now_value =
        crate::store::meta::meta_f64(pool, crate::store::overview::IBKR_NOW_VALUE_KEY).await?;
    let hkd_cash =
        crate::store::meta::meta_f64(pool, crate::store::overview::IBKR_HKD_CASH_KEY).await?;
    let usd_cash =
        crate::store::meta::meta_f64(pool, crate::store::overview::IBKR_USD_CASH_KEY).await?;
    let rate = crate::store::meta::meta_f64(pool, crate::store::aia::RATE_KEY).await?;
    let stock_value_usd =
        super::summary::market_value_only(pool, crate::models::Market::Us).await?;

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
    Ok(IbkrBlock {
        transferred_hkd,
        now_value,
        hkd_cash,
        usd_cash,
        stock_value_usd,
        computed_total_hkd,
        net,
        net_pct,
        vs_now_value,
    })
}

/// `Overview!A3:C18` plus the B1/H1/J1 headline, derived on read.
pub async fn build(pool: &SqlitePool) -> Result<OverviewResponse, StoreError> {
    let input = super::months::live_totals_input(pool).await?;
    let totals = live_totals(&input);
    let ibkr = ibkr_block(pool).await?;
    let salary = crate::store::meta::meta_f64(pool, crate::store::months::SALARY_KEY).await?;
    let semi_target = super::months::semi_liquid_target(pool).await?;
    let manual = crate::store::months::load_assets(pool).await?;
    let items = crate::store::months::load_all_items(pool).await?;

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
    for asset in manual
        .iter()
        .filter(|asset| asset.kind == ManualAssetKind::Asset)
    {
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

    let cash_rows: Vec<ManualAsset> = manual
        .into_iter()
        .filter(|asset| asset.kind == ManualAssetKind::Cash)
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
    let stat_rows = super::months::load_stat_rows(pool, Some(&totals)).await?;
    let trailing = trailing_averages(&stat_rows, &items, super::months::live_from());
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
        start_date: crate::store::meta::meta_get(pool, crate::store::months::MM_START_DATE_KEY)
            .await?
            .and_then(|date| chrono::NaiveDate::parse_from_str(&date, "%Y-%m-%d").ok()),
        month_now_override: crate::store::meta::meta_f64(
            pool,
            crate::store::months::MM_MONTH_NOW_KEY,
        )
        .await?,
        saved: crate::store::meta::meta_f64(pool, crate::store::months::MM_SAVED_KEY).await?,
        coming_save_override: crate::store::meta::meta_f64(
            pool,
            crate::store::months::MM_COMING_SAVE_KEY,
        )
        .await?,
        target_months: crate::store::meta::meta_f64(
            pool,
            crate::store::months::MM_TARGET_MONTHS_KEY,
        )
        .await?,
        target_amount: crate::store::meta::meta_f64(
            pool,
            crate::store::months::MM_TARGET_AMOUNT_KEY,
        )
        .await?,
        salary,
        today: today(),
    });

    // J10:M20 + N13:P16 目標 — the milestone amounts are stored settings;
    // everything else derives. N4 = the latest completed year's
    // income_value_rate from the year-review rows.
    let running = month_running_averages(
        &stat_rows,
        &items,
        super::months::live_tail(&stat_rows, Some(totals)),
    );
    let n4_year = today().year() - 1;
    let n4 = review
        .years
        .iter()
        .find(|row| row.year == n4_year)
        .and_then(|row| row.assets.income_value_rate.map(|rate| (row.year, rate)));
    let (goal3_inflation, goal3_years, final_replacement) =
        super::months::goal_constants(pool).await?;
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
        goal1_amount: crate::store::meta::meta_f64(pool, crate::store::months::GOAL1_AMOUNT_KEY)
            .await?,
        goal2_amount: crate::store::meta::meta_f64(pool, crate::store::months::GOAL2_AMOUNT_KEY)
            .await?,
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
    }
}
