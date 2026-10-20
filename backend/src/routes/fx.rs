use axum::Json;
use axum::extract::{Path, State};

use super::AppState;
use crate::calc::FieldError;
use crate::error::{ApiError, ErrorBody};
use crate::models::{Currency, FxRate, FxRateEntry, FxRatePatch};
use crate::store::{fx, now_timestamp};

#[utoipa::path(
    get,
    path = "/api/fx-rates",
    tag = "fx",
    responses(
        (status = 200, description = "Stored exchange rates to HKD", body = [FxRate]),
    )
)]
pub async fn list(State(state): State<AppState>) -> Result<Json<Vec<FxRate>>, ApiError> {
    Ok(Json(fx::list(&state.pool).await?))
}

/// Only `USD` exists in `fx_rates`; a number stores the rate and `null`
/// clears the row so every HKD-priced total drops out of the responses.
#[utoipa::path(
    patch,
    path = "/api/fx-rates/{currency}",
    tag = "fx",
    params(("currency" = String, Path, description = "Rate currency; only USD is supported")),
    request_body = FxRatePatch,
    responses(
        (status = 200, description = "The stored or cleared rate", body = FxRateEntry),
        (status = 400, description = "Unsupported currency or invalid rate", body = ErrorBody),
    )
)]
pub async fn update(
    State(state): State<AppState>,
    Path(currency): Path<String>,
    Json(patch): Json<FxRatePatch>,
) -> Result<Json<FxRateEntry>, ApiError> {
    if currency != "USD" {
        return Err(ApiError::Validation(vec![FieldError::new(
            "currency",
            "only USD is supported",
        )]));
    }
    match patch.rate {
        Some(rate) => {
            if !rate.is_finite() || rate <= 0.0 {
                return Err(ApiError::Validation(vec![FieldError::new(
                    "rate",
                    "rate must be greater than 0",
                )]));
            }
            let now = now_timestamp();
            fx::set(&state.pool, Currency::Usd, rate, &now).await?;
        }
        None => fx::clear(&state.pool, Currency::Usd).await?,
    }
    let entry = match fx::list(&state.pool).await?.into_iter().next() {
        Some(row) => FxRateEntry {
            currency: row.currency,
            rate_to_hkd: Some(row.rate_to_hkd),
            updated_at: Some(row.updated_at),
        },
        None => FxRateEntry {
            currency: Currency::Usd,
            rate_to_hkd: None,
            updated_at: None,
        },
    };
    Ok(Json(entry))
}
