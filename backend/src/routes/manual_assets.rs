use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;

use super::AppState;
use crate::error::{ApiError, ErrorBody};
use crate::models::{ManualAsset, ManualAssetPatch, NewManualAsset};
use crate::store::manual_assets as store;

#[utoipa::path(
    operation_id = "manual_assets_list",
    get,
    path = "/api/manual-assets",
    tag = "manual-assets",
    responses(
        (status = 200, description = "List manual asset rows", body = [ManualAsset]),
    )
)]
pub async fn list(State(state): State<AppState>) -> Result<Json<Vec<ManualAsset>>, ApiError> {
    store::load_all(&state.pool)
        .await
        .map(Json)
        .map_err(ApiError::from)
}

#[utoipa::path(
    operation_id = "manual_assets_create",
    post,
    path = "/api/manual-assets",
    tag = "manual-assets",
    request_body = NewManualAsset,
    responses(
        (status = 201, description = "Manual asset created", body = ManualAsset),
        (status = 400, description = "Missing or invalid fields", body = ErrorBody),
    )
)]
pub async fn create(
    State(state): State<AppState>,
    Json(body): Json<NewManualAsset>,
) -> Result<(StatusCode, Json<ManualAsset>), ApiError> {
    let asset = crate::service::manual_assets::create(&state.pool, &body).await?;
    Ok((StatusCode::CREATED, Json(asset)))
}

#[utoipa::path(
    operation_id = "manual_assets_update",
    patch,
    path = "/api/manual-assets/{id}",
    tag = "manual-assets",
    params(("id" = i64, Path, description = "Manual asset id")),
    request_body = ManualAssetPatch,
    responses(
        (status = 200, description = "Updated manual asset", body = ManualAsset),
        (status = 400, description = "Missing or invalid fields", body = ErrorBody),
        (status = 404, description = "Manual asset not found", body = ErrorBody),
    )
)]
pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(patch): Json<ManualAssetPatch>,
) -> Result<Json<ManualAsset>, ApiError> {
    Ok(Json(
        crate::service::manual_assets::update(&state.pool, id, &patch).await?,
    ))
}

#[utoipa::path(
    operation_id = "manual_assets_remove",
    delete,
    path = "/api/manual-assets/{id}",
    tag = "manual-assets",
    params(("id" = i64, Path, description = "Manual asset id")),
    responses(
        (status = 204, description = "Manual asset deleted"),
        (status = 404, description = "Manual asset not found", body = ErrorBody),
    )
)]
pub async fn remove(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    crate::service::manual_assets::remove(&state.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connect_memory;

    #[tokio::test]
    async fn unknown_liquidity_is_rejected_with_a_client_error() {
        let pool = connect_memory().await.expect("memory db");
        let (router, _api) = crate::routes::api_router(AppState { pool });
        let response = tower::ServiceExt::oneshot(
            router,
            axum::http::Request::builder()
                .method("POST")
                .uri("/api/manual-assets")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(
                    r#"{"label":"X","kind":"asset","amount":1,"liquidity":"medium"}"#,
                ))
                .unwrap(),
        )
        .await
        .expect("response");
        assert!(
            response.status().is_client_error(),
            "expected 4xx, got {}",
            response.status()
        );
    }
}
