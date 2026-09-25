use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde::Deserialize;
use sqlx::SqlitePool;

use super::{AppState, TRADE_SELECT, now_timestamp, parse_market, row_to_trade};
use crate::calc::{TradeInput, ValidatedTrade, validate_trade};
use crate::error::ApiError;
use crate::models::{InputMode, Market, NewTrade, Trade, TradePatch};

#[derive(Debug, Deserialize)]
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

pub async fn list(
    State(state): State<AppState>,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<Trade>>, ApiError> {
    let mut sql = TRADE_SELECT.to_string();
    let mut conditions: Vec<&str> = Vec::new();
    if query.market.is_some() {
        conditions.push("s.market = ?");
    }
    if query.stock_id.is_some() {
        conditions.push("t.stock_id = ?");
    }
    if query.code.is_some() {
        conditions.push("s.code = ?");
    }
    if query.from.is_some() {
        conditions.push("t.trade_date >= ?");
    }
    if query.to.is_some() {
        conditions.push("t.trade_date <= ?");
    }
    if !conditions.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&conditions.join(" AND "));
    }
    let descending =
        matches!(query.order.as_deref(), Some(order) if order.eq_ignore_ascii_case("desc"));
    sql.push_str(if descending {
        " ORDER BY t.trade_date DESC, t.id DESC"
    } else {
        " ORDER BY t.trade_date ASC, t.id ASC"
    });

    let mut statement = sqlx::query(sqlx::AssertSqlSafe(sql));
    if let Some(market) = query.market.as_deref() {
        statement = statement.bind(parse_market(market)?.as_str());
    }
    if let Some(stock_id) = query.stock_id {
        statement = statement.bind(stock_id);
    }
    if let Some(code) = query.code.as_deref() {
        statement = statement.bind(code.to_string());
    }
    if let Some(from) = query.from.as_deref() {
        statement = statement.bind(from.to_string());
    }
    if let Some(to) = query.to.as_deref() {
        statement = statement.bind(to.to_string());
    }

    statement
        .fetch_all(&state.pool)
        .await?
        .iter()
        .map(row_to_trade)
        .collect::<Result<Vec<_>, _>>()
        .map(Json)
}

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

    let id = insert_trade(&state.pool, stock_id, &validated, body.note.as_deref()).await?;
    Ok((StatusCode::CREATED, Json(load_one(&state.pool, id).await?)))
}

pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(patch): Json<TradePatch>,
) -> Result<Json<Trade>, ApiError> {
    let existing = load_one(&state.pool, id).await?;

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

    sqlx::query(
        "UPDATE trades SET stock_id = ?, trade_type = ?, trade_date = ?, shares = ?, \
         unit_price = ?, fee = ?, total = ?, input_mode = ?, note = ?, updated_at = ? WHERE id = ?",
    )
    .bind(stock_id)
    .bind(validated.trade_type.as_str())
    .bind(&validated.trade_date)
    .bind(validated.shares)
    .bind(validated.unit_price)
    .bind(validated.fee)
    .bind(validated.total)
    .bind(validated.input_mode.as_str())
    .bind(note)
    .bind(now_timestamp())
    .bind(id)
    .execute(&state.pool)
    .await?;

    Ok(Json(load_one(&state.pool, id).await?))
}

pub async fn remove(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    let result = sqlx::query("DELETE FROM trades WHERE id = ?")
        .bind(id)
        .execute(&state.pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound(format!("trade {id} not found")));
    }
    Ok(StatusCode::NO_CONTENT)
}

pub async fn insert_trade(
    pool: &SqlitePool,
    stock_id: i64,
    trade: &ValidatedTrade,
    note: Option<&str>,
) -> Result<i64, ApiError> {
    let now = now_timestamp();
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO trades (stock_id, trade_type, trade_date, shares, unit_price, fee, total, \
         input_mode, note, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(stock_id)
    .bind(trade.trade_type.as_str())
    .bind(&trade.trade_date)
    .bind(trade.shares)
    .bind(trade.unit_price)
    .bind(trade.fee)
    .bind(trade.total)
    .bind(trade.input_mode.as_str())
    .bind(note)
    .bind(&now)
    .bind(&now)
    .fetch_one(pool)
    .await?;
    Ok(id)
}

pub async fn load_one(pool: &SqlitePool, id: i64) -> Result<Trade, ApiError> {
    let row = sqlx::query(sqlx::AssertSqlSafe(format!(
        "{TRADE_SELECT} WHERE t.id = ?"
    )))
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| ApiError::NotFound(format!("trade {id} not found")))?;
    row_to_trade(&row)
}

pub async fn resolve_stock(
    pool: &SqlitePool,
    stock_id: Option<i64>,
    market: Option<Market>,
    code: Option<&str>,
) -> Result<(i64, Market), ApiError> {
    if let Some(stock_id) = stock_id {
        let stock = super::stocks::load_one(pool, stock_id)
            .await
            .map_err(|err| match err {
                ApiError::NotFound(_) => {
                    ApiError::field("stock_id", format!("stock {stock_id} is not registered"))
                }
                other => other,
            })?;
        return Ok((stock.id, stock.market));
    }

    match (market, code.map(str::trim).filter(|c| !c.is_empty())) {
        (Some(market), Some(code)) => {
            let stock = super::stocks::find_by_code(pool, market, code)
                .await?
                .ok_or_else(|| {
                    ApiError::field(
                        "code",
                        format!("{code} is not registered in market {}", market.as_str()),
                    )
                })?;
            Ok((stock.id, stock.market))
        }
        _ => Err(ApiError::field(
            "stock_id",
            "either stock_id, or market and 股票代碼, is required",
        )),
    }
}
