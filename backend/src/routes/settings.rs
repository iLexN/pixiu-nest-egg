use axum::Json;
use axum::extract::State;

use super::AppState;
use crate::error::{ApiError, ErrorBody};
use crate::models::{AppSettings, AppSettingsPatch};

#[utoipa::path(
    get,
    path = "/api/settings",
    tag = "settings",
    responses(
        (status = 200, description = "Salary and current-year pool rate", body = AppSettings),
    )
)]
pub async fn get(State(state): State<AppState>) -> Result<Json<AppSettings>, ApiError> {
    Ok(Json(crate::service::settings::get(&state.pool).await?))
}

#[utoipa::path(
    patch,
    path = "/api/settings",
    tag = "settings",
    request_body = AppSettingsPatch,
    responses(
        (status = 200, description = "Updated settings", body = AppSettings),
        (status = 400, description = "Invalid fields", body = ErrorBody),
    )
)]
pub async fn update(
    State(state): State<AppState>,
    Json(patch): Json<AppSettingsPatch>,
) -> Result<Json<AppSettings>, ApiError> {
    Ok(Json(
        crate::service::settings::update(&state.pool, &patch).await?,
    ))
}
