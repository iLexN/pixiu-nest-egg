//! Shared helpers for the HTTP-surface integration tests: app wiring,
//! request/response plumbing, seeded rows, and entity creators. Tests live in
//! the per-domain `tests/*.rs` binaries; helpers live here only.
// Each binary compiles this module fresh and uses a subset — dead code is
// expected, not drift.
#![allow(dead_code)]

use axum::body::Body;
use axum::http::Request;
use http_body_util::BodyExt;
use tower::ServiceExt;
use wealth_backend::db;
use wealth_backend::routes::{AppState, api_router};

pub use axum::Router;
pub use axum::http::StatusCode;
pub use serde_json::{Value, json};

pub async fn app() -> Router {
    app_with_pool().await.0
}

pub async fn app_with_pool() -> (Router, sqlx::SqlitePool) {
    let pool = db::connect_memory().await.expect("in-memory database");
    // Route paths already carry the /api prefix.
    let (api, _openapi) = api_router(AppState { pool: pool.clone() });
    (Router::new().merge(api), pool)
}

/// A 活期 bank row seeded directly; 'HS' is flagged the receipt default,
/// matching the migrated production data.
pub async fn seed_cash(pool: &sqlx::SqlitePool, label: &str, amount: f64) -> i64 {
    let is_default = label == "HS";
    sqlx::query_scalar(
        "INSERT INTO cash_accounts (label, currency, role, amount, is_default_receipt, \
         sort_order, updated_at) VALUES (?, 'HKD', 'bank', ?, ?, 1, '2026-01-01T00:00:00') \
         RETURNING id",
    )
    .bind(label)
    .bind(amount)
    .bind(is_default)
    .fetch_one(pool)
    .await
    .expect("cash account")
}

pub async fn cash_amount(pool: &sqlx::SqlitePool, id: i64) -> f64 {
    sqlx::query_scalar("SELECT amount FROM cash_accounts WHERE id = ?")
        .bind(id)
        .fetch_one(pool)
        .await
        .expect("cash amount")
}

pub async fn send(
    app: &Router,
    method: &str,
    uri: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let request = Request::builder().method(method).uri(uri);
    let request = match body {
        Some(body) => request
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .expect("request"),
        None => request.body(Body::empty()).expect("request"),
    };
    let response = app.clone().oneshot(request).await.expect("response");
    let status = response.status();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    let value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).expect("json body")
    };
    (status, value)
}

pub async fn create_stock(app: &Router, market: &str, code: &str, sector: Option<&str>) -> i64 {
    let (status, body) = send(
        app,
        "POST",
        "/api/stocks",
        Some(json!({ "market": market, "code": code, "sector": sector })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "body = {body}");
    body["id"].as_i64().expect("stock id")
}

pub fn approx(value: &Value, expected: f64) {
    let actual = value
        .as_f64()
        .unwrap_or_else(|| panic!("not a number: {value}"));
    assert!(
        (actual - expected).abs() <= 1e-6 * actual.abs().max(expected.abs()).max(1.0),
        "expected {expected}, got {actual}"
    );
}

// --- 4.1 stocks ---

pub async fn create_stock_with_ticker(app: &Router, market: &str, code: &str, ticker: &str) -> i64 {
    let (status, body) = send(
        app,
        "POST",
        "/api/stocks",
        Some(json!({ "market": market, "code": code, "ticker": ticker })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "body = {body}");
    body["id"].as_i64().expect("stock id")
}

pub async fn create_deposit(app: &Router, body: Value) -> Value {
    let (status, body) = send(app, "POST", "/api/deposits", Some(body)).await;
    assert_eq!(status, StatusCode::CREATED, "body = {body}");
    body
}

pub async fn create_buy(app: &Router, stock_id: i64, date: &str, shares: f64, total: f64) {
    let (status, body) = send(
        app,
        "POST",
        "/api/trades",
        Some(json!({
            "stock_id": stock_id, "trade_type": "BUY", "trade_date": date,
            "shares": shares, "unit_price": 1.0, "total": total
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "body = {body}");
}

pub async fn create_family_deposit(app: &Router, body: Value) -> Value {
    let (status, body) = send(app, "POST", "/api/family/deposits", Some(body)).await;
    assert_eq!(status, StatusCode::CREATED, "body = {body}");
    body
}

/// `YYYY-MM` + n months.
pub fn ym_add(ym: &str, months: i64) -> String {
    let year: i64 = ym[..4].parse().expect("year");
    let month: i64 = ym[5..7].parse().expect("month");
    let total = year * 12 + month - 1 + months;
    format!(
        "{:04}-{:02}",
        total.div_euclid(12),
        total.rem_euclid(12) + 1
    )
}

pub async fn history_rows(pool: &sqlx::SqlitePool, market: &str) -> Vec<(String, f64, f64)> {
    sqlx::query_as(
        "SELECT recorded_on, buy_cost_priced, market_value FROM market_history \
         WHERE market = ? ORDER BY recorded_on",
    )
    .bind(market)
    .fetch_all(pool)
    .await
    .expect("market_history")
}

pub async fn history_count(pool: &sqlx::SqlitePool) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM market_history")
        .fetch_one(pool)
        .await
        .expect("count")
}

pub async fn set_price(app: &Router, stock_id: i64, price: f64) {
    let (status, body) = send(
        app,
        "PATCH",
        &format!("/api/stocks/{stock_id}"),
        Some(json!({ "manual_price": price })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body = {body}");
}

pub async fn buy_trade(app: &Router, stock_id: i64, shares: f64, total: f64) -> i64 {
    let (status, body) = send(
        app,
        "POST",
        "/api/trades",
        Some(json!({
            "stock_id": stock_id, "trade_type": "BUY", "trade_date": "2026-01-05",
            "shares": shares, "unit_price": total / shares, "total": total
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "body = {body}");
    body["id"].as_i64().expect("trade id")
}
