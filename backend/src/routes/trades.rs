use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde::Deserialize;

use super::{AppState, parse_market, resolve_stock};
use crate::calc::{TradeInput, validate_trade};
use crate::error::{ApiError, ErrorBody};
use crate::models::{InputMode, NewTrade, Trade, TradePatch};
use crate::store::trades as store;

#[derive(Debug, Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
pub struct ListQuery {
    pub market: Option<String>,
    pub stock_id: Option<i64>,
    pub code: Option<String>,
    /// Inclusive lower bound on 日期 (YYYY-MM-DD).
    pub from: Option<String>,
    /// Inclusive upper bound on 日期 (YYYY-MM-DD).
    pub to: Option<String>,
    /// `asc` (default) or `desc`, by 日期.
    pub order: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/trades",
    tag = "trades",
    params(ListQuery),
    responses(
        (status = 200, description = "List trades with optional filters", body = [Trade]),
        (status = 400, description = "Invalid market", body = ErrorBody),
    )
)]
pub async fn list(
    State(state): State<AppState>,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<Trade>>, ApiError> {
    let market = query.market.as_deref().map(parse_market).transpose()?;
    let descending =
        matches!(query.order.as_deref(), Some(order) if order.eq_ignore_ascii_case("desc"));
    store::list(
        &state.pool,
        market,
        query.stock_id,
        query.code.as_deref(),
        query.from.as_deref(),
        query.to.as_deref(),
        descending,
    )
    .await
    .map(Json)
    .map_err(ApiError::from)
}

#[utoipa::path(
    post,
    path = "/api/trades",
    tag = "trades",
    request_body = NewTrade,
    responses(
        (status = 201, description = "Trade created", body = Trade),
        (status = 400, description = "Missing or invalid fields", body = ErrorBody),
        (status = 404, description = "Stock not found", body = ErrorBody),
    )
)]
pub async fn create(
    State(state): State<AppState>,
    Json(body): Json<NewTrade>,
) -> Result<(StatusCode, Json<Trade>), ApiError> {
    let (stock_id, market) = resolve_stock(
        &state.pool,
        body.stock_id,
        body.market,
        body.code.as_deref(),
    )
    .await?;

    let input_mode = body.input_mode.unwrap_or(market.default_input_mode());
    let validated = validate_trade(TradeInput {
        trade_type: &body.trade_type,
        trade_date: &body.trade_date,
        shares: body.shares,
        unit_price: body.unit_price,
        total: body.total,
        fee: body.fee,
        input_mode,
        note: body.note.as_deref(),
    })
    .map_err(ApiError::Validation)?;

    let id = store::insert_trade(&state.pool, stock_id, &validated, body.note.as_deref()).await?;
    Ok((
        StatusCode::CREATED,
        Json(store::load_one(&state.pool, id).await?),
    ))
}

#[utoipa::path(
    patch,
    path = "/api/trades/{id}",
    tag = "trades",
    params(("id" = i64, Path, description = "Trade id")),
    request_body = TradePatch,
    responses(
        (status = 200, description = "Updated trade", body = Trade),
        (status = 400, description = "Missing or invalid fields", body = ErrorBody),
        (status = 404, description = "Trade not found", body = ErrorBody),
    )
)]
pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(patch): Json<TradePatch>,
) -> Result<Json<Trade>, ApiError> {
    let existing = store::load_one(&state.pool, id).await?;

    let (stock_id, market) = match patch.stock_id {
        Some(stock_id) => resolve_stock(&state.pool, Some(stock_id), None, None).await?,
        None => (existing.stock_id, existing.market),
    };

    let input_mode = patch.input_mode.unwrap_or(if market == existing.market {
        existing.input_mode
    } else {
        market.default_input_mode()
    });
    let note = match patch.note {
        Some(note) => note
            .map(|note| note.trim().to_string())
            .filter(|note| !note.is_empty()),
        None => existing.note.clone(),
    };

    // Re-derive from whichever money figure the mode treats as authoritative,
    // falling back to the stored value when the patch does not carry it.
    let (total, fee) = match input_mode {
        InputMode::HkTotal => (Some(patch.total.unwrap_or(existing.total)), None),
        InputMode::UsFee => (None, Some(patch.fee.unwrap_or(existing.fee))),
    };

    let trade_type = patch
        .trade_type
        .unwrap_or_else(|| existing.trade_type.as_str().to_string());
    let trade_date = patch.trade_date.unwrap_or(existing.trade_date);

    let validated = validate_trade(TradeInput {
        trade_type: &trade_type,
        trade_date: &trade_date,
        shares: patch.shares.unwrap_or(existing.shares),
        unit_price: patch.unit_price.unwrap_or(existing.unit_price),
        total,
        fee,
        input_mode,
        note: note.as_deref(),
    })
    .map_err(ApiError::Validation)?;

    store::update_trade(&state.pool, id, stock_id, &validated, note).await?;

    Ok(Json(store::load_one(&state.pool, id).await?))
}

#[utoipa::path(
    delete,
    path = "/api/trades/{id}",
    tag = "trades",
    params(("id" = i64, Path, description = "Trade id")),
    responses(
        (status = 204, description = "Trade deleted"),
        (status = 404, description = "Trade not found", body = ErrorBody),
    )
)]
pub async fn remove(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    if store::delete_trade(&state.pool, id).await? == 0 {
        return Err(ApiError::NotFound(format!("trade {id} not found")));
    }
    Ok(StatusCode::NO_CONTENT)
}
