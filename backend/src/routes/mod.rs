pub mod aia;
pub mod bonds;
pub mod deposits;
pub mod dividends;
pub mod family;
pub mod forecast;
pub mod months;
pub mod mpf;
pub mod overview;
pub mod stocks;
pub mod summary;
pub mod trades;
pub mod year_review;
pub mod yearly;

use axum::Router;
use chrono::Datelike;
use sqlx::sqlite::SqliteRow;
use sqlx::{Row, SqlitePool};
use utoipa::OpenApi;

use crate::calc::{
    bond_active, coupon_expected, coupon_status, coupon_variance, deposit_total, dividend_amount,
    dividend_variance, unit_price_incl_fee, yield_on_cost, yield_on_price,
};
use crate::error::ApiError;
use crate::models::{
    Bond, BondCoupon, BondStatus, Deposit, DepositStatus, Dividend, DividendStatus, FamilyDeposit,
    InputMode, Market, Stock, Trade, TradeType,
};

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
}

#[derive(utoipa::OpenApi)]
#[openapi(info(title = "Wealth Report API", version = "0.1.0"))]
struct ApiDoc;

/// Build the API router and the OpenAPI document describing it. Routes MUST be
/// registered via `routes!` — a plain `.route()` on an `OpenApiRouter` compiles
/// but silently leaves the endpoint out of the doc.
pub fn api_router(state: AppState) -> (Router, utoipa::openapi::OpenApi) {
    let (router, api) = utoipa_axum::router::OpenApiRouter::with_openapi(ApiDoc::openapi())
        .routes(utoipa_axum::routes!(stocks::list, stocks::create))
        .routes(utoipa_axum::routes!(stocks::reorder))
        .routes(utoipa_axum::routes!(stocks::upload_prices))
        .routes(utoipa_axum::routes!(stocks::update, stocks::remove))
        .routes(utoipa_axum::routes!(trades::list, trades::create))
        .routes(utoipa_axum::routes!(trades::update, trades::remove))
        .routes(utoipa_axum::routes!(summary::show))
        .routes(utoipa_axum::routes!(yearly::list))
        .routes(utoipa_axum::routes!(yearly::update))
        .routes(utoipa_axum::routes!(yearly::freeze))
        .routes(utoipa_axum::routes!(year_review::list))
        .routes(utoipa_axum::routes!(year_review::update))
        .routes(utoipa_axum::routes!(deposits::list, deposits::create))
        .routes(utoipa_axum::routes!(deposits::summary))
        .routes(utoipa_axum::routes!(deposits::update, deposits::remove))
        .routes(utoipa_axum::routes!(deposits::receive))
        .routes(utoipa_axum::routes!(deposits::unreceive))
        .routes(utoipa_axum::routes!(family::list, family::create))
        .routes(utoipa_axum::routes!(family::summary))
        .routes(utoipa_axum::routes!(family::update, family::remove))
        .routes(utoipa_axum::routes!(family::receive))
        .routes(utoipa_axum::routes!(family::unreceive))
        .routes(utoipa_axum::routes!(family::update_note))
        .routes(utoipa_axum::routes!(dividends::list, dividends::create))
        .routes(utoipa_axum::routes!(dividends::summary))
        .routes(utoipa_axum::routes!(dividends::update, dividends::remove))
        .routes(utoipa_axum::routes!(bonds::list, bonds::create))
        .routes(utoipa_axum::routes!(bonds::summary))
        .routes(utoipa_axum::routes!(bonds::update, bonds::remove))
        .routes(utoipa_axum::routes!(bonds::receive))
        .routes(utoipa_axum::routes!(bonds::unreceive))
        .routes(utoipa_axum::routes!(
            bonds::list_coupons,
            bonds::create_coupon
        ))
        .routes(utoipa_axum::routes!(
            bonds::update_coupon,
            bonds::remove_coupon
        ))
        .routes(utoipa_axum::routes!(mpf::overview))
        .routes(utoipa_axum::routes!(mpf::create))
        .routes(utoipa_axum::routes!(mpf::update, mpf::remove))
        .routes(utoipa_axum::routes!(mpf::update_note))
        .routes(utoipa_axum::routes!(mpf::remove_history))
        .routes(utoipa_axum::routes!(aia::list, aia::create))
        .routes(utoipa_axum::routes!(aia::summary))
        .routes(utoipa_axum::routes!(aia::update_rate))
        .routes(utoipa_axum::routes!(aia::update, aia::remove))
        .routes(utoipa_axum::routes!(aia::list_events, aia::create_event))
        .routes(utoipa_axum::routes!(aia::remove_event))
        .routes(utoipa_axum::routes!(months::list))
        .routes(utoipa_axum::routes!(months::summary))
        .routes(utoipa_axum::routes!(
            months::settings,
            months::update_settings
        ))
        .routes(utoipa_axum::routes!(
            months::show,
            months::upsert,
            months::remove
        ))
        .routes(utoipa_axum::routes!(months::create_item))
        .routes(utoipa_axum::routes!(months::dismiss_item))
        .routes(utoipa_axum::routes!(
            months::update_item,
            months::remove_item
        ))
        .routes(utoipa_axum::routes!(
            months::list_assets,
            months::create_asset
        ))
        .routes(utoipa_axum::routes!(
            months::update_asset,
            months::remove_asset
        ))
        .routes(utoipa_axum::routes!(overview::overview))
        .routes(utoipa_axum::routes!(overview::ibkr, overview::update_ibkr))
        .routes(utoipa_axum::routes!(forecast::forecast))
        .routes(utoipa_axum::routes!(forecast::create_item))
        .routes(utoipa_axum::routes!(
            forecast::update_item,
            forecast::remove_item
        ))
        .routes(utoipa_axum::routes!(forecast::convert))
        .split_for_parts();
    (router.with_state(state), api)
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

/// Record a receipt's bank-in as an `adjustment` month item (`dep-end`,
/// `bond-end`, `coupon`, `div`), inside the caller's transaction. Skipped when
/// an item with the same `auto_key` exists or the month has no row; any
/// dismissal tombstone is cleared so a fresh receipt re-surfaces cleanly.
pub async fn record_receipt_item(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    month: &str,
    auto_key: &str,
    label: &str,
    amount: f64,
) -> Result<(), ApiError> {
    let item_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM month_items WHERE month = ? AND auto_key = ?)",
    )
    .bind(month)
    .bind(auto_key)
    .fetch_one(&mut **tx)
    .await?;
    let month_exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM month_stats WHERE month = ?)")
            .bind(month)
            .fetch_one(&mut **tx)
            .await?;
    if !item_exists && month_exists {
        sqlx::query(
            "INSERT INTO month_items (month, category, label, amount, auto_key, created_at) \
             VALUES (?, 'adjustment', ?, ?, ?, ?)",
        )
        .bind(month)
        .bind(label)
        .bind(amount)
        .bind(auto_key)
        .bind(now_timestamp())
        .execute(&mut **tx)
        .await?;
    }
    sqlx::query("DELETE FROM month_item_dismissals WHERE month = ? AND auto_key = ?")
        .bind(month)
        .bind(auto_key)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

/// The HS 活期 cash row receipts bank into by default (the workbook's HS
/// account) — NULL when no such manual asset exists.
pub async fn hs_cash_asset_id(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
) -> Result<Option<i64>, ApiError> {
    Ok(sqlx::query_scalar(
        "SELECT id FROM manual_assets WHERE kind = 'cash' AND label = 'HS' LIMIT 1",
    )
    .fetch_optional(&mut **tx)
    .await?)
}

pub const DEPOSIT_COLUMNS: &str = "id, label, bank, principal, rate, interest, start_date, \
     end_date, received_at, credited_asset_id, credited_amount, note1, note2, sort_order";

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

pub fn row_to_deposit(row: &SqliteRow, _today: chrono::NaiveDate) -> Result<Deposit, ApiError> {
    let end_date: String = row.try_get("end_date")?;
    let parsed = chrono::NaiveDate::parse_from_str(&end_date, "%Y-%m-%d")
        .map_err(|_| ApiError::Conflict(format!("stored end_date {end_date} is not valid")))?;
    let principal: Option<f64> = row.try_get("principal")?;
    let interest: Option<f64> = row.try_get("interest")?;
    let received_at: Option<String> = row.try_get("received_at")?;
    Ok(Deposit {
        id: row.try_get("id")?,
        label: row.try_get("label")?,
        bank: row.try_get("bank")?,
        principal,
        rate: row.try_get("rate")?,
        interest,
        start_date: row.try_get("start_date")?,
        end_date,
        received_at: received_at.clone(),
        note1: row.try_get("note1")?,
        note2: row.try_get("note2")?,
        sort_order: row.try_get("sort_order")?,
        total: deposit_total(principal, interest),
        // A deposit stays "active" until 收訖 — even past its end date.
        status: if received_at.is_some() {
            DepositStatus::End
        } else {
            DepositStatus::Active
        },
        end_year: parsed.year(),
        end_month: parsed.month(),
    })
}

pub const FAMILY_DEPOSIT_COLUMNS: &str = "id, holder, label, bank, principal, interest, \
     start_date, end_date, received_at, note, sort_order";

/// Like `row_to_deposit` minus the bank-in columns family deposits don't have;
/// `status` derives from `received_at` only (never auto-ended by date).
pub fn row_to_family_deposit(row: &SqliteRow) -> Result<FamilyDeposit, ApiError> {
    let end_date: String = row.try_get("end_date")?;
    let parsed = chrono::NaiveDate::parse_from_str(&end_date, "%Y-%m-%d")
        .map_err(|_| ApiError::Conflict(format!("stored end_date {end_date} is not valid")))?;
    let principal: Option<f64> = row.try_get("principal")?;
    let interest: Option<f64> = row.try_get("interest")?;
    let received_at: Option<String> = row.try_get("received_at")?;
    Ok(FamilyDeposit {
        id: row.try_get("id")?,
        holder: row.try_get("holder")?,
        label: row.try_get("label")?,
        bank: row.try_get("bank")?,
        principal,
        interest,
        start_date: row.try_get("start_date")?,
        end_date,
        received_at: received_at.clone(),
        note: row.try_get("note")?,
        sort_order: row.try_get("sort_order")?,
        total: deposit_total(principal, interest),
        status: if received_at.is_some() {
            DepositStatus::End
        } else {
            DepositStatus::Active
        },
        end_year: parsed.year(),
        end_month: parsed.month(),
    })
}

/// `next_pay_date` comes from a scalar subquery on non-received coupons.
pub const BOND_SELECT: &str = "SELECT b.id, b.label, b.issue_no, b.principal, b.maturity_date, \
     b.received_at, b.note, b.sort_order, (SELECT MIN(c.pay_date) FROM bond_coupons c \
     WHERE c.bond_id = b.id AND c.received_amount IS NULL) AS next_pay_date FROM bonds b";

pub fn row_to_bond(row: &SqliteRow, today: chrono::NaiveDate) -> Result<Bond, ApiError> {
    let maturity_date: String = row.try_get("maturity_date")?;
    let parsed = chrono::NaiveDate::parse_from_str(&maturity_date, "%Y-%m-%d").map_err(|_| {
        ApiError::Conflict(format!("stored maturity_date {maturity_date} is not valid"))
    })?;
    Ok(Bond {
        id: row.try_get("id")?,
        label: row.try_get("label")?,
        issue_no: row.try_get("issue_no")?,
        principal: row.try_get("principal")?,
        maturity_date,
        received_at: row.try_get("received_at")?,
        note: row.try_get("note")?,
        sort_order: row.try_get("sort_order")?,
        status: if bond_active(parsed, today) {
            BondStatus::Active
        } else {
            BondStatus::Matured
        },
        next_pay_date: row.try_get("next_pay_date")?,
    })
}

/// `principal` is joined from the parent bond so `expected` can be derived.
pub const COUPON_SELECT: &str = "SELECT c.id, c.bond_id, c.pay_date, c.fixing_date, \
     c.annual_rate, c.per_10k, c.received_amount, c.note, b.principal \
     FROM bond_coupons c JOIN bonds b ON b.id = c.bond_id";

pub fn row_to_coupon(row: &SqliteRow) -> Result<BondCoupon, ApiError> {
    let annual_rate: Option<f64> = row.try_get("annual_rate")?;
    let per_10k: Option<f64> = row.try_get("per_10k")?;
    let received_amount: Option<f64> = row.try_get("received_amount")?;
    let principal: f64 = row.try_get("principal")?;
    let expected = coupon_expected(per_10k, principal);
    Ok(BondCoupon {
        id: row.try_get("id")?,
        bond_id: row.try_get("bond_id")?,
        pay_date: row.try_get("pay_date")?,
        fixing_date: row.try_get("fixing_date")?,
        annual_rate,
        per_10k,
        received_amount,
        note: row.try_get("note")?,
        status: coupon_status(received_amount, annual_rate, per_10k),
        expected,
        variance: coupon_variance(received_amount, expected),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connect_memory;
    use std::collections::BTreeSet;

    /// The doc is generated from the same `routes!` registrations that serve
    /// requests, so an undocumented endpoint is impossible — this instead pins
    /// the exact surface so a route change always updates this list.
    #[tokio::test]
    async fn openapi_documents_every_api_operation() {
        let pool = connect_memory().await.expect("memory db");
        let (_router, api) = api_router(AppState { pool });

        let doc: serde_json::Value =
            serde_json::from_str(&api.to_json().expect("openapi serializes")).unwrap();
        let actual: BTreeSet<(String, String)> = doc["paths"]
            .as_object()
            .expect("paths object")
            .iter()
            .flat_map(|(path, item)| {
                item.as_object()
                    .expect("path item")
                    .keys()
                    .map(|method| (path.clone(), method.clone()))
                    .collect::<Vec<_>>()
            })
            .collect();

        let expected: BTreeSet<(String, String)> = [
            ("/api/aia/events", "get"),
            ("/api/aia/events", "post"),
            ("/api/aia/events/{id}", "delete"),
            ("/api/aia/policies", "get"),
            ("/api/aia/policies", "post"),
            ("/api/aia/policies/{id}", "delete"),
            ("/api/aia/policies/{id}", "patch"),
            ("/api/aia/rate", "patch"),
            ("/api/aia/summary", "get"),
            ("/api/bonds", "get"),
            ("/api/bonds", "post"),
            ("/api/bonds/summary", "get"),
            ("/api/bonds/{id}", "delete"),
            ("/api/bonds/{id}", "patch"),
            ("/api/bonds/{id}/receive", "post"),
            ("/api/bonds/{id}/unreceive", "post"),
            ("/api/coupons", "get"),
            ("/api/coupons", "post"),
            ("/api/coupons/{id}", "delete"),
            ("/api/coupons/{id}", "patch"),
            ("/api/deposits", "get"),
            ("/api/deposits", "post"),
            ("/api/deposits/summary", "get"),
            ("/api/deposits/{id}", "delete"),
            ("/api/deposits/{id}", "patch"),
            ("/api/deposits/{id}/receive", "post"),
            ("/api/deposits/{id}/unreceive", "post"),
            ("/api/dividends", "get"),
            ("/api/dividends", "post"),
            ("/api/dividends/summary", "get"),
            ("/api/dividends/{id}", "delete"),
            ("/api/dividends/{id}", "patch"),
            ("/api/family/deposits", "get"),
            ("/api/family/deposits", "post"),
            ("/api/family/deposits/summary", "get"),
            ("/api/family/deposits/{id}", "delete"),
            ("/api/family/deposits/{id}", "patch"),
            ("/api/family/deposits/{id}/receive", "post"),
            ("/api/family/deposits/{id}/unreceive", "post"),
            ("/api/family/holders/{holder}/note", "put"),
            ("/api/forecast", "get"),
            ("/api/forecast-items/{id}", "delete"),
            ("/api/forecast-items/{id}", "patch"),
            ("/api/forecast-items/{id}/convert", "post"),
            ("/api/forecast/{ym}/items", "post"),
            ("/api/ibkr", "get"),
            ("/api/ibkr", "patch"),
            ("/api/manual-assets", "get"),
            ("/api/manual-assets", "post"),
            ("/api/manual-assets/{id}", "delete"),
            ("/api/manual-assets/{id}", "patch"),
            ("/api/month-items/{id}", "delete"),
            ("/api/month-items/{id}", "patch"),
            ("/api/months", "get"),
            ("/api/months/settings", "get"),
            ("/api/months/settings", "patch"),
            ("/api/months/summary", "get"),
            ("/api/months/{ym}", "delete"),
            ("/api/months/{ym}", "get"),
            ("/api/months/{ym}", "patch"),
            ("/api/months/{ym}/items", "post"),
            ("/api/months/{ym}/items/dismiss", "post"),
            ("/api/mpf", "get"),
            ("/api/mpf/accounts", "post"),
            ("/api/mpf/accounts/{id}", "delete"),
            ("/api/mpf/accounts/{id}", "patch"),
            ("/api/mpf/history/{id}", "delete"),
            ("/api/mpf/note", "patch"),
            ("/api/overview", "get"),
            ("/api/stocks", "get"),
            ("/api/stocks", "post"),
            ("/api/stocks/order", "post"),
            ("/api/stocks/prices", "post"),
            ("/api/stocks/{id}", "delete"),
            ("/api/stocks/{id}", "patch"),
            ("/api/summary", "get"),
            ("/api/summary/yearly", "get"),
            ("/api/summary/yearly/{market}/{year}", "patch"),
            ("/api/summary/yearly/{market}/{year}/freeze", "post"),
            ("/api/trades", "get"),
            ("/api/trades", "post"),
            ("/api/trades/{id}", "delete"),
            ("/api/trades/{id}", "patch"),
            ("/api/year-review", "get"),
            ("/api/year-review/{year}", "patch"),
        ]
        .iter()
        .map(|(path, method)| (path.to_string(), method.to_string()))
        .collect();

        assert_eq!(actual, expected);
    }
}
