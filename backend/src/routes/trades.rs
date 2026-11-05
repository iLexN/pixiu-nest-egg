use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde::Deserialize;

use super::{AppState, parse_market, resolve_stock};
use crate::calc::{FieldError, TradeInput, validate_trade};
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
    operation_id = "trades_list",
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
    operation_id = "trades_create",
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

    let id = crate::service::trades::create(
        &state.pool,
        stock_id,
        market,
        &validated,
        body.note.as_deref(),
    )
    .await?;
    crate::service::summary::record_market(&state.pool, market).await?;
    Ok((
        StatusCode::CREATED,
        Json(store::load_one(&state.pool, id).await?),
    ))
}

#[utoipa::path(
    operation_id = "trades_update",
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
    // US trades settle inside IBKR and HK trades at the HS bank, so a
    // trade's market is fixed at creation — wrong market means recreate it.
    if market != existing.market {
        return Err(ApiError::Validation(vec![FieldError::new(
            "stock_id",
            "cannot change a trade's market",
        )]));
    }

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

    crate::service::trades::update(&state.pool, id, stock_id, market, &validated, note).await?;
    crate::service::summary::record_market(&state.pool, existing.market).await?;

    Ok(Json(store::load_one(&state.pool, id).await?))
}

#[utoipa::path(
    operation_id = "trades_remove",
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
    let market = store::load_one(&state.pool, id).await?.market;
    if !crate::service::trades::remove(&state.pool, id).await? {
        return Err(ApiError::NotFound(format!("trade {id} not found")));
    }
    crate::service::summary::record_market(&state.pool, market).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connect_memory;
    use sqlx::SqlitePool;

    async fn seed_stock(pool: &SqlitePool, market: &str, code: &str) -> i64 {
        sqlx::query_scalar("INSERT INTO stocks (market, code) VALUES (?, ?) RETURNING id")
            .bind(market)
            .bind(code)
            .fetch_one(pool)
            .await
            .unwrap()
    }

    async fn seed_trade(pool: &SqlitePool, stock_id: i64) -> i64 {
        sqlx::query_scalar(
            "INSERT INTO trades (stock_id, trade_type, trade_date, shares, unit_price, fee, \
             total, input_mode, created_at, updated_at) \
             VALUES (?, 'BUY', '2026-11-05', 3, 696.04, 1.0, 2089.12, 'US_FEE', \
             '2026-01-01T00:00:00', '2026-01-01T00:00:00') RETURNING id",
        )
        .bind(stock_id)
        .fetch_one(pool)
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn update_rejects_a_stock_from_the_other_market() {
        let pool = connect_memory().await.expect("memory db");
        let state = AppState { pool: pool.clone() };
        let voo = seed_stock(&pool, "US", "VOO").await;
        let hsbc = seed_stock(&pool, "HK", "0005").await;
        let us_trade = seed_trade(&pool, voo).await;
        let hk_trade = seed_trade(&pool, hsbc).await;

        for (trade, wrong_stock) in [(us_trade, hsbc), (hk_trade, voo)] {
            let err = update(
                State(state.clone()),
                Path(trade),
                Json(TradePatch {
                    stock_id: Some(wrong_stock),
                    ..Default::default()
                }),
            )
            .await
            .expect_err("cross-market repoint");
            let ApiError::Validation(fields) = err else {
                panic!("expected validation");
            };
            assert_eq!(fields[0].field, "stock_id");
        }
    }

    #[tokio::test]
    async fn update_allows_a_same_market_repoint() {
        let pool = connect_memory().await.expect("memory db");
        let state = AppState { pool: pool.clone() };
        let voo = seed_stock(&pool, "US", "VOO").await;
        let qqq = seed_stock(&pool, "US", "QQQ").await;
        let trade = seed_trade(&pool, voo).await;

        let updated = update(
            State(state),
            Path(trade),
            Json(TradePatch {
                stock_id: Some(qqq),
                ..Default::default()
            }),
        )
        .await
        .unwrap()
        .0;

        assert_eq!(updated.stock_id, qqq);
    }
}
