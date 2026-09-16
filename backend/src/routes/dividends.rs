use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::Datelike;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use super::{now_timestamp, row_to_dividend, today, AppState, DIVIDEND_SELECT};
use crate::calc::{
    dividend_year_rollups, holdings_snapshot, validate_dividend, DividendFacts, DividendInput,
    DividendYearRollup, TradeFacts, ValidatedDividend,
};
use crate::error::ApiError;
use crate::models::{Dividend, DividendPatch, NewDividend, TradeType};

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub market: Option<String>,
    pub stock_id: Option<i64>,
    /// `pending` (no received amount yet) or `received`.
    pub status: Option<String>,
    /// Restrict to dividends whose pay date falls in this year.
    pub year: Option<i32>,
    /// `asc` (default) or `desc`, by pay date.
    pub order: Option<String>,
}

pub async fn list(
    State(state): State<AppState>,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<Dividend>>, ApiError> {
    let mut sql = DIVIDEND_SELECT.to_string();
    let mut conditions: Vec<&str> = Vec::new();
    if query.market.is_some() {
        conditions.push("s.market = ?");
    }
    if query.stock_id.is_some() {
        conditions.push("d.stock_id = ?");
    }
    if let Some(status) = query.status.as_deref() {
        match status.to_ascii_lowercase().as_str() {
            "pending" => conditions.push("d.received_amount IS NULL"),
            "received" => conditions.push("d.received_amount IS NOT NULL"),
            _ => {
                return Err(ApiError::field(
                    "status",
                    "status must be pending or received",
                ))
            }
        }
    }
    if query.year.is_some() {
        conditions.push("d.pay_date >= ? AND d.pay_date <= ?");
    }
    if !conditions.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&conditions.join(" AND "));
    }
    let descending =
        matches!(query.order.as_deref(), Some(order) if order.eq_ignore_ascii_case("desc"));
    sql.push_str(if descending {
        " ORDER BY d.pay_date DESC, d.id DESC"
    } else {
        " ORDER BY d.pay_date ASC, d.id ASC"
    });

    let mut statement = sqlx::query(&sql);
    if let Some(market) = query.market.as_deref() {
        statement = statement.bind(super::parse_market(market)?.as_str());
    }
    if let Some(stock_id) = query.stock_id {
        statement = statement.bind(stock_id);
    }
    if let Some(year) = query.year {
        statement = statement
            .bind(format!("{year:04}-01-01"))
            .bind(format!("{year:04}-12-31"));
    }

    statement
        .fetch_all(&state.pool)
        .await?
        .iter()
        .map(row_to_dividend)
        .collect::<Result<Vec<_>, _>>()
        .map(Json)
}

/// The stock's trades reduced to what the snapshot derivation needs.
async fn dated_trade_facts(
    pool: &SqlitePool,
    stock_id: i64,
) -> Result<Vec<(chrono::NaiveDate, TradeFacts)>, ApiError> {
    let rows = sqlx::query(
        "SELECT trade_type, trade_date, shares, total FROM trades \
         WHERE stock_id = ? ORDER BY trade_date, id",
    )
    .bind(stock_id)
    .fetch_all(pool)
    .await?;
    let mut facts = Vec::with_capacity(rows.len());
    for row in &rows {
        use sqlx::Row;
        let trade_type: String = row.try_get("trade_type")?;
        let trade_date: String = row.try_get("trade_date")?;
        let parsed = chrono::NaiveDate::parse_from_str(&trade_date, "%Y-%m-%d").map_err(|_| {
            ApiError::Conflict(format!("stored trade_date {trade_date} is not valid"))
        })?;
        facts.push((
            parsed,
            TradeFacts {
                trade_type: TradeType::parse(&trade_type).ok_or_else(|| {
                    ApiError::Conflict(format!("stored trade type {trade_type} is not valid"))
                })?,
                shares: row.try_get("shares")?,
                total: row.try_get("total")?,
            },
        ));
    }
    Ok(facts)
}

/// Derive the point-in-time snapshots for a stock as of `pay_date`.
async fn derive_snapshots(
    pool: &SqlitePool,
    stock_id: i64,
    pay_date: &str,
) -> Result<(f64, f64), ApiError> {
    let as_of = chrono::NaiveDate::parse_from_str(pay_date, "%Y-%m-%d").map_err(|_| {
        ApiError::field(
            "pay_date",
            "派息日 must be a calendar date in YYYY-MM-DD form",
        )
    })?;
    let facts = dated_trade_facts(pool, stock_id).await?;
    Ok(holdings_snapshot(&facts, as_of))
}

pub async fn create(
    State(state): State<AppState>,
    Json(body): Json<NewDividend>,
) -> Result<(StatusCode, Json<Dividend>), ApiError> {
    let (stock_id, _market) = super::trades::resolve_stock(
        &state.pool,
        body.stock_id,
        body.market,
        body.code.as_deref(),
    )
    .await?;

    // Snapshots come from the request when given, else from trades on or
    // before the pay date — the values are then frozen on the record.
    let (shares_held, buy_cost) = match (body.shares_held, body.buy_cost) {
        (Some(shares), Some(cost)) => (shares, cost),
        _ => {
            let (shares, cost) = derive_snapshots(&state.pool, stock_id, &body.pay_date).await?;
            (
                body.shares_held.unwrap_or(shares),
                body.buy_cost.unwrap_or(cost),
            )
        }
    };

    let validated = validate_dividend(DividendInput {
        pay_date: &body.pay_date,
        per_share: body.per_share,
        shares_held: Some(shares_held),
        buy_cost: Some(buy_cost),
        estimated_amount: body.estimated_amount,
        received_amount: None,
        received_price: None,
    })
    .map_err(ApiError::Validation)?;

    let id = insert_dividend(&state.pool, stock_id, &validated, body.note.as_deref()).await?;
    Ok((StatusCode::CREATED, Json(load_one(&state.pool, id).await?)))
}

pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(patch): Json<DividendPatch>,
) -> Result<Json<Dividend>, ApiError> {
    let existing = load_one(&state.pool, id).await?;

    let stock_id = match patch.stock_id {
        Some(stock_id) => {
            super::trades::resolve_stock(&state.pool, Some(stock_id), None, None)
                .await?
                .0
        }
        None => existing.stock_id,
    };
    let pay_date = patch.pay_date.unwrap_or(existing.pay_date);

    let merge_num = |patch: Option<Option<f64>>, existing: Option<f64>| match patch {
        Some(value) => value,
        None => existing,
    };
    let merge_text = |patch: Option<Option<String>>, existing: Option<String>| match patch {
        Some(value) => value
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        None => existing,
    };

    // Snapshot fields: explicit patch values win; `refresh_snapshots`
    // re-derives from trades; otherwise the stored snapshots stay frozen.
    let (shares_held, buy_cost) = if patch.refresh_snapshots {
        let (shares, cost) = derive_snapshots(&state.pool, stock_id, &pay_date).await?;
        (Some(shares), Some(cost))
    } else {
        (
            merge_num(patch.shares_held, existing.shares_held),
            merge_num(patch.buy_cost, existing.buy_cost),
        )
    };

    let received_amount = merge_num(patch.received_amount, existing.received_amount);
    let received_price = merge_num(patch.received_price, existing.received_price);
    let note = merge_text(patch.note, existing.note.clone());

    let validated = validate_dividend(DividendInput {
        pay_date: &pay_date,
        per_share: merge_num(patch.per_share, existing.per_share),
        shares_held,
        buy_cost,
        estimated_amount: merge_num(patch.estimated_amount, existing.estimated_amount),
        received_amount,
        received_price,
    })
    .map_err(ApiError::Validation)?;

    sqlx::query(
        "UPDATE dividends SET stock_id = ?, pay_date = ?, per_share = ?, shares_held = ?, \
         buy_cost = ?, estimated_amount = ?, received_amount = ?, \
         received_price = ?, note = ?, updated_at = ? WHERE id = ?",
    )
    .bind(stock_id)
    .bind(&validated.pay_date)
    .bind(validated.per_share)
    .bind(validated.shares_held)
    .bind(validated.buy_cost)
    .bind(validated.estimated_amount)
    .bind(validated.received_amount)
    .bind(validated.received_price)
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
    let result = sqlx::query("DELETE FROM dividends WHERE id = ?")
        .bind(id)
        .execute(&state.pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound(format!("dividend {id} not found")));
    }
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Serialize)]
pub struct DividendSummaryResponse {
    /// The date the response was generated.
    pub today: String,
    /// Pending dividends (no received amount), earliest pay date first.
    pub pending: Vec<Dividend>,
    /// Per-year totals of received amounts, with a per-stock breakdown.
    pub years: Vec<DividendYearRollup>,
    /// Distinct pay-date years in the data plus the current year.
    pub history_years: Vec<i32>,
}

/// Every figure here is derived from the stored dividends on each read.
pub async fn summary(
    State(state): State<AppState>,
    Query(query): Query<ListQuery>,
) -> Result<Json<DividendSummaryResponse>, ApiError> {
    let mut sql = DIVIDEND_SELECT.to_string();
    if query.market.is_some() {
        sql.push_str(" WHERE s.market = ?");
    }
    sql.push_str(" ORDER BY d.pay_date ASC, d.id ASC");
    let mut statement = sqlx::query(&sql);
    if let Some(market) = query.market.as_deref() {
        statement = statement.bind(super::parse_market(market)?.as_str());
    }
    let dividends: Vec<Dividend> = statement
        .fetch_all(&state.pool)
        .await?
        .iter()
        .map(row_to_dividend)
        .collect::<Result<Vec<_>, _>>()?;

    let mut facts = Vec::with_capacity(dividends.len());
    for dividend in &dividends {
        let pay_date =
            chrono::NaiveDate::parse_from_str(&dividend.pay_date, "%Y-%m-%d").map_err(|_| {
                ApiError::Conflict(format!(
                    "stored pay_date {} is not valid",
                    dividend.pay_date
                ))
            })?;
        facts.push(DividendFacts {
            code: &dividend.code,
            pay_date,
            received_amount: dividend.received_amount,
        });
    }

    let pending: Vec<Dividend> = dividends
        .iter()
        .filter(|dividend| dividend.received_amount.is_none())
        .cloned()
        .collect();

    let mut history_years: Vec<i32> = dividends
        .iter()
        .map(|dividend| dividend.pay_date.get(..4).and_then(|y| y.parse().ok()))
        .collect::<Option<Vec<i32>>>()
        .unwrap_or_default();
    history_years.push(today().year());
    history_years.sort_unstable();
    history_years.dedup();

    Ok(Json(DividendSummaryResponse {
        today: today().to_string(),
        pending,
        years: dividend_year_rollups(&facts),
        history_years,
    }))
}

pub async fn insert_dividend(
    pool: &SqlitePool,
    stock_id: i64,
    dividend: &ValidatedDividend,
    note: Option<&str>,
) -> Result<i64, ApiError> {
    let now = now_timestamp();
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO dividends (stock_id, pay_date, per_share, shares_held, buy_cost, \
         estimated_amount, received_amount, received_price, note, \
         created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(stock_id)
    .bind(&dividend.pay_date)
    .bind(dividend.per_share)
    .bind(dividend.shares_held)
    .bind(dividend.buy_cost)
    .bind(dividend.estimated_amount)
    .bind(dividend.received_amount)
    .bind(dividend.received_price)
    .bind(note)
    .bind(&now)
    .bind(&now)
    .fetch_one(pool)
    .await?;
    Ok(id)
}

pub async fn load_one(pool: &SqlitePool, id: i64) -> Result<Dividend, ApiError> {
    let row = sqlx::query(&format!("{DIVIDEND_SELECT} WHERE d.id = ?"))
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("dividend {id} not found")))?;
    row_to_dividend(&row)
}
