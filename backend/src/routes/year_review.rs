use axum::Json;
use axum::extract::{Path, State};

use super::AppState;
use crate::calc::YearReviewRow;
use crate::error::{ApiError, ErrorBody};
use crate::models::YearReviewPatch;
use crate::service::year_review::{YearReviewResponse, build};

/// Every figure here derives from stored data on each read except the manual
/// inputs and seeded overrides in `year_review`.
#[utoipa::path(
    get,
    path = "/api/year-review",
    tag = "year-review",
    responses(
        (status = 200, description = "Per-year ledger/investment/asset review rows", body = YearReviewResponse),
    )
)]
pub async fn list(State(state): State<AppState>) -> Result<Json<YearReviewResponse>, ApiError> {
    Ok(Json(build(&state.pool).await?))
}

/// Set or clear the year's stored manual figures. `sold_pl` writes the HK
/// `year_snapshots` row; the rest upsert `year_review`. A field left absent
/// stays as-is; `null` clears it back to live derivation; clearing every
/// stored value removes the row.
#[utoipa::path(
    patch,
    path = "/api/year-review/{year}",
    tag = "year-review",
    params(("year" = i32, Path, description = "Review year")),
    request_body = YearReviewPatch,
    responses(
        (status = 200, description = "The year's updated review row", body = YearReviewRow),
        (status = 400, description = "Invalid year or figure", body = ErrorBody),
    )
)]
pub async fn update(
    State(state): State<AppState>,
    Path(year): Path<i32>,
    Json(patch): Json<YearReviewPatch>,
) -> Result<Json<YearReviewRow>, ApiError> {
    if !(2000..=2100).contains(&year) {
        return Err(ApiError::field("year", "year must be a plausible year"));
    }
    if patch.income.is_none()
        && patch.invested_adjustment.is_none()
        && patch.raise.is_none()
        && patch.sold_pl.is_none()
        && patch.bond_principal.is_none()
        && patch.bond_interest.is_none()
        && patch.deposit_principal.is_none()
        && patch.deposit_interest.is_none()
    {
        return Err(ApiError::field("patch", "at least one field is required"));
    }

    crate::service::year_review::update(&state.pool, year, &patch).await?;

    let response = build(&state.pool).await?;
    let row = response
        .years
        .into_iter()
        .find(|row| row.year == year)
        .unwrap_or(YearReviewRow {
            year,
            ledger: Default::default(),
            investment: Default::default(),
            assets: Default::default(),
            record: None,
        });
    Ok(Json(row))
}
