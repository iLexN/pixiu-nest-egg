use axum::Json;
use axum::extract::{Path, Query, State};
use chrono::Datelike;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use super::{AppState, now_timestamp, parse_market, today};
use crate::calc::{YearRow, yearly_rows};
use crate::error::{ApiError, ErrorBody};
use crate::models::{Market, YearlyPatch};
use crate::store::yearly as store;

#[derive(Debug, Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
pub struct YearlyQuery {
    pub market: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct YearlyResponse {
    pub market: Market,
    /// The date the response was generated.
    pub today: String,
    pub years: Vec<YearRow>,
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

/// Every figure here is derived from stored trades, dividends and snapshots
/// on each read.
pub async fn build(pool: &SqlitePool, market: Market) -> Result<YearlyResponse, ApiError> {
    let trades = crate::store::trades::market_dated_trade_facts(pool, market).await?;
    let dividends = crate::store::dividends::market_dividend_facts(pool, market).await?;
    let snapshots = store::load_snapshots(pool, market).await?;
    let live_market_value = crate::service::summary::build(pool, market)
        .await?
        .totals
        .market_value;

    Ok(YearlyResponse {
        market,
        today: today().to_string(),
        years: yearly_rows(
            &trades,
            &dividends,
            &snapshots,
            live_market_value,
            today().year(),
        ),
    })
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
    let merge = |patch: Option<Option<f64>>, field: &str, existing: Option<f64>| {
        validate_figure(field, patch.unwrap_or(existing))
    };
    let merge_signed = |patch: Option<Option<f64>>, field: &str, existing: Option<f64>| {
        validate_signed_figure(field, patch.unwrap_or(existing))
    };
    let (invested, cost, market_value, sold_pl) = match &existing {
        Some(existing) => (
            merge(patch.invested, "invested", existing.invested)?,
            merge(patch.cost, "cost", existing.cost)?,
            merge(patch.market_value, "market_value", existing.market_value)?,
            merge_signed(patch.sold_pl, "sold_pl", existing.sold_pl)?,
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
