use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;

use super::AppState;
use crate::error::{ApiError, ErrorBody};
use crate::models::{CashAccount, CashAccountPatch, NewCashAccount};
use crate::service::cash_accounts;
use crate::store::cash_accounts as store;

#[utoipa::path(
    get,
    path = "/api/cash-accounts",
    tag = "cash-accounts",
    responses(
        (status = 200, description = "Bank and broker cash accounts", body = [CashAccount]),
    )
)]
pub async fn list(State(state): State<AppState>) -> Result<Json<Vec<CashAccount>>, ApiError> {
    Ok(Json(store::load_all(&state.pool).await?))
}

#[utoipa::path(
    post,
    path = "/api/cash-accounts",
    tag = "cash-accounts",
    request_body = NewCashAccount,
    responses(
        (status = 201, description = "Cash account created", body = CashAccount),
        (status = 400, description = "Missing or invalid fields", body = ErrorBody),
        (status = 409, description = "Duplicate label or a second broker for the currency", body = ErrorBody),
    )
)]
pub async fn create(
    State(state): State<AppState>,
    Json(body): Json<NewCashAccount>,
) -> Result<(StatusCode, Json<CashAccount>), ApiError> {
    let account = cash_accounts::create(&state.pool, &body).await?;
    Ok((StatusCode::CREATED, Json(account)))
}

#[utoipa::path(
    patch,
    path = "/api/cash-accounts/{id}",
    tag = "cash-accounts",
    params(("id" = i64, Path, description = "Cash account id")),
    request_body = CashAccountPatch,
    responses(
        (status = 200, description = "Updated cash account", body = CashAccount),
        (status = 400, description = "Invalid fields", body = ErrorBody),
        (status = 404, description = "Cash account not found", body = ErrorBody),
        (status = 409, description = "Duplicate label or a second broker for the currency", body = ErrorBody),
    )
)]
pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(patch): Json<CashAccountPatch>,
) -> Result<Json<CashAccount>, ApiError> {
    Ok(Json(cash_accounts::update(&state.pool, id, &patch).await?))
}

#[utoipa::path(
    delete,
    path = "/api/cash-accounts/{id}",
    tag = "cash-accounts",
    params(("id" = i64, Path, description = "Cash account id")),
    responses(
        (status = 204, description = "Cash account deleted"),
        (status = 404, description = "Cash account not found", body = ErrorBody),
        (status = 409, description = "Broker accounts cannot be deleted", body = ErrorBody),
    )
)]
pub async fn remove(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    cash_accounts::remove(&state.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
