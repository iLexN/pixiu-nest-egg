use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;

use super::AppState;
use crate::error::{ApiError, ErrorBody};
use crate::models::{IbkrBlock, IbkrPatch, IbkrTransfer, NewIbkrTransfer, OverviewResponse};

#[utoipa::path(
    operation_id = "overview_ibkr",
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
    operation_id = "overview_update_ibkr",
    patch,
    path = "/api/ibkr",
    tag = "overview",
    request_body = IbkrPatch,
    responses(
        (status = 200, description = "Updated IBKR block", body = IbkrBlock),
        (status = 400, description = "Missing or invalid fields", body = ErrorBody),
    )
)]
pub async fn update_ibkr(
    State(state): State<AppState>,
    Json(patch): Json<IbkrPatch>,
) -> Result<Json<IbkrBlock>, ApiError> {
    crate::service::overview::update_ibkr(&state.pool, &patch)
        .await
        .map(Json)
        .map_err(ApiError::from)
}

/// Every `ibkr_transfers` row, newest `transfer_date` then newest id first,
/// with the source account label joined in (NULL for log-only rows).
#[utoipa::path(
    operation_id = "overview_list_ibkr_transfers",
    get,
    path = "/api/ibkr/transfers",
    tag = "overview",
    responses(
        (status = 200, description = "IBKR transfer log, newest first", body = Vec<IbkrTransfer>),
    )
)]
pub async fn list_ibkr_transfers(
    State(state): State<AppState>,
) -> Result<Json<Vec<IbkrTransfer>>, ApiError> {
    crate::store::ibkr::list_transfers(&state.pool)
        .await
        .map(Json)
        .map_err(ApiError::from)
}

/// A bank→IBKR transfer as one atomic cash movement: log row, source debit,
/// broker credit, and the month's `ibkr:<id>` adjustment item.
#[utoipa::path(
    operation_id = "overview_create_ibkr_transfer",
    post,
    path = "/api/ibkr/transfers",
    tag = "overview",
    request_body = NewIbkrTransfer,
    responses(
        (status = 201, description = "Transfer recorded and cash moved", body = IbkrTransfer),
        (status = 400, description = "Invalid amount/date/source, or insufficient balance", body = ErrorBody),
        (status = 404, description = "Source account not found", body = ErrorBody),
        (status = 409, description = "No IBKR HKD broker account", body = ErrorBody),
    )
)]
pub async fn create_ibkr_transfer(
    State(state): State<AppState>,
    Json(body): Json<NewIbkrTransfer>,
) -> Result<(StatusCode, Json<IbkrTransfer>), ApiError> {
    let transfer = crate::service::ibkr::create_transfer(&state.pool, &body).await?;
    Ok((StatusCode::CREATED, Json(transfer)))
}

/// Undo a transfer exactly: reverse the recorded balance moves, drop its
/// `ibkr:<id>` month item, then delete the row. Log-only rows move no cash.
#[utoipa::path(
    operation_id = "overview_delete_ibkr_transfer",
    delete,
    path = "/api/ibkr/transfers/{id}",
    tag = "overview",
    params(("id" = i64, Path, description = "IBKR transfer id")),
    responses(
        (status = 204, description = "Transfer deleted and its cash movement reversed"),
        (status = 404, description = "Transfer not found", body = ErrorBody),
    )
)]
pub async fn delete_ibkr_transfer(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    crate::service::ibkr::delete_transfer(&state.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `Overview!A3:C18` plus the B1/H1/J1 headline, derived on read.
#[utoipa::path(
    operation_id = "overview_overview",
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
    use crate::models::Currency;
    use crate::service::overview::ibkr_block;

    #[tokio::test]
    async fn overview_uses_the_configured_semi_liquid_target() {
        let pool = connect_memory().await.expect("memory db");
        let mut conn = pool.acquire().await.unwrap();
        crate::store::settings::SettingsPatch {
            semi_liquid_target: Some(Some(0.3)),
            ..Default::default()
        }
        .apply(&mut conn)
        .await
        .unwrap();
        drop(conn);

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
        let mut conn = pool.acquire().await.unwrap();
        crate::store::settings::SettingsPatch {
            goal1_amount: Some(Some(1550000.0)),
            goal3_years: Some(Some(25.0)),
            ..Default::default()
        }
        .apply(&mut conn)
        .await
        .unwrap();
        drop(conn);

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
        let mut conn = pool.acquire().await.unwrap();
        crate::store::settings::SettingsPatch {
            ibkr_reported_value: Some(Some(134232.01)),
            ..Default::default()
        }
        .apply(&mut conn)
        .await
        .unwrap();
        drop(conn);
        seed_broker(&pool, Currency::Hkd, 765.419).await;
        seed_broker(&pool, Currency::Usd, 1200.25).await;
        crate::store::fx::set(&pool, Currency::Usd, 7.845135, "2026-01-05T00:00:00")
            .await
            .unwrap();

        let block = ibkr_block(&pool).await.unwrap();
        // (0 stocks + 1200.25) × 7.845135 + 765.419
        assert!((block.computed_total_hkd.unwrap() - 10181.54).abs() < 0.01);
        assert!((block.net.unwrap() - 3232.01).abs() < 1e-6);
        assert!((block.net_pct.unwrap() - 0.0246718).abs() < 1e-6);
        assert!(block.vs_now_value.unwrap() < 0.0);
    }

    /// A broker cash_accounts row set to `amount` — the group-2 migration
    /// seeds them on every DB, so `set_amount` on the broker row suffices.
    async fn seed_broker(pool: &sqlx::SqlitePool, currency: Currency, amount: f64) {
        let account = crate::store::cash_accounts::broker(pool, currency)
            .await
            .unwrap()
            .expect("broker account");
        let mut conn = pool.acquire().await.unwrap();
        crate::store::cash_accounts::set_amount(
            &mut conn,
            account.id,
            amount,
            "2026-01-05T00:00:00",
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn ibkr_block_is_rate_dependent() {
        let pool = connect_memory().await.expect("memory db");
        seed_broker(&pool, Currency::Hkd, 765.42).await;
        let block = ibkr_block(&pool).await.unwrap();
        assert!(block.computed_total_hkd.is_none());
        assert!(block.net.is_none());
    }

    /// A PATCH touches only the three account fields — it never appends a
    /// transfer row nor moves a bank balance.
    #[tokio::test]
    async fn update_ibkr_appends_no_transfer() {
        let pool = connect_memory().await.expect("memory db");
        let state = AppState { pool: pool.clone() };
        let _ = update_ibkr(
            State(state),
            Json(IbkrPatch {
                now_value: Some(Some(134232.01)),
                hkd_cash: Some(Some(765.42)),
                usd_cash: Some(Some(1200.25)),
            }),
        )
        .await
        .unwrap();
        let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM ibkr_transfers")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(rows, 0);
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

        seed_broker(&pool, Currency::Hkd, 100.0).await;
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
            crate::store::cash_accounts::broker(&pool, Currency::Hkd)
                .await
                .unwrap()
                .map(|account| account.amount),
            Some(0.0)
        );
    }

    #[tokio::test]
    async fn update_ibkr_sets_the_usd_broker_account() {
        let pool = connect_memory().await.expect("memory db");
        let state = AppState { pool: pool.clone() };
        let block = update_ibkr(
            State(state),
            Json(IbkrPatch {
                usd_cash: Some(Some(1200.25)),
                ..Default::default()
            }),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(block.usd_cash, Some(1200.25));
        assert_eq!(
            crate::store::cash_accounts::broker(&pool, Currency::Usd)
                .await
                .unwrap()
                .map(|account| account.amount),
            Some(1200.25)
        );
    }
}
