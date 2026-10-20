use axum::Json;
use axum::extract::{Path, Query, State};
use serde::Deserialize;

use super::patch::merge_num;
use super::{AppState, parse_market};
use crate::error::{ApiError, ErrorBody};
use crate::models::YearlyPatch;
use crate::service::yearly::{YearlyResponse, build};
use crate::store::now_timestamp;
use crate::store::yearly as store;

#[derive(Debug, Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
pub struct YearlyQuery {
    pub market: String,
}

#[utoipa::path(
    get,
    path = "/api/summary/yearly",
    tag = "summary",
    params(YearlyQuery),
    responses(
        (status = 200, description = "Per-year rollup for one market", body = YearlyResponse),
        (status = 400, description = "Invalid market", body = ErrorBody),
    )
)]
pub async fn list(
    State(state): State<AppState>,
    Query(query): Query<YearlyQuery>,
) -> Result<Json<YearlyResponse>, ApiError> {
    let market = parse_market(&query.market)?;
    Ok(Json(build(&state.pool, market).await?))
}

fn validate_figure(field: &str, value: Option<f64>) -> Result<Option<f64>, ApiError> {
    match value {
        Some(value) if !value.is_finite() || value < 0.0 => Err(ApiError::field(
            field,
            format!("{field} must be a non-negative number"),
        )),
        other => Ok(other),
    }
}

/// sold P/L may be negative — only finiteness is enforced.
fn validate_signed_figure(field: &str, value: Option<f64>) -> Result<Option<f64>, ApiError> {
    match value {
        Some(value) if !value.is_finite() => {
            Err(ApiError::field(field, format!("{field} must be a number")))
        }
        other => Ok(other),
    }
}

#[utoipa::path(
    patch,
    path = "/api/summary/yearly/{market}/{year}",
    tag = "summary",
    params(
        ("market" = String, Path, description = "HK or US"),
        ("year" = i32, Path, description = "Snapshot year"),
    ),
    request_body = YearlyPatch,
    responses(
        (status = 200, description = "Upserted or cleared year snapshot", body = crate::models::YearSnapshot),
        (status = 400, description = "Invalid market, year, or figure", body = ErrorBody),
    )
)]
pub async fn update(
    State(state): State<AppState>,
    Path((market, year)): Path<(String, i32)>,
    Json(patch): Json<YearlyPatch>,
) -> Result<Json<crate::models::YearSnapshot>, ApiError> {
    let market = parse_market(&market)?;
    if !(2000..=2100).contains(&year) {
        return Err(ApiError::field("year", "year must be a plausible year"));
    }
    if patch.invested.is_none()
        && patch.cost.is_none()
        && patch.market_value.is_none()
        && patch.sold_pl.is_none()
    {
        return Err(ApiError::field(
            "patch",
            "at least one of invested, cost, market_value, sold_pl is required",
        ));
    }

    let existing = store::load_snapshot(&state.pool, market, year).await?;
    let (invested, cost, market_value, sold_pl) = match &existing {
        Some(existing) => (
            validate_figure("invested", merge_num(patch.invested, existing.invested))?,
            validate_figure("cost", merge_num(patch.cost, existing.cost))?,
            validate_figure(
                "market_value",
                merge_num(patch.market_value, existing.market_value),
            )?,
            validate_signed_figure("sold_pl", merge_num(patch.sold_pl, existing.sold_pl))?,
        ),
        None => (
            validate_figure("invested", patch.invested.flatten())?,
            validate_figure("cost", patch.cost.flatten())?,
            validate_figure("market_value", patch.market_value.flatten())?,
            validate_signed_figure("sold_pl", patch.sold_pl.flatten())?,
        ),
    };

    if invested.is_none() && cost.is_none() && market_value.is_none() && sold_pl.is_none() {
        // Clearing the last stored figure removes the row entirely.
        store::delete_snapshot(&state.pool, market, year).await?;
        return Ok(Json(crate::models::YearSnapshot {
            market,
            year,
            invested: None,
            cost: None,
            market_value: None,
            sold_pl: None,
            updated_at: now_timestamp(),
        }));
    }

    store::upsert_snapshot(
        &state.pool,
        market,
        year,
        invested,
        cost,
        market_value,
        sold_pl,
    )
    .await?;

    Ok(Json(
        store::load_snapshot(&state.pool, market, year)
            .await?
            .expect("just upserted"),
    ))
}

/// Freeze the year: store the currently computed cumulative 成本 and the
/// live 總市值 as the snapshot. `invested` is left untouched.
#[utoipa::path(
    post,
    path = "/api/summary/yearly/{market}/{year}/freeze",
    tag = "summary",
    params(
        ("market" = String, Path, description = "HK or US"),
        ("year" = i32, Path, description = "Snapshot year"),
    ),
    responses(
        (status = 200, description = "Snapshot frozen at computed cost and live market value", body = crate::models::YearSnapshot),
        (status = 400, description = "Invalid market or year", body = ErrorBody),
    )
)]
pub async fn freeze(
    State(state): State<AppState>,
    Path((market, year)): Path<(String, i32)>,
) -> Result<Json<crate::models::YearSnapshot>, ApiError> {
    let market = parse_market(&market)?;
    if !(2000..=2100).contains(&year) {
        return Err(ApiError::field("year", "year must be a plausible year"));
    }

    let cost = store::buy_cost_sum(&state.pool, market, year).await?;
    let market_value = crate::service::summary::build(&state.pool, market)
        .await?
        .totals
        .market_value;

    store::upsert_frozen(&state.pool, market, year, cost, market_value).await?;

    Ok(Json(
        store::load_snapshot(&state.pool, market, year)
            .await?
            .expect("just upserted"),
    ))
}
