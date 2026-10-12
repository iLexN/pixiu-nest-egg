use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use chrono::Datelike;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use super::AppState;
use crate::calc::{
    DividendFacts, DividendInput, DividendYearRollup, dividend_year_rollups, holdings_snapshot,
    net_from_gross, validate_dividend,
};
use crate::error::{ApiError, ErrorBody};
use crate::models::{Dividend, DividendPatch, Market, NewDividend};
use crate::store::dividends as store;
use crate::store::months as months_store;
use crate::store::{now_timestamp, today};

#[derive(Debug, Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
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

#[utoipa::path(
    get,
    path = "/api/dividends",
    tag = "dividends",
    params(ListQuery),
    responses(
        (status = 200, description = "List dividend records", body = [Dividend]),
    )
)]
pub async fn list(
    State(state): State<AppState>,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<Dividend>>, ApiError> {
    let received = match query.status.as_deref() {
        Some(status) => match status.to_ascii_lowercase().as_str() {
            "pending" => Some(false),
            "received" => Some(true),
            _ => {
                return Err(ApiError::field(
                    "status",
                    "status must be pending or received",
                ));
            }
        },
        None => None,
    };
    let market = query
        .market
        .as_deref()
        .map(super::parse_market)
        .transpose()?;
    let descending =
        matches!(query.order.as_deref(), Some(order) if order.eq_ignore_ascii_case("desc"));
    store::list(
        &state.pool,
        market,
        query.stock_id,
        received,
        query.year,
        descending,
    )
    .await
    .map(Json)
    .map_err(ApiError::from)
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
    let facts = store::dated_trade_facts(pool, stock_id).await?;
    Ok(holdings_snapshot(&facts, as_of))
}

#[utoipa::path(
    post,
    path = "/api/dividends",
    tag = "dividends",
    request_body = NewDividend,
    responses(
        (status = 201, description = "Dividend created with frozen holding/cost snapshots", body = Dividend),
        (status = 400, description = "Missing or invalid fields", body = ErrorBody),
        (status = 404, description = "Stock not found", body = ErrorBody),
    )
)]
pub async fn create(
    State(state): State<AppState>,
    Json(body): Json<NewDividend>,
) -> Result<(StatusCode, Json<Dividend>), ApiError> {
    let (stock_id, _market) = super::resolve_stock(
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
        withholding_tax: None,
        gross_amount: None,
    })
    .map_err(ApiError::Validation)?;

    let id =
        store::insert_dividend(&state.pool, stock_id, &validated, body.note.as_deref()).await?;
    Ok((
        StatusCode::CREATED,
        Json(store::load_one(&state.pool, id).await?),
    ))
}

#[utoipa::path(
    patch,
    path = "/api/dividends/{id}",
    tag = "dividends",
    params(("id" = i64, Path, description = "Dividend id")),
    request_body = DividendPatch,
    responses(
        (status = 200, description = "Updated dividend (receipt lifecycle fields)", body = Dividend),
        (status = 400, description = "Missing or invalid fields", body = ErrorBody),
        (status = 404, description = "Dividend not found", body = ErrorBody),
    )
)]
pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(patch): Json<DividendPatch>,
) -> Result<Json<Dividend>, ApiError> {
    let existing = store::load_one(&state.pool, id).await?;

    let (stock_id, stock_market) = match patch.stock_id {
        Some(stock_id) => super::resolve_stock(&state.pool, Some(stock_id), None, None).await?,
        None => (existing.stock_id, existing.market),
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

    if patch.gross_amount.is_some() && patch.received_amount.is_some() {
        return Err(ApiError::field(
            "gross_amount",
            "supply 派息 or 實收派息, not both",
        ));
    }

    // Gross input resolves to the net amount before the merge/validate/transition
    // logic; the stored tax wins over a request that omits it.
    let mut withholding_tax = merge_num(patch.withholding_tax, existing.withholding_tax);
    let received_amount = match patch.gross_amount {
        Some(gross) => Some(net_from_gross(gross, withholding_tax)),
        None => merge_num(patch.received_amount, existing.received_amount),
    };
    // Un-receive clears the tax too, so a re-receive starts clean.
    if existing.received_amount.is_some() && received_amount.is_none() {
        withholding_tax = None;
    }
    let mut received_price = merge_num(patch.received_price, existing.received_price);
    // 收訖 with no price supplied snapshots the stock's current 現價;
    // clearing the price on an already-received record stays cleared.
    if existing.received_amount.is_none() && received_amount.is_some() && received_price.is_none() {
        received_price = crate::store::stocks::manual_price(&state.pool, stock_id).await?;
    }
    let note = merge_text(patch.note, existing.note.clone());

    let validated = validate_dividend(DividendInput {
        pay_date: &pay_date,
        per_share: merge_num(patch.per_share, existing.per_share),
        shares_held,
        buy_cost,
        estimated_amount: merge_num(patch.estimated_amount, existing.estimated_amount),
        received_amount,
        received_price,
        withholding_tax,
        gross_amount: patch.gross_amount,
    })
    .map_err(ApiError::Validation)?;

    let now = now_timestamp();
    let mut tx = state.pool.begin().await?;
    store::update_dividend(&mut tx, id, stock_id, &validated, note, &now).await?;

    // 收訖: received_amount NULL → set banks the amount (HK → the HS cash
    // row; US → the IBKR USD cash meta value, already USD) and, for HK,
    // records the div:<id> month item. Clearing received_amount reverses.
    let credited_before = store::credited_amount(&mut tx, id).await?;
    let auto_key = format!("div:{id}");
    let month = format!("{}-01", &validated.pay_date[..7]);
    match (existing.received_amount, validated.received_amount) {
        (None, Some(amount)) => {
            let mut credited = None;
            if patch.bank_in.unwrap_or(true) {
                match stock_market {
                    Market::Hk => {
                        if let Some(asset_id) = months_store::hs_cash_asset_id(&mut tx).await? {
                            months_store::add_manual_asset_amount(&mut tx, amount, &now, asset_id)
                                .await?;
                            credited = Some(amount);
                        }
                    }
                    Market::Us => {
                        credit_ibkr_usd(&mut tx, amount, &now).await?;
                        credited = Some(amount);
                    }
                }
            }
            store::set_credited_amount(&mut tx, id, credited).await?;
            // US dividends land in IBKR, not 活期 — no adjustment item.
            if stock_market == Market::Hk {
                months_store::record_receipt_item(
                    &mut tx,
                    &month,
                    &auto_key,
                    &existing.code,
                    amount,
                )
                .await?;
            }
        }
        (Some(_), None) => {
            if let Some(amount) = credited_before {
                match stock_market {
                    Market::Hk => {
                        if let Some(asset_id) = months_store::hs_cash_asset_id(&mut tx).await? {
                            months_store::add_manual_asset_amount(&mut tx, -amount, &now, asset_id)
                                .await?;
                        }
                    }
                    Market::Us => credit_ibkr_usd(&mut tx, -amount, &now).await?,
                }
            }
            store::set_credited_amount(&mut tx, id, None).await?;
            months_store::delete_auto_item(&mut tx, &month, &auto_key).await?;
        }
        _ => {}
    }
    tx.commit().await?;

    Ok(Json(store::load_one(&state.pool, id).await?))
}

/// `ibkr.usd_cash += amount` — a US dividend receipt lands there directly.
async fn credit_ibkr_usd(
    conn: &mut sqlx::SqliteConnection,
    amount: f64,
    _now: &str,
) -> Result<(), ApiError> {
    let current =
        crate::store::meta::meta_get(&mut *conn, crate::store::overview::IBKR_USD_CASH_KEY).await?;
    let total = current.and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0) + amount;
    crate::store::meta::meta_put(
        &mut *conn,
        crate::store::overview::IBKR_USD_CASH_KEY,
        Some(&total.to_string()),
    )
    .await?;
    Ok(())
}

#[utoipa::path(
    delete,
    path = "/api/dividends/{id}",
    tag = "dividends",
    params(("id" = i64, Path, description = "Dividend id")),
    responses(
        (status = 204, description = "Dividend deleted"),
        (status = 404, description = "Dividend not found", body = ErrorBody),
    )
)]
pub async fn remove(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    if store::delete_dividend(&state.pool, id).await? == 0 {
        return Err(ApiError::NotFound(format!("dividend {id} not found")));
    }
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
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
#[utoipa::path(
    get,
    path = "/api/dividends/summary",
    tag = "dividends",
    params(ListQuery),
    responses(
        (status = 200, description = "Dividend totals and year rollups", body = DividendSummaryResponse),
    )
)]
pub async fn summary(
    State(state): State<AppState>,
    Query(query): Query<ListQuery>,
) -> Result<Json<DividendSummaryResponse>, ApiError> {
    let market = query
        .market
        .as_deref()
        .map(super::parse_market)
        .transpose()?;
    let dividends = store::list_for_market(&state.pool, market).await?;

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
