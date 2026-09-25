use std::collections::HashSet;

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde::Deserialize;
use sqlx::SqlitePool;

use super::{AppState, STOCK_COLUMNS, now_timestamp, parse_market, row_to_stock};
use crate::error::{ApiError, ErrorBody};
use crate::models::{Market, NewStock, PriceReport, Stock, StockPatch};
use crate::prices;

#[derive(Debug, Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
pub struct ListQuery {
    pub market: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/stocks",
    tag = "stocks",
    params(ListQuery),
    responses(
        (status = 200, description = "List stocks, optionally filtered by market", body = [Stock]),
        (status = 400, description = "Invalid market", body = ErrorBody),
    )
)]
pub async fn list(
    State(state): State<AppState>,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<Stock>>, ApiError> {
    let market = query.market.as_deref().map(parse_market).transpose()?;
    Ok(Json(load_stocks(&state.pool, market).await?))
}

pub async fn load_stocks(
    pool: &SqlitePool,
    market: Option<Market>,
) -> Result<Vec<Stock>, ApiError> {
    let sql = format!(
        "SELECT {STOCK_COLUMNS} FROM stocks {} ORDER BY market, sort_order, code",
        if market.is_some() {
            "WHERE market = ?"
        } else {
            ""
        }
    );
    let mut statement = sqlx::query(sqlx::AssertSqlSafe(sql));
    if let Some(market) = market {
        statement = statement.bind(market.as_str());
    }
    statement
        .fetch_all(pool)
        .await?
        .iter()
        .map(row_to_stock)
        .collect()
}

#[utoipa::path(
    post,
    path = "/api/stocks",
    tag = "stocks",
    request_body = NewStock,
    responses(
        (status = 201, description = "Stock created", body = Stock),
        (status = 400, description = "Missing or invalid fields", body = ErrorBody),
        (status = 409, description = "Stock code already exists in market", body = ErrorBody),
    )
)]
pub async fn create(
    State(state): State<AppState>,
    Json(body): Json<NewStock>,
) -> Result<(StatusCode, Json<Stock>), ApiError> {
    let code = body.code.trim().to_string();
    if code.is_empty() {
        return Err(ApiError::field("code", "股票代碼 is required"));
    }
    if find_by_code(&state.pool, body.market, &code)
        .await?
        .is_some()
    {
        return Err(ApiError::Conflict(format!(
            "{} already exists in market {}",
            code,
            body.market.as_str()
        )));
    }

    let price_updated_at = body.manual_price.map(|_| now_timestamp());
    let sort_order: i64 =
        sqlx::query_scalar("SELECT COALESCE(MAX(sort_order), 0) + 1 FROM stocks WHERE market = ?")
            .bind(body.market.as_str())
            .fetch_one(&state.pool)
            .await?;
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO stocks (market, code, ticker, exchange, sector, manual_price, \
         price_updated_at, pe, eps, high52, low52, note, is_active, sort_order) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 1, ?) RETURNING id",
    )
    .bind(body.market.as_str())
    .bind(&code)
    .bind(trimmed(body.ticker.as_deref()))
    .bind(trimmed(body.exchange.as_deref()))
    .bind(trimmed(body.sector.as_deref()))
    .bind(body.manual_price)
    .bind(price_updated_at)
    .bind(body.pe)
    .bind(body.eps)
    .bind(body.high52)
    .bind(body.low52)
    .bind(trimmed(body.note.as_deref()))
    .bind(sort_order)
    .fetch_one(&state.pool)
    .await?;

    Ok((StatusCode::CREATED, Json(load_one(&state.pool, id).await?)))
}

#[derive(Debug, Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
pub struct ReorderRequest {
    pub market: String,
    pub stock_ids: Vec<i64>,
}

/// Persist a complete ordering for one market. Requiring every stock prevents a
/// partial drag/drop request from silently dropping rows from the saved order.
#[utoipa::path(
    post,
    path = "/api/stocks/order",
    tag = "stocks",
    request_body = ReorderRequest,
    responses(
        (status = 200, description = "Stocks in the new saved order", body = [Stock]),
        (status = 400, description = "Id list is not the complete market set", body = ErrorBody),
    )
)]
pub async fn reorder(
    State(state): State<AppState>,
    Json(body): Json<ReorderRequest>,
) -> Result<Json<Vec<Stock>>, ApiError> {
    let market = parse_market(&body.market)?;
    let existing = load_stocks(&state.pool, Some(market)).await?;
    let expected: HashSet<i64> = existing.iter().map(|stock| stock.id).collect();
    let provided: HashSet<i64> = body.stock_ids.iter().copied().collect();

    if provided.len() != body.stock_ids.len() || provided != expected {
        return Err(ApiError::field(
            "stock_ids",
            format!(
                "must contain every {} stock id exactly once",
                market.as_str()
            ),
        ));
    }

    let mut transaction = state.pool.begin().await?;
    for (index, stock_id) in body.stock_ids.iter().enumerate() {
        sqlx::query("UPDATE stocks SET sort_order = ? WHERE id = ? AND market = ?")
            .bind(index as i64 + 1)
            .bind(stock_id)
            .bind(market.as_str())
            .execute(&mut *transaction)
            .await?;
    }
    transaction.commit().await?;

    Ok(Json(load_stocks(&state.pool, Some(market)).await?))
}

#[utoipa::path(
    patch,
    path = "/api/stocks/{id}",
    tag = "stocks",
    params(("id" = i64, Path, description = "Stock id")),
    request_body = StockPatch,
    responses(
        (status = 200, description = "Updated stock", body = Stock),
        (status = 400, description = "Missing or invalid fields", body = ErrorBody),
        (status = 404, description = "Stock not found", body = ErrorBody),
        (status = 409, description = "Stock code already exists in market", body = ErrorBody),
    )
)]
pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(patch): Json<StockPatch>,
) -> Result<Json<Stock>, ApiError> {
    let existing = load_one(&state.pool, id).await?;

    let code = match patch.code.as_deref().map(str::trim) {
        Some("") => return Err(ApiError::field("code", "股票代碼 is required")),
        Some(code) => {
            if code != existing.code
                && let Some(other) = find_by_code(&state.pool, existing.market, code).await?
                && other.id != id
            {
                return Err(ApiError::Conflict(format!(
                    "{} already exists in market {}",
                    code,
                    existing.market.as_str()
                )));
            }
            code.to_string()
        }
        None => existing.code.clone(),
    };

    // Touch the price timestamp only when the price field is supplied; `null`
    // clears both the price and its timestamp.
    let (manual_price, price_updated_at) = match patch.manual_price {
        Some(Some(price)) => (Some(price), Some(now_timestamp())),
        Some(None) => (None, None),
        None => (existing.manual_price, existing.price_updated_at.clone()),
    };

    sqlx::query(
        "UPDATE stocks SET code = ?, ticker = ?, exchange = ?, sector = ?, manual_price = ?, \
         price_updated_at = ?, pe = ?, eps = ?, high52 = ?, low52 = ?, note = ?, is_active = ? \
         WHERE id = ?",
    )
    .bind(code)
    .bind(patched_trimmed(patch.ticker, existing.ticker))
    .bind(patched_trimmed(patch.exchange, existing.exchange))
    .bind(patched_trimmed(patch.sector, existing.sector))
    .bind(manual_price)
    .bind(price_updated_at)
    .bind(patched_optional(patch.pe, existing.pe))
    .bind(patched_optional(patch.eps, existing.eps))
    .bind(patched_optional(patch.high52, existing.high52))
    .bind(patched_optional(patch.low52, existing.low52))
    .bind(patched_trimmed(patch.note, existing.note))
    .bind(i64::from(patch.is_active.unwrap_or(existing.is_active)))
    .bind(id)
    .execute(&state.pool)
    .await?;

    Ok(Json(load_one(&state.pool, id).await?))
}

/// Bulk 現價 update from a `current-price.json` body. Takes the raw file text
/// so parse errors surface as a normal validation response.
#[utoipa::path(
    post,
    path = "/api/stocks/prices",
    tag = "stocks",
    request_body(
        content = String,
        content_type = "text/plain",
        description = "Raw current-price.json text: {\"stocks\": [{\"symbol\", \"price\"}]}",
    ),
    responses(
        (status = 200, description = "Bulk price update report", body = PriceReport),
        (status = 400, description = "File could not be parsed", body = ErrorBody),
    )
)]
pub async fn upload_prices(
    State(state): State<AppState>,
    body: String,
) -> Result<Json<PriceReport>, ApiError> {
    let upload = prices::parse(&body)?;
    Ok(Json(prices::apply(&state.pool, upload).await?))
}

#[utoipa::path(
    delete,
    path = "/api/stocks/{id}",
    tag = "stocks",
    params(("id" = i64, Path, description = "Stock id")),
    responses(
        (status = 204, description = "Stock deleted"),
        (status = 404, description = "Stock not found", body = ErrorBody),
        (status = 409, description = "Stock still has trades or dividends", body = ErrorBody),
    )
)]
pub async fn remove(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    let stock = load_one(&state.pool, id).await?;
    let trades: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM trades WHERE stock_id = ?")
        .bind(id)
        .fetch_one(&state.pool)
        .await?;
    let dividends: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM dividends WHERE stock_id = ?")
        .bind(id)
        .fetch_one(&state.pool)
        .await?;
    if trades > 0 || dividends > 0 {
        return Err(ApiError::Conflict(format!(
            "{} still has {trades} trade(s) and {dividends} dividend record(s); \
             delete them before deleting the stock",
            stock.code
        )));
    }
    sqlx::query("DELETE FROM stocks WHERE id = ?")
        .bind(id)
        .execute(&state.pool)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn load_one(pool: &SqlitePool, id: i64) -> Result<Stock, ApiError> {
    let row = sqlx::query(sqlx::AssertSqlSafe(format!(
        "SELECT {STOCK_COLUMNS} FROM stocks WHERE id = ?"
    )))
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| ApiError::NotFound(format!("stock {id} not found")))?;
    row_to_stock(&row)
}

pub async fn find_by_code(
    pool: &SqlitePool,
    market: Market,
    code: &str,
) -> Result<Option<Stock>, ApiError> {
    let row = sqlx::query(sqlx::AssertSqlSafe(format!(
        "SELECT {STOCK_COLUMNS} FROM stocks WHERE market = ? AND code = ?"
    )))
    .bind(market.as_str())
    .bind(code)
    .fetch_optional(pool)
    .await?;
    row.as_ref().map(row_to_stock).transpose()
}

fn trimmed(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string)
}

fn patched_optional<T>(patch: Option<Option<T>>, existing: Option<T>) -> Option<T> {
    patch.unwrap_or(existing)
}

fn patched_trimmed(patch: Option<Option<String>>, existing: Option<String>) -> Option<String> {
    match patch {
        Some(value) => trimmed(value.as_deref()),
        None => existing,
    }
}
