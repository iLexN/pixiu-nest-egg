pub mod deposits;
pub mod dividends;
pub mod stocks;
pub mod summary;
pub mod trades;

use axum::routing::{get, patch, post};
use axum::Router;
use chrono::Datelike;
use sqlx::sqlite::SqliteRow;
use sqlx::{Row, SqlitePool};

use crate::calc::{
    deposit_active, deposit_total, dividend_amount, dividend_variance, unit_price_incl_fee,
    yield_on_cost, yield_on_price,
};
use crate::error::ApiError;
use crate::models::{
    Deposit, DepositStatus, Dividend, DividendStatus, InputMode, Market, Stock, Trade, TradeType,
};

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
}

pub fn api_router(state: AppState) -> Router {
    Router::new()
        .route("/stocks", get(stocks::list).post(stocks::create))
        .route("/stocks/order", post(stocks::reorder))
        .route("/stocks/prices", post(stocks::upload_prices))
        .route("/stocks/{id}", patch(stocks::update).delete(stocks::remove))
        .route("/trades", get(trades::list).post(trades::create))
        .route("/trades/{id}", patch(trades::update).delete(trades::remove))
        .route("/summary", get(summary::show))
        .route("/deposits", get(deposits::list).post(deposits::create))
        .route("/deposits/summary", get(deposits::summary))
        .route(
            "/deposits/{id}",
            patch(deposits::update).delete(deposits::remove),
        )
        .route("/dividends", get(dividends::list).post(dividends::create))
        .route("/dividends/summary", get(dividends::summary))
        .route(
            "/dividends/{id}",
            patch(dividends::update).delete(dividends::remove),
        )
        .with_state(state)
}

/// Kept next to the router so both stock and trade handlers share it.
pub fn parse_market(value: &str) -> Result<Market, ApiError> {
    Market::parse(&value.to_ascii_uppercase())
        .ok_or_else(|| ApiError::field("market", "market must be HK or US"))
}

pub const STOCK_COLUMNS: &str = "id, market, code, ticker, exchange, sector, manual_price, \
     price_updated_at, pe, eps, high52, low52, note, is_active, sort_order";

pub fn row_to_stock(row: &SqliteRow) -> Result<Stock, ApiError> {
    let market: String = row.try_get("market")?;
    Ok(Stock {
        id: row.try_get("id")?,
        market: Market::parse(&market)
            .ok_or_else(|| ApiError::Conflict(format!("stored market {market} is not valid")))?,
        code: row.try_get("code")?,
        ticker: row.try_get("ticker")?,
        exchange: row.try_get("exchange")?,
        sector: row.try_get("sector")?,
        manual_price: row.try_get("manual_price")?,
        price_updated_at: row.try_get("price_updated_at")?,
        pe: row.try_get("pe")?,
        eps: row.try_get("eps")?,
        high52: row.try_get("high52")?,
        low52: row.try_get("low52")?,
        note: row.try_get("note")?,
        is_active: row.try_get::<i64, _>("is_active")? != 0,
        sort_order: row.try_get("sort_order")?,
    })
}

pub const TRADE_SELECT: &str = "SELECT t.id, t.stock_id, s.market, s.code, t.trade_type, \
     t.trade_date, t.shares, t.unit_price, t.fee, t.total, t.input_mode, t.note \
     FROM trades t JOIN stocks s ON s.id = t.stock_id";

pub fn row_to_trade(row: &SqliteRow) -> Result<Trade, ApiError> {
    let market: String = row.try_get("market")?;
    let trade_type: String = row.try_get("trade_type")?;
    let input_mode: String = row.try_get("input_mode")?;
    let shares: f64 = row.try_get("shares")?;
    let total: f64 = row.try_get("total")?;
    Ok(Trade {
        id: row.try_get("id")?,
        stock_id: row.try_get("stock_id")?,
        market: Market::parse(&market)
            .ok_or_else(|| ApiError::Conflict(format!("stored market {market} is not valid")))?,
        code: row.try_get("code")?,
        trade_type: TradeType::parse(&trade_type).ok_or_else(|| {
            ApiError::Conflict(format!("stored trade type {trade_type} is not valid"))
        })?,
        trade_date: row.try_get("trade_date")?,
        shares,
        unit_price: row.try_get("unit_price")?,
        fee: row.try_get("fee")?,
        total,
        input_mode: InputMode::parse(&input_mode).ok_or_else(|| {
            ApiError::Conflict(format!("stored input mode {input_mode} is not valid"))
        })?,
        note: row.try_get("note")?,
        unit_price_incl_fee: unit_price_incl_fee(total, shares),
    })
}

pub fn now_timestamp() -> String {
    chrono::Local::now().to_rfc3339()
}

/// Local today: what deposit status derives from.
pub fn today() -> chrono::NaiveDate {
    chrono::Local::now().date_naive()
}

pub const DEPOSIT_COLUMNS: &str =
    "id, label, bank, principal, rate, interest, end_date, note1, note2, sort_order";

pub const DIVIDEND_SELECT: &str = "SELECT d.id, d.stock_id, s.market, s.code, d.pay_date, \
     d.per_share, d.shares_held, d.buy_cost, d.estimated_amount, d.received_amount, \
     d.received_price, d.note \
     FROM dividends d JOIN stocks s ON s.id = d.stock_id";

pub fn row_to_dividend(row: &SqliteRow) -> Result<Dividend, ApiError> {
    let market: String = row.try_get("market")?;
    let shares_held: Option<f64> = row.try_get("shares_held")?;
    let buy_cost: Option<f64> = row.try_get("buy_cost")?;
    let estimated_amount: Option<f64> = row.try_get("estimated_amount")?;
    let received_amount: Option<f64> = row.try_get("received_amount")?;
    let received_price: Option<f64> = row.try_get("received_price")?;
    let amount = dividend_amount(received_amount, estimated_amount);
    Ok(Dividend {
        id: row.try_get("id")?,
        stock_id: row.try_get("stock_id")?,
        market: Market::parse(&market)
            .ok_or_else(|| ApiError::Conflict(format!("stored market {market} is not valid")))?,
        code: row.try_get("code")?,
        pay_date: row.try_get("pay_date")?,
        per_share: row.try_get("per_share")?,
        shares_held,
        buy_cost,
        estimated_amount,
        received_amount,
        received_price,
        note: row.try_get("note")?,
        status: if received_amount.is_some() {
            DividendStatus::Received
        } else {
            DividendStatus::Pending
        },
        amount,
        yield_on_cost: yield_on_cost(amount, buy_cost),
        yield_on_price: yield_on_price(amount, received_price, shares_held),
        variance: dividend_variance(received_amount, estimated_amount),
    })
}

pub fn row_to_deposit(row: &SqliteRow, today: chrono::NaiveDate) -> Result<Deposit, ApiError> {
    let end_date: String = row.try_get("end_date")?;
    let parsed = chrono::NaiveDate::parse_from_str(&end_date, "%Y-%m-%d")
        .map_err(|_| ApiError::Conflict(format!("stored end_date {end_date} is not valid")))?;
    let principal: Option<f64> = row.try_get("principal")?;
    let interest: Option<f64> = row.try_get("interest")?;
    Ok(Deposit {
        id: row.try_get("id")?,
        label: row.try_get("label")?,
        bank: row.try_get("bank")?,
        principal,
        rate: row.try_get("rate")?,
        interest,
        end_date,
        note1: row.try_get("note1")?,
        note2: row.try_get("note2")?,
        sort_order: row.try_get("sort_order")?,
        total: deposit_total(principal, interest),
        status: if deposit_active(parsed, today) {
            DepositStatus::Active
        } else {
            DepositStatus::End
        },
        end_year: parsed.year(),
        end_month: parsed.month(),
    })
}
