use std::collections::{BTreeMap, HashMap};

use axum::Json;
use axum::extract::{Path, Query, State};
use chrono::Datelike;
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};

use super::{AppState, now_timestamp, parse_market, today};
use crate::calc::{DividendFacts, TradeFacts, YearRow, YearSnapshot, yearly_rows};
use crate::error::ApiError;
use crate::models::{Market, TradeType, YearlyPatch};

#[derive(Debug, Deserialize)]
pub struct YearlyQuery {
    pub market: String,
}

#[derive(Debug, Serialize)]
pub struct YearlyResponse {
    pub market: Market,
    /// The date the response was generated.
    pub today: String,
    pub years: Vec<YearRow>,
}

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
    let trades = dated_trade_facts(pool, market).await?;
    let dividends = dividend_facts(pool, market).await?;
    let snapshots = load_snapshots(pool, market).await?;
    let live_market_value = super::summary::build(pool, market)
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

/// The market's trades reduced to what the yearly rollup needs.
async fn dated_trade_facts(
    pool: &SqlitePool,
    market: Market,
) -> Result<Vec<(chrono::NaiveDate, TradeFacts)>, ApiError> {
    let rows = sqlx::query(
        "SELECT t.trade_date, t.trade_type, t.shares, t.total FROM trades t \
         JOIN stocks s ON s.id = t.stock_id WHERE s.market = ? \
         ORDER BY t.trade_date, t.id",
    )
    .bind(market.as_str())
    .fetch_all(pool)
    .await?;

    let mut facts = Vec::with_capacity(rows.len());
    for row in &rows {
        let trade_date: String = row.try_get("trade_date")?;
        let parsed = chrono::NaiveDate::parse_from_str(&trade_date, "%Y-%m-%d").map_err(|_| {
            ApiError::Conflict(format!("stored trade_date {trade_date} is not valid"))
        })?;
        let trade_type: String = row.try_get("trade_type")?;
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

/// Only received amounts count toward a year's 派息; pending estimates are
/// filtered out here so the rollup sees nothing but real payouts.
async fn dividend_facts(
    pool: &SqlitePool,
    market: Market,
) -> Result<Vec<DividendFacts<'static>>, ApiError> {
    let rows = sqlx::query(
        "SELECT d.pay_date, d.received_amount FROM dividends d \
         JOIN stocks s ON s.id = d.stock_id WHERE s.market = ? \
         AND d.received_amount IS NOT NULL",
    )
    .bind(market.as_str())
    .fetch_all(pool)
    .await?;

    let mut facts = Vec::with_capacity(rows.len());
    for row in &rows {
        let pay_date: String = row.try_get("pay_date")?;
        let parsed = chrono::NaiveDate::parse_from_str(&pay_date, "%Y-%m-%d")
            .map_err(|_| ApiError::Conflict(format!("stored pay_date {pay_date} is not valid")))?;
        facts.push(DividendFacts {
            code: "",
            pay_date: parsed,
            received_amount: row.try_get("received_amount")?,
        });
    }
    Ok(facts)
}

async fn load_snapshots(
    pool: &SqlitePool,
    market: Market,
) -> Result<HashMap<i32, YearSnapshot>, ApiError> {
    let rows = sqlx::query(
        "SELECT year, invested, cost, market_value, sold_pl, updated_at \
         FROM year_snapshots WHERE market = ?",
    )
    .bind(market.as_str())
    .fetch_all(pool)
    .await?;

    let mut snapshots = HashMap::with_capacity(rows.len());
    for row in &rows {
        snapshots.insert(
            row.try_get("year")?,
            YearSnapshot {
                invested: row.try_get("invested")?,
                cost: row.try_get("cost")?,
                market_value: row.try_get("market_value")?,
                sold_pl: row.try_get("sold_pl")?,
                updated_at: row.try_get("updated_at")?,
            },
        );
    }
    Ok(snapshots)
}

async fn load_snapshot(
    pool: &SqlitePool,
    market: Market,
    year: i32,
) -> Result<Option<crate::models::YearSnapshot>, ApiError> {
    let row = sqlx::query(
        "SELECT invested, cost, market_value, sold_pl, updated_at \
         FROM year_snapshots WHERE market = ? AND year = ?",
    )
    .bind(market.as_str())
    .bind(year)
    .fetch_optional(pool)
    .await?;
    row.map(|row| {
        Ok(crate::models::YearSnapshot {
            market,
            year,
            invested: row.try_get("invested")?,
            cost: row.try_get("cost")?,
            market_value: row.try_get("market_value")?,
            sold_pl: row.try_get("sold_pl")?,
            updated_at: row.try_get("updated_at")?,
        })
    })
    .transpose()
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

/// HK `sold_pl` per year — feeds 投資純利 in the Month Stat yearly block and
/// 投資P/L in the year review.
pub async fn hk_sold_pl(pool: &SqlitePool) -> Result<BTreeMap<i32, f64>, ApiError> {
    let rows = sqlx::query(
        "SELECT year, sold_pl FROM year_snapshots \
         WHERE market = 'HK' AND sold_pl IS NOT NULL",
    )
    .fetch_all(pool)
    .await?;
    let mut map = BTreeMap::new();
    for row in &rows {
        map.insert(row.try_get("year")?, row.try_get("sold_pl")?);
    }
    Ok(map)
}

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

    let existing = load_snapshot(&state.pool, market, year).await?;
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
        sqlx::query("DELETE FROM year_snapshots WHERE market = ? AND year = ?")
            .bind(market.as_str())
            .bind(year)
            .execute(&state.pool)
            .await?;
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

    sqlx::query(
        "INSERT INTO year_snapshots (market, year, invested, cost, market_value, sold_pl, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?) \
         ON CONFLICT (market, year) DO UPDATE SET \
         invested = excluded.invested, cost = excluded.cost, \
         market_value = excluded.market_value, sold_pl = excluded.sold_pl, \
         updated_at = excluded.updated_at",
    )
    .bind(market.as_str())
    .bind(year)
    .bind(invested)
    .bind(cost)
    .bind(market_value)
    .bind(sold_pl)
    .bind(now_timestamp())
    .execute(&state.pool)
    .await?;

    Ok(Json(
        load_snapshot(&state.pool, market, year)
            .await?
            .expect("just upserted"),
    ))
}

/// Freeze the year: store the currently computed cumulative 成本 and the
/// live 總市值 as the snapshot. `invested` is left untouched.
pub async fn freeze(
    State(state): State<AppState>,
    Path((market, year)): Path<(String, i32)>,
) -> Result<Json<crate::models::YearSnapshot>, ApiError> {
    let market = parse_market(&market)?;
    if !(2000..=2100).contains(&year) {
        return Err(ApiError::field("year", "year must be a plausible year"));
    }

    // Computed year-end cost: Σ BUY total on or before Dec 31 of the year.
    let cost: f64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(t.total), 0.0) FROM trades t \
         JOIN stocks s ON s.id = t.stock_id \
         WHERE s.market = ? AND t.trade_type = 'BUY' AND t.trade_date <= ?",
    )
    .bind(market.as_str())
    .bind(format!("{year:04}-12-31"))
    .fetch_one(&state.pool)
    .await?;
    let market_value = super::summary::build(&state.pool, market)
        .await?
        .totals
        .market_value;

    sqlx::query(
        "INSERT INTO year_snapshots (market, year, cost, market_value, updated_at) \
         VALUES (?, ?, ?, ?, ?) \
         ON CONFLICT (market, year) DO UPDATE SET \
         cost = excluded.cost, market_value = excluded.market_value, \
         updated_at = excluded.updated_at",
    )
    .bind(market.as_str())
    .bind(year)
    .bind(cost)
    .bind(market_value)
    .bind(now_timestamp())
    .execute(&state.pool)
    .await?;

    Ok(Json(
        load_snapshot(&state.pool, market, year)
            .await?
            .expect("just upserted"),
    ))
}
