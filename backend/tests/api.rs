use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;
use wealth_backend::db;
use wealth_backend::routes::{api_router, AppState};

async fn app() -> Router {
    let pool = db::connect_memory().await.expect("in-memory database");
    Router::new().nest("/api", api_router(AppState { pool }))
}

async fn send(app: &Router, method: &str, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
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

async fn create_stock(app: &Router, market: &str, code: &str, sector: Option<&str>) -> i64 {
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

fn approx(value: &Value, expected: f64) {
    let actual = value
        .as_f64()
        .unwrap_or_else(|| panic!("not a number: {value}"));
    assert!(
        (actual - expected).abs() <= 1e-6 * actual.abs().max(expected.abs()).max(1.0),
        "expected {expected}, got {actual}"
    );
}

// --- 4.1 stocks ---

#[tokio::test]
async fn stock_crud_and_conflicts() {
    let app = app().await;

    let id = create_stock(&app, "HK", "中國銀行", Some("Banks - Diversified")).await;

    let (status, body) = send(
        &app,
        "POST",
        "/api/stocks",
        Some(json!({ "market": "HK", "code": "中國銀行" })),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "body = {body}");

    // Same code in the other market is a different stock.
    let (status, _) = send(
        &app,
        "POST",
        "/api/stocks",
        Some(json!({ "market": "US", "code": "中國銀行" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    // Manual market data, including the price timestamp.
    let (status, body) = send(
        &app,
        "PATCH",
        &format!("/api/stocks/{id}"),
        Some(
            json!({ "manual_price": 5.91, "pe": 6.92, "eps": 0.85, "high52": 6.14, "low52": 4.1 }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    approx(&body["manual_price"], 5.91);
    approx(&body["pe"], 6.92);
    assert!(body["price_updated_at"].is_string());

    let (status, body) = send(&app, "GET", "/api/stocks?market=HK", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().expect("array").len(), 1);

    let (status, _) = send(&app, "PATCH", "/api/stocks/9999", Some(json!({}))).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, _) = send(&app, "DELETE", &format!("/api/stocks/{id}"), None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn stock_with_trades_cannot_be_deleted() {
    let app = app().await;
    let id = create_stock(&app, "HK", "港燈", None).await;
    let (status, _) = send(
        &app,
        "POST",
        "/api/trades",
        Some(json!({
            "stock_id": id, "trade_type": "BUY", "trade_date": "2024-04-05",
            "shares": 7000, "unit_price": 4.67, "total": 32895.78
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    let (status, body) = send(&app, "DELETE", &format!("/api/stocks/{id}"), None).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert!(body["message"].as_str().expect("message").contains("港燈"));
}

#[tokio::test]
async fn stock_order_can_be_reordered_per_market() {
    let app = app().await;
    let first = create_stock(&app, "HK", "第一", None).await;
    let second = create_stock(&app, "HK", "第二", None).await;
    let third = create_stock(&app, "HK", "第三", None).await;

    let (status, body) = send(
        &app,
        "POST",
        "/api/stocks/order",
        Some(json!({ "market": "HK", "stock_ids": [third, first, second] })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body = {body}");
    let codes: Vec<&str> = body
        .as_array()
        .expect("stock array")
        .iter()
        .map(|stock| stock["code"].as_str().expect("code"))
        .collect();
    assert_eq!(codes, vec!["第三", "第一", "第二"]);

    let (_, body) = send(&app, "GET", "/api/summary?market=HK", None).await;
    let summary_codes: Vec<&str> = body["stocks"]
        .as_array()
        .expect("summary stocks")
        .iter()
        .map(|stock| stock["code"].as_str().expect("code"))
        .collect();
    assert_eq!(summary_codes, vec!["第三", "第一", "第二"]);

    let (status, _) = send(
        &app,
        "POST",
        "/api/stocks/order",
        Some(json!({ "market": "HK", "stock_ids": [first, first, second] })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

// --- 4.2 trades ---

#[tokio::test]
async fn trade_create_filter_edit_delete() {
    let app = app().await;
    create_stock(&app, "HK", "中移動", Some("Telecom Services")).await;
    create_stock(&app, "HK", "港燈", Some("Utilities")).await;

    // HK input: buy total given, fee derived.
    let (status, body) = send(
        &app,
        "POST",
        "/api/trades",
        Some(json!({
            "market": "HK", "code": "中移動", "trade_type": "BUY", "trade_date": "2025-06-06",
            "shares": 500, "unit_price": 87.55, "total": 43912.16
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "body = {body}");
    let trade_id = body["id"].as_i64().expect("id");
    approx(&body["fee"], 137.16);
    approx(&body["unit_price_incl_fee"], 87.82432);

    // US input on a US stock: fee given, total derived.
    create_stock(&app, "US", "VOO", None).await;
    let (status, body) = send(
        &app,
        "POST",
        "/api/trades",
        Some(json!({
            "market": "US", "code": "VOO", "trade_type": "BUY", "trade_date": "2026-06-02",
            "shares": 3, "unit_price": 696.04, "fee": 1.000009
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "body = {body}");
    approx(&body["total"], 2089.120009);
    approx(&body["unit_price_incl_fee"], 696.373336333);

    // Another HK trade, on a different stock and year, for the filters.
    let (status, _) = send(
        &app,
        "POST",
        "/api/trades",
        Some(json!({
            "market": "HK", "code": "港燈", "trade_type": "BUY", "trade_date": "2024-04-05",
            "shares": 7000, "unit_price": 4.67, "total": 32895.78
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    let (_, body) = send(&app, "GET", "/api/trades?market=HK&code=中移動", None).await;
    let rows = body.as_array().expect("array");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["code"], "中移動");

    let (_, body) = send(
        &app,
        "GET",
        "/api/trades?market=HK&from=2025-01-01&to=2025-12-31",
        None,
    )
    .await;
    let rows = body.as_array().expect("array");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["trade_date"], "2025-06-06");

    let (_, body) = send(&app, "GET", "/api/trades?market=HK&order=desc", None).await;
    let rows = body.as_array().expect("array");
    assert_eq!(rows[0]["trade_date"], "2025-06-06");

    // Editing the buy total re-derives fee and 平均單價.
    let (status, body) = send(
        &app,
        "PATCH",
        &format!("/api/trades/{trade_id}"),
        Some(json!({ "total": 43882.16 })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body = {body}");
    approx(&body["fee"], 107.16);
    approx(&body["unit_price_incl_fee"], 87.76432);

    let (status, _) = send(&app, "DELETE", &format!("/api/trades/{trade_id}"), None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = send(&app, "DELETE", &format!("/api/trades/{trade_id}"), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

// --- 4.3 summary ---

#[tokio::test]
async fn summary_recomputes_after_edit() {
    let app = app().await;
    let id = create_stock(&app, "HK", "中國銀行", Some("Banks - Diversified")).await;
    for (date, shares, unit_price, total) in [
        ("2023-05-23", 30000.0, 3.2, 96523.15),
        ("2024-01-24", 18000.0, 2.97, 53731.5),
        ("2024-02-02", 20000.0, 2.91, 58491.99),
    ] {
        let (status, _) = send(
            &app,
            "POST",
            "/api/trades",
            Some(json!({
                "stock_id": id, "trade_type": "BUY", "trade_date": date,
                "shares": shares, "unit_price": unit_price, "total": total
            })),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
    }
    send(
        &app,
        "PATCH",
        &format!("/api/stocks/{id}"),
        Some(json!({ "manual_price": 5.91 })),
    )
    .await;

    let (status, body) = send(&app, "GET", "/api/summary?market=HK", None).await;
    assert_eq!(status, StatusCode::OK);
    let row = &body["stocks"][0];
    approx(&row["shares_held"], 68000.0);
    approx(&row["total_buy_cost"], 208746.64);
    approx(&row["weighted_avg_buy_price"], 3.069803529);
    approx(&row["market_value"], 401880.0);
    approx(&row["unrealized_amount"], 193133.36);
    approx(&row["unrealized_return"], 0.9252046404);
    approx(&body["totals"]["buy_cost"], 208746.64);
    assert_eq!(body["sectors"][0]["sector"], "Banks - Diversified");
    assert!(body["average_price_definition"]
        .as_str()
        .expect("definition")
        .contains("shares bought"));

    // Editing a trade changes the summary with no extra recalculation step.
    let (_, trades) = send(&app, "GET", "/api/trades?market=HK", None).await;
    let first = trades[0]["id"].as_i64().expect("id");
    let (status, _) = send(
        &app,
        "PATCH",
        &format!("/api/trades/{first}"),
        Some(json!({ "shares": 20000 })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (_, body) = send(&app, "GET", "/api/summary?market=HK", None).await;
    let row = &body["stocks"][0];
    approx(&row["shares_held"], 58000.0);
    approx(&row["total_buy_cost"], 208746.64);
    approx(&row["weighted_avg_buy_price"], 3.5990800);
}

#[tokio::test]
async fn summary_marks_stocks_without_a_price_as_excluded() {
    let app = app().await;
    let priced = create_stock(&app, "HK", "港燈", Some("Utilities")).await;
    create_stock(&app, "HK", "港交所", Some("Financial Services")).await;
    send(
        &app,
        "POST",
        "/api/trades",
        Some(json!({
            "stock_id": priced, "trade_type": "BUY", "trade_date": "2024-04-05",
            "shares": 7000, "unit_price": 4.67, "total": 32895.78
        })),
    )
    .await;
    send(
        &app,
        "PATCH",
        &format!("/api/stocks/{priced}"),
        Some(json!({ "manual_price": 6.32 })),
    )
    .await;

    let (_, body) = send(&app, "GET", "/api/summary?market=HK", None).await;
    let excluded = body["totals"]["excluded_codes"].as_array().expect("array");
    assert_eq!(excluded.len(), 1);
    assert_eq!(excluded[0], "港交所");

    let unpriced = body["stocks"]
        .as_array()
        .expect("array")
        .iter()
        .find(|row| row["code"] == "港交所")
        .expect("unpriced stock is listed");
    assert!(unpriced["market_value"].is_null());
    assert!(unpriced["unrealized_amount"].is_null());
    approx(&unpriced["shares_held"], 0.0);
}

// --- 4.4 error mapping ---

#[tokio::test]
async fn errors_carry_status_and_field_messages() {
    let app = app().await;
    create_stock(&app, "HK", "港燈", None).await;

    let (status, body) = send(
        &app,
        "POST",
        "/api/trades",
        Some(json!({
            "market": "HK", "code": "港燈", "trade_type": "BUY", "trade_date": "2026/13/45",
            "shares": 100, "unit_price": 1.0, "total": 100.0
        })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "validation");
    assert!(body["fields"]
        .as_array()
        .expect("fields")
        .iter()
        .any(|f| f["field"] == "trade_date"));

    let (status, body) = send(
        &app,
        "POST",
        "/api/trades",
        Some(json!({
            "market": "HK", "code": "不存在", "trade_type": "BUY", "trade_date": "2026-01-02",
            "shares": 100, "unit_price": 1.0, "total": 100.0
        })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["fields"]
        .as_array()
        .expect("fields")
        .iter()
        .any(|f| f["field"] == "code"));

    let (status, body) = send(
        &app,
        "POST",
        "/api/stocks",
        Some(json!({ "market": "HK", "code": "港燈" })),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["error"], "conflict");

    let (status, _) = send(&app, "GET", "/api/summary?market=JP", None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let (status, _) = send(&app, "GET", "/api/trades?stock_id=9999", None).await;
    assert_eq!(status, StatusCode::OK);
}

// --- 4.5 bulk price upload ---

async fn create_stock_with_ticker(app: &Router, market: &str, code: &str, ticker: &str) -> i64 {
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

#[tokio::test]
async fn price_upload_updates_matches_and_reports_the_rest() {
    let app = app().await;
    create_stock_with_ticker(&app, "HK", "中國銀行", "3988").await;
    create_stock_with_ticker(&app, "HK", "ＦＧ恆生紅利", "3031").await;
    create_stock_with_ticker(&app, "US", "VOO", "VOO").await;
    create_stock_with_ticker(&app, "US", "BRK.B", "BRK.B").await;

    let (status, body) = send(
        &app,
        "POST",
        "/api/stocks/prices",
        Some(json!({ "stocks": [
            { "symbol": "3988.HK", "price": 5.975 },
            { "symbol": "VOO", "price": 699.3 },
            { "symbol": "BRK-B", "price": 514.95 },
            { "symbol": "1310.HK", "price": 5.47 },
            { "price": 1.0 },
            { "symbol": "BE", "price": 0 }
        ]})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body = {body}");

    let updated = body["updated"].as_array().expect("updated");
    assert_eq!(updated.len(), 3);
    assert_eq!(body["unmatched"], json!(["1310.HK"]));
    assert_eq!(body["invalid"].as_array().expect("invalid").len(), 2);
    assert_eq!(body["not_updated"], json!(["HK ＦＧ恆生紅利"]));

    // BRK-B resolved to BRK.B; every updated row carries a fresh timestamp.
    let (status, stocks) = send(&app, "GET", "/api/stocks?market=US", None).await;
    assert_eq!(status, StatusCode::OK);
    let brk = stocks
        .as_array()
        .expect("stocks")
        .iter()
        .find(|s| s["code"] == "BRK.B")
        .expect("BRK.B");
    approx(&brk["manual_price"], 514.95);
    assert!(brk["price_updated_at"].is_string());

    let (status, stocks) = send(&app, "GET", "/api/stocks?market=HK", None).await;
    assert_eq!(status, StatusCode::OK);
    let boc = stocks
        .as_array()
        .expect("stocks")
        .iter()
        .find(|s| s["code"] == "中國銀行")
        .expect("中國銀行");
    approx(&boc["manual_price"], 5.975);
}

#[tokio::test]
async fn price_upload_rejects_a_body_without_a_stocks_array() {
    let app = app().await;
    let (status, body) = send(
        &app,
        "POST",
        "/api/stocks/prices",
        Some(json!({ "foo": 1 })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "validation");
}

// --- 4.6 deposits ---

async fn create_deposit(app: &Router, body: Value) -> Value {
    let (status, body) = send(app, "POST", "/api/deposits", Some(body)).await;
    assert_eq!(status, StatusCode::CREATED, "body = {body}");
    body
}

#[tokio::test]
async fn deposit_crud_and_filters() {
    let app = app().await;

    // An active deposit ending far in the future.
    let active = create_deposit(
        &app,
        json!({
            "label": "SC-9632", "bank": "SC", "principal": 110000, "rate": 0.028,
            "interest": 993, "end_date": "2099-10-12", "note2": "rate schedule"
        }),
    )
    .await;
    let active_id = active["id"].as_i64().expect("id");
    assert_eq!(active["status"], "ACTIVE");
    assert_eq!(active["bank"], "SC");
    assert_eq!(active["end_year"], 2099);
    assert_eq!(active["end_month"], 10);
    approx(&active["total"], 110993.0);
    assert_eq!(active["sort_order"], 1);

    // An ended deposit and an interest-only row.
    create_deposit(
        &app,
        json!({ "label": "HS-74", "principal": 60000, "rate": 0.012,
                "interest": 362.96, "end_date": "2026-01-05" }),
    )
    .await;
    create_deposit(
        &app,
        json!({ "interest": 539.25, "end_date": "2026-05-14" }),
    )
    .await;

    let (_, body) = send(&app, "GET", "/api/deposits", None).await;
    assert_eq!(body.as_array().expect("array").len(), 3);

    let (_, body) = send(&app, "GET", "/api/deposits?status=active", None).await;
    let rows = body.as_array().expect("array");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["label"], "SC-9632");

    let (_, body) = send(&app, "GET", "/api/deposits?status=ended&year=2026", None).await;
    let rows = body.as_array().expect("array");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[1]["interest"], 539.25);

    let (status, body) = send(&app, "GET", "/api/deposits?status=soon", None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body = {body}");

    // PATCH merges; a null clears an optional field.
    let (status, body) = send(
        &app,
        "PATCH",
        &format!("/api/deposits/{active_id}"),
        Some(json!({ "interest": 1000, "note2": null })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body = {body}");
    approx(&body["interest"], 1000.0);
    approx(&body["total"], 111000.0);
    assert!(body["note2"].is_null());

    let (status, _) = send(&app, "DELETE", &format!("/api/deposits/{active_id}"), None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = send(&app, "DELETE", &format!("/api/deposits/{active_id}"), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn deposit_validation_errors_name_the_fields() {
    let app = app().await;

    let (status, body) = send(
        &app,
        "POST",
        "/api/deposits",
        Some(json!({ "label": "SC-1", "end_date": "2026/13/45" })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["fields"]
        .as_array()
        .expect("fields")
        .iter()
        .any(|f| f["field"] == "end_date"));

    let (status, body) = send(
        &app,
        "POST",
        "/api/deposits",
        Some(json!({ "end_date": "2099-01-01" })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["fields"]
        .as_array()
        .expect("fields")
        .iter()
        .any(|f| f["field"] == "label"));

    let (status, body) = send(
        &app,
        "POST",
        "/api/deposits",
        Some(json!({ "label": "SC-1", "principal": -5000, "end_date": "2099-01-01" })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["fields"]
        .as_array()
        .expect("fields")
        .iter()
        .any(|f| f["field"] == "principal"));

    let (status, body) = send(
        &app,
        "POST",
        "/api/deposits",
        Some(json!({ "label": "SC-1", "rate": 3.0, "end_date": "2099-01-01" })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["fields"]
        .as_array()
        .expect("fields")
        .iter()
        .any(|f| f["field"] == "rate"));
}

#[tokio::test]
async fn deposit_summary_reports_upcoming_rollups_and_year_tables() {
    let app = app().await;

    // Two active deposits in different months and banks, one ended, one
    // interest-only ended row.
    create_deposit(
        &app,
        json!({ "label": "SC-9632", "bank": "SC", "principal": 110000, "rate": 0.028,
                "interest": 993, "end_date": "2099-10-12" }),
    )
    .await;
    create_deposit(
        &app,
        json!({ "label": "HS-88", "bank": "HS", "principal": 80000, "rate": 0.03,
                "interest": 604.93, "end_date": "2099-11-17" }),
    )
    .await;
    create_deposit(
        &app,
        json!({ "label": "HS-74", "bank": "HS", "principal": 60000, "interest": 362.96,
                "end_date": "2026-01-05" }),
    )
    .await;
    create_deposit(&app, json!({ "interest": 15.27, "end_date": "2026-04-20" })).await;

    let (status, body) = send(&app, "GET", "/api/deposits/summary", None).await;
    assert_eq!(status, StatusCode::OK, "body = {body}");

    let upcoming = body["upcoming"].as_array().expect("upcoming");
    assert_eq!(upcoming.len(), 2);
    assert_eq!(upcoming[0]["end_date"], "2099-10-12");
    approx(&body["active_totals"]["principal"], 190000.0);

    let months = body["months"].as_array().expect("months");
    assert_eq!(months.len(), 2);
    assert_eq!(months[0]["month"], 10);
    approx(&months[0]["principal"], 110000.0);
    approx(&months[0]["interest"], 993.0);
    approx(&months[0]["total"], 110993.0);

    let banks = body["banks"].as_array().expect("banks");
    assert_eq!(banks.len(), 2);
    assert_eq!(banks[0]["bank"], "SC");
    approx(&banks[0]["principal"], 110000.0);
    assert_eq!(banks[1]["bank"], "HS");

    // Year tables cover every deposit ending that year, ended or not.
    let years = body["years"].as_array().expect("years");
    let y2026 = years
        .iter()
        .find(|y| y["year"] == 2026)
        .expect("2026 table");
    let jan = &y2026["months"][0];
    approx(&jan["interest"], 362.96);
    approx(&jan["payout"], 60362.96);
    approx(&jan["total"], 60725.92);
    let apr = &y2026["months"][3];
    approx(&apr["interest"], 15.27);

    let history_years = body["history_years"].as_array().expect("history_years");
    assert!(history_years.iter().any(|y| *y == 2026));
    assert!(history_years.iter().any(|y| *y == 2099));
}
