use axum::Json;
use axum::extract::{Query, State};
use serde::Deserialize;

use super::{AppState, parse_market};
use crate::error::{ApiError, ErrorBody};

pub use crate::service::summary::{AVERAGE_PRICE_DEFINITION, StockRow, SummaryResponse};

#[derive(Debug, Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
pub struct SummaryQuery {
    pub market: String,
}

#[utoipa::path(
    get,
    path = "/api/summary",
    tag = "summary",
    params(SummaryQuery),
    responses(
        (status = 200, description = "Per-stock summary, sector rollup, and market totals", body = SummaryResponse),
        (status = 400, description = "Invalid market", body = ErrorBody),
    )
)]
pub async fn show(
    State(state): State<AppState>,
    Query(query): Query<SummaryQuery>,
) -> Result<Json<SummaryResponse>, ApiError> {
    let market = parse_market(&query.market)?;
    Ok(Json(
        crate::service::summary::build(&state.pool, market).await?,
    ))
}
