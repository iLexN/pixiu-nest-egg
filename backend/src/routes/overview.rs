use axum::Json;
use axum::extract::State;

use super::{AppState, today};
use crate::calc::FieldError;
use crate::error::{ApiError, ErrorBody};
use crate::models::{IbkrBlock, IbkrPatch, OverviewResponse};

#[utoipa::path(
    get,
    path = "/api/ibkr",
    tag = "overview",
    responses(
        (status = 200, description = "IBKR account block (manual cells plus derived cross-checks)", body = IbkrBlock),
    )
)]
pub async fn ibkr(State(state): State<AppState>) -> Result<Json<IbkrBlock>, ApiError> {
    crate::service::overview::ibkr_block(&state.pool)
        .await
        .map(Json)
        .map_err(ApiError::from)
}

#[utoipa::path(
    patch,
    path = "/api/ibkr",
    tag = "overview",
    request_body = IbkrPatch,
    responses(
        (status = 200, description = "Updated IBKR block; 轉入 appends a dated transfer row", body = IbkrBlock),
        (status = 400, description = "Missing or invalid fields", body = ErrorBody),
    )
)]
pub async fn update_ibkr(
    State(state): State<AppState>,
    Json(patch): Json<IbkrPatch>,
) -> Result<Json<IbkrBlock>, ApiError> {
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
    // A transfer delta may be negative — it records a withdrawal or undoes a
    // mistyped entry; it must only be finite.
    if let Some(delta) = patch.transfer_hkd
        && !delta.is_finite()
    {
        errors.push(FieldError::new("transfer_hdk", "value must be a number"));
    }
    let transfer_date = match &patch.transfer_date {
        Some(date) => match chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d") {
            Ok(date) => date,
            Err(_) => {
                errors.push(FieldError::new("transfer_date", "date must be YYYY-MM-DD"));
                today()
            }
        },
        None => today(),
    };
    if !errors.is_empty() {
        return Err(ApiError::Validation(errors));
    }
    if let Some(delta) = patch.transfer_hkd.filter(|delta| *delta != 0.0) {
        crate::store::overview::insert_transfer(&state.pool, &transfer_date.to_string(), delta)
            .await?;
    }
    for (key, value) in [
        (crate::store::overview::IBKR_NOW_VALUE_KEY, patch.now_value),
        (crate::store::overview::IBKR_HKD_CASH_KEY, patch.hkd_cash),
        (crate::store::overview::IBKR_USD_CASH_KEY, patch.usd_cash),
    ] {
        if let Some(value) = value {
            crate::store::meta::meta_put(
                &state.pool,
                key,
                value.map(|value| value.to_string()).as_deref(),
            )
            .await?;
        }
    }
    crate::service::overview::ibkr_block(&state.pool)
        .await
        .map(Json)
        .map_err(ApiError::from)
}

/// `Overview!A3:C18` plus the B1/H1/J1 headline, derived on read.
#[utoipa::path(
    get,
    path = "/api/overview",
    tag = "overview",
    responses(
        (status = 200, description = "Headline totals, asset table, 半流動資金, and IBKR block", body = OverviewResponse),
    )
)]
pub async fn overview(State(state): State<AppState>) -> Result<Json<OverviewResponse>, ApiError> {
    Ok(Json(crate::service::overview::build(&state.pool).await?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connect_memory;
    use crate::service::overview::ibkr_block;

    #[tokio::test]
    async fn overview_uses_the_configured_semi_liquid_target() {
        let pool = connect_memory().await.expect("memory db");
        crate::store::meta::meta_put(
            &pool,
            crate::store::months::SEMI_LIQUID_TARGET_KEY,
            Some("0.3"),
        )
        .await
        .unwrap();

        let response = overview(State(AppState { pool }))
            .await
            .expect("overview")
            .0;
        assert_eq!(response.semi_liquid_target, 0.3);
        let expected = response.semi_liquid.total - 0.3 * response.liquid_assets;
        assert!((response.semi_liquid.vs_quarter_liquid - expected).abs() < 1e-9);
    }

    #[tokio::test]
    async fn overview_reports_the_goals_block() {
        let pool = connect_memory().await.expect("memory db");
        crate::store::meta::meta_put(
            &pool,
            crate::store::months::GOAL1_AMOUNT_KEY,
            Some("1550000"),
        )
        .await
        .unwrap();
        crate::store::meta::meta_put(&pool, crate::store::months::GOAL3_YEARS_KEY, Some("25"))
            .await
            .unwrap();

        let goals = overview(State(AppState { pool }))
            .await
            .expect("overview")
            .0
            .goals;
        assert_eq!(goals.milestones.len(), 2);
        assert_eq!(goals.milestones[0].target, Some(1_550_000.0));
        assert_eq!(goals.milestones[1].target, None);
        // No stored data: the derived figures are absent, not zero.
        assert!(goals.current_invest.is_none());
        assert!(goals.saving_year.is_none());
        assert!(goals.milestones[0].remain.is_none());
        assert!(goals.milestones[0].eta.is_none());
        assert!(goals.third.target.is_none());
        assert!(goals.final_goal.target.is_none());
    }

    #[tokio::test]
    async fn ibkr_block_derives_the_cross_checks() {
        let pool = connect_memory().await.expect("memory db");
        sqlx::query(
            "INSERT INTO ibkr_transfers (transfer_date, amount_hkd, created_at) \
             VALUES ('2026-01-05', 131000, '2026-01-05T00:00:00')",
        )
        .execute(&pool)
        .await
        .unwrap();
        crate::store::meta::meta_put(
            &pool,
            crate::store::overview::IBKR_NOW_VALUE_KEY,
            Some("134232.01"),
        )
        .await
        .unwrap();
        crate::store::meta::meta_put(
            &pool,
            crate::store::overview::IBKR_HKD_CASH_KEY,
            Some("765.419"),
        )
        .await
        .unwrap();
        crate::store::meta::meta_put(
            &pool,
            crate::store::overview::IBKR_USD_CASH_KEY,
            Some("1200.25"),
        )
        .await
        .unwrap();
        crate::store::meta::meta_put(&pool, crate::store::aia::RATE_KEY, Some("7.845135"))
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
        crate::store::meta::meta_put(
            &pool,
            crate::store::overview::IBKR_HKD_CASH_KEY,
            Some("765.42"),
        )
        .await
        .unwrap();
        let block = ibkr_block(&pool).await.unwrap();
        assert!(block.computed_total_hkd.is_none());
        assert!(block.net.is_none());
    }

    #[tokio::test]
    async fn update_ibkr_appends_a_dated_transfer() {
        let pool = connect_memory().await.expect("memory db");
        let state = AppState { pool: pool.clone() };
        let block = update_ibkr(
            State(state.clone()),
            Json(IbkrPatch {
                transfer_hkd: Some(50000.0),
                transfer_date: Some("2026-03-01".to_string()),
                ..Default::default()
            }),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(block.transferred_hkd, Some(50000.0));

        // A negative delta records a withdrawal/correction on top.
        let block = update_ibkr(
            State(state),
            Json(IbkrPatch {
                transfer_hkd: Some(-10000.0),
                ..Default::default()
            }),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(block.transferred_hkd, Some(40000.0));

        let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM ibkr_transfers")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(rows, 2);
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

        crate::store::meta::meta_put(
            &pool,
            crate::store::overview::IBKR_HKD_CASH_KEY,
            Some("100"),
        )
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
            crate::store::meta::meta_get(&pool, crate::store::overview::IBKR_HKD_CASH_KEY)
                .await
                .unwrap(),
            None
        );
    }
}
