use axum::extract::State;
use axum::Json;
use sqlx::SqlitePool;

use super::{aia, months, mpf, summary, today, AppState};
use crate::calc::{live_totals, trailing_averages, FieldError};
use crate::error::ApiError;
use crate::models::{
    IbkrBlock, IbkrPatch, ManualAsset, ManualAssetKind, OverviewAssetRow, OverviewResponse,
    SemiLiquid, TwelveMonthAverages,
};

/// `app_meta` keys holding the 美股 sheet's IBKR account block (A1:B5).
pub const IBKR_TRANSFERRED_KEY: &str = "ibkr.transferred_hkd";
pub const IBKR_NOW_VALUE_KEY: &str = "ibkr.now_value";
pub const IBKR_HKD_CASH_KEY: &str = "ibkr.hkd_cash";
pub const IBKR_USD_CASH_KEY: &str = "ibkr.usd_cash";

/// The IBKR block: stored inputs plus the derived 美股!B7/C1/C2 figures.
pub async fn ibkr_block(pool: &SqlitePool) -> Result<IbkrBlock, ApiError> {
    let transferred_hkd = mpf::meta_f64(pool, IBKR_TRANSFERRED_KEY).await?;
    let now_value = mpf::meta_f64(pool, IBKR_NOW_VALUE_KEY).await?;
    let hkd_cash = mpf::meta_f64(pool, IBKR_HKD_CASH_KEY).await?;
    let usd_cash = mpf::meta_f64(pool, IBKR_USD_CASH_KEY).await?;
    let rate = mpf::meta_f64(pool, aia::RATE_KEY).await?;
    let stock_value_usd = summary::market_value_only(pool, crate::models::Market::Us).await?;

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

pub async fn ibkr(State(state): State<AppState>) -> Result<Json<IbkrBlock>, ApiError> {
    ibkr_block(&state.pool).await.map(Json)
}

pub async fn update_ibkr(
    State(state): State<AppState>,
    Json(patch): Json<IbkrPatch>,
) -> Result<Json<IbkrBlock>, ApiError> {
    let mut errors = Vec::new();
    for (field, value) in [
        ("transferred_hkd", patch.transferred_hkd),
        ("now_value", patch.now_value),
        ("hkd_cash", patch.hkd_cash),
        ("usd_cash", patch.usd_cash),
    ] {
        if let Some(Some(value)) = value {
            if !value.is_finite() || value < 0.0 {
                errors.push(FieldError::new(field, "value must not be negative"));
            }
        }
    }
    if !errors.is_empty() {
        return Err(ApiError::Validation(errors));
    }
    for (key, value) in [
        (IBKR_TRANSFERRED_KEY, patch.transferred_hkd),
        (IBKR_NOW_VALUE_KEY, patch.now_value),
        (IBKR_HKD_CASH_KEY, patch.hkd_cash),
        (IBKR_USD_CASH_KEY, patch.usd_cash),
    ] {
        if let Some(value) = value {
            crate::mpf::meta_put(
                &state.pool,
                key,
                value.map(|value| value.to_string()).as_deref(),
            )
            .await?;
        }
    }
    ibkr_block(&state.pool).await.map(Json)
}

/// `Overview!A3:C18` plus the B1/H1/J1 headline, derived on read.
pub async fn overview(State(state): State<AppState>) -> Result<Json<OverviewResponse>, ApiError> {
    let input = months::live_totals_input(&state.pool).await?;
    let totals = live_totals(&input);
    let ibkr = ibkr_block(&state.pool).await?;
    let salary = mpf::meta_f64(&state.pool, months::SALARY_KEY).await?;
    let manual = months::load_assets(&state.pool).await?;
    let items = months::load_all_items(&state.pool).await?;

    let aia_hkd = input.usd_hkd_rate.map(|rate| input.aia_value_usd * rate);
    let mut assets = vec![
        asset_row("hk", "港股", Some(input.hk_market_value)),
        asset_row("bonds", "債券", Some(input.bonds_active_principal)),
        asset_row("aia", "基金", aia_hkd),
        asset_row("mpf", "MPF", Some(input.mpf_balance)),
    ];
    for asset in manual
        .iter()
        .filter(|asset| asset.kind == ManualAssetKind::Asset)
    {
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
        vs_quarter_liquid: semi_total - 0.25 * totals.liquid_assets,
        // A13 = B14 ÷ (港股 + 債券 + 半流動資金 + IBKR).
        share: ibkr.computed_total_hkd.and_then(|ibkr_total| {
            let divisor =
                input.hk_market_value + input.bonds_active_principal + semi_total + ibkr_total;
            (divisor != 0.0).then_some(semi_total / divisor)
        }),
    };

    // F3:G10 + H6 — live-filled totals feed the window's edge (e.g. the last
    // completed month's Changed needs the live current row's 總數).
    let stat_rows = months::load_stat_rows(&state.pool, Some(&totals)).await?;
    let trailing = trailing_averages(&stat_rows, &items, months::live_from());
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

    Ok(Json(OverviewResponse {
        today: today().to_string(),
        rate: input.usd_hkd_rate,
        salary,
        total_assets: totals.total_assets,
        liquid_assets: totals.liquid_assets,
        liquid_ratio: salary.map(|salary| totals.liquid_assets / (salary * 100.0)),
        assets,
        assets_sum,
        semi_liquid,
        ibkr,
        averages,
    }))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connect_memory;

    #[tokio::test]
    async fn ibkr_block_derives_the_cross_checks() {
        let pool = connect_memory().await.expect("memory db");
        crate::mpf::meta_put(&pool, IBKR_TRANSFERRED_KEY, Some("131000"))
            .await
            .unwrap();
        crate::mpf::meta_put(&pool, IBKR_NOW_VALUE_KEY, Some("134232.01"))
            .await
            .unwrap();
        crate::mpf::meta_put(&pool, IBKR_HKD_CASH_KEY, Some("765.419"))
            .await
            .unwrap();
        crate::mpf::meta_put(&pool, IBKR_USD_CASH_KEY, Some("1200.25"))
            .await
            .unwrap();
        crate::mpf::meta_put(&pool, aia::RATE_KEY, Some("7.845135"))
            .await
            .unwrap();

        let block = ibkr_block(&pool).await.unwrap();
        // (0 stocks + 1200.25) × 7.845135 + 765.419
        assert!((block.computed_total_hkd.unwrap() - 10181.54).abs() < 0.01);
        assert!((block.net.unwrap() - 3232.01).abs() < 1e-6);
        assert!((block.net_pct.unwrap() - 0.0246718).abs() < 1e-6);
        assert!(block.vs_now_value.unwrap() < 0.0);
    }

    #[tokio::test]
    async fn ibkr_block_is_rate_dependent() {
        let pool = connect_memory().await.expect("memory db");
        crate::mpf::meta_put(&pool, IBKR_HKD_CASH_KEY, Some("765.42"))
            .await
            .unwrap();
        let block = ibkr_block(&pool).await.unwrap();
        assert!(block.computed_total_hkd.is_none());
        assert!(block.net.is_none());
    }

    #[tokio::test]
    async fn update_ibkr_validates_and_clears() {
        let pool = connect_memory().await.expect("memory db");
        let state = AppState { pool: pool.clone() };
        let err = update_ibkr(
            State(state.clone()),
            Json(IbkrPatch {
                usd_cash: Some(Some(-1.0)),
                ..Default::default()
            }),
        )
        .await
        .expect_err("negative rejected");
        assert!(matches!(err, ApiError::Validation(_)));

        crate::mpf::meta_put(&pool, IBKR_HKD_CASH_KEY, Some("100"))
            .await
            .unwrap();
        let _ = update_ibkr(
            State(state),
            Json(IbkrPatch {
                hkd_cash: Some(None),
                ..Default::default()
            }),
        )
        .await
        .unwrap();
        assert_eq!(
            crate::mpf::meta_get(&pool, IBKR_HKD_CASH_KEY)
                .await
                .unwrap(),
            None
        );
    }
}
