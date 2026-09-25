use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;
use wealth_backend::db;
use wealth_backend::routes::{AppState, api_router};

async fn app() -> Router {
    let pool = db::connect_memory().await.expect("in-memory database");
    // Route paths already carry the /api prefix.
    let (api, _openapi) = api_router(AppState { pool });
    Router::new().merge(api)
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
    assert!(
        body["average_price_definition"]
            .as_str()
            .expect("definition")
            .contains("shares bought")
    );

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
    assert!(
        body["fields"]
            .as_array()
            .expect("fields")
            .iter()
            .any(|f| f["field"] == "trade_date")
    );

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
    assert!(
        body["fields"]
            .as_array()
            .expect("fields")
            .iter()
            .any(|f| f["field"] == "code")
    );

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
    assert!(
        body["fields"]
            .as_array()
            .expect("fields")
            .iter()
            .any(|f| f["field"] == "end_date")
    );

    let (status, body) = send(
        &app,
        "POST",
        "/api/deposits",
        Some(json!({ "end_date": "2099-01-01" })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(
        body["fields"]
            .as_array()
            .expect("fields")
            .iter()
            .any(|f| f["field"] == "label")
    );

    let (status, body) = send(
        &app,
        "POST",
        "/api/deposits",
        Some(json!({ "label": "SC-1", "principal": -5000, "end_date": "2099-01-01" })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(
        body["fields"]
            .as_array()
            .expect("fields")
            .iter()
            .any(|f| f["field"] == "principal")
    );

    let (status, body) = send(
        &app,
        "POST",
        "/api/deposits",
        Some(json!({ "label": "SC-1", "rate": 3.0, "end_date": "2099-01-01" })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(
        body["fields"]
            .as_array()
            .expect("fields")
            .iter()
            .any(|f| f["field"] == "rate")
    );
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

#[tokio::test]
async fn deposit_receive_credits_cash_and_records_the_month_item() {
    let app = app().await;

    // The cash row the returned money credits into, and the month the
    // dep-end item lands in.
    let (status, asset) = send(
        &app,
        "POST",
        "/api/manual-assets",
        Some(json!({ "label": "渣打", "kind": "cash", "amount": 1000.0 })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "body = {asset}");
    let asset_id = asset["id"].as_i64().expect("asset id");
    let (status, _) = send(
        &app,
        "PATCH",
        "/api/months/2099-10",
        Some(json!({ "start_cash": 5000 })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let deposit = create_deposit(
        &app,
        json!({ "label": "SC-9632", "bank": "SC", "principal": 110000,
                "interest": 993, "end_date": "2099-10-12" }),
    )
    .await;
    let id = deposit["id"].as_i64().expect("id");
    assert_eq!(deposit["status"], "ACTIVE");
    assert!(deposit["received_at"].is_null());

    // Before 收訖 the deposit's interest previews but does not count.
    let (_, detail) = send(&app, "GET", "/api/months/2099-10", None).await;
    approx(&detail["month"]["interest"], 0.0);
    let auto = detail["interest_auto"].as_array().expect("interest_auto");
    assert_eq!(auto.len(), 1);
    assert_eq!(auto[0]["received"], false);

    let (status, body) = send(
        &app,
        "POST",
        &format!("/api/deposits/{id}/receive"),
        Some(json!({ "credit_asset_id": asset_id })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body = {body}");
    assert_eq!(body["status"], "END");
    assert!(body["received_at"].is_string());

    let (_, assets) = send(&app, "GET", "/api/manual-assets", None).await;
    approx(&assets[0]["amount"], 111993.0);

    let (_, detail) = send(&app, "GET", "/api/months/2099-10", None).await;
    approx(&detail["month"]["interest"], 993.0);
    let dep_end_key = format!("dep-end:{id}");
    let items = detail["items"].as_array().expect("items");
    let dep_end = items
        .iter()
        .find(|item| item["auto_key"] == dep_end_key)
        .expect("dep-end item");
    approx(&dep_end["amount"], 110993.0);

    // Re-receiving conflicts; unreceive reverses everything.
    let (status, _) = send(
        &app,
        "POST",
        &format!("/api/deposits/{id}/receive"),
        Some(json!({})),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    let (status, body) = send(&app, "POST", &format!("/api/deposits/{id}/unreceive"), None).await;
    assert_eq!(status, StatusCode::OK, "body = {body}");
    assert_eq!(body["status"], "ACTIVE");

    let (_, assets) = send(&app, "GET", "/api/manual-assets", None).await;
    approx(&assets[0]["amount"], 1000.0);
    let (_, detail) = send(&app, "GET", "/api/months/2099-10", None).await;
    approx(&detail["month"]["interest"], 0.0);
    assert!(
        detail["items"]
            .as_array()
            .expect("items")
            .iter()
            .all(|item| item["auto_key"] != dep_end_key)
    );
}

#[tokio::test]
async fn bond_receive_credits_principal_and_records_the_month_item() {
    let app = app().await;

    let (status, asset) = send(
        &app,
        "POST",
        "/api/manual-assets",
        Some(json!({ "label": "HS", "kind": "cash", "amount": 500.0 })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "body = {asset}");
    let asset_id = asset["id"].as_i64().expect("asset id");
    let (status, _) = send(
        &app,
        "PATCH",
        "/api/months/2099-12",
        Some(json!({ "start_cash": 5000 })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, bond) = send(
        &app,
        "POST",
        "/api/bonds",
        Some(json!({ "label": "silver bond", "principal": 100000,
                     "maturity_date": "2099-12-23" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "body = {bond}");
    let id = bond["id"].as_i64().expect("id");
    assert!(bond["received_at"].is_null());

    // 收訖 the principal return: credit the cash row, record the bond-end item.
    let (status, body) = send(
        &app,
        "POST",
        &format!("/api/bonds/{id}/receive"),
        Some(json!({ "credit_asset_id": asset_id })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body = {body}");
    assert!(body["received_at"].is_string());

    let (_, assets) = send(&app, "GET", "/api/manual-assets", None).await;
    approx(&assets[0]["amount"], 100500.0);
    let bond_end_key = format!("bond-end:{id}");
    let (_, detail) = send(&app, "GET", "/api/months/2099-12", None).await;
    let item = detail["items"]
        .as_array()
        .expect("items")
        .iter()
        .find(|item| item["auto_key"] == bond_end_key)
        .expect("bond-end item");
    approx(&item["amount"], 100000.0);

    // 取消收訖 reverses everything.
    let (status, body) = send(&app, "POST", &format!("/api/bonds/{id}/unreceive"), None).await;
    assert_eq!(status, StatusCode::OK, "body = {body}");
    assert!(body["received_at"].is_null());
    let (_, assets) = send(&app, "GET", "/api/manual-assets", None).await;
    approx(&assets[0]["amount"], 500.0);
    let (_, detail) = send(&app, "GET", "/api/months/2099-12", None).await;
    assert!(
        detail["items"]
            .as_array()
            .expect("items")
            .iter()
            .all(|item| item["auto_key"] != bond_end_key)
    );
}

// --- 4.7 dividends (派息) ---

async fn create_buy(app: &Router, stock_id: i64, date: &str, shares: f64, total: f64) {
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

#[tokio::test]
async fn dividend_lifecycle_freezes_its_snapshots() {
    let app = app().await;
    let stock = create_stock(&app, "HK", "中國銀行", None).await;
    create_buy(&app, stock, "2023-05-23", 30000.0, 96523.15).await;
    create_buy(&app, stock, "2024-02-15", 18000.0, 53731.5).await;
    create_buy(&app, stock, "2024-03-04", 20000.0, 58491.99).await;

    // Record an expected dividend: per-share only, snapshot derived from
    // trades on or before the pay date.
    let (status, body) = send(
        &app,
        "POST",
        "/api/dividends",
        Some(json!({
            "market": "HK", "code": "中國銀行",
            "pay_date": "2024-08-05", "per_share": 0.2
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "body = {body}");
    let id = body["id"].as_i64().expect("id");
    assert_eq!(body["status"], "PENDING");
    approx(&body["shares_held"], 68000.0);
    approx(&body["buy_cost"], 208746.64);
    approx(&body["estimated_amount"], 13600.0);
    approx(&body["yield_on_cost"], 13600.0 / 208746.64);
    assert!(body["yield_on_price"].is_null());

    // A later buy must not rewrite the recorded dividend's snapshot.
    create_buy(&app, stock, "2025-01-10", 1000.0, 5000.0).await;
    let (_, body) = send(&app, "GET", "/api/dividends?market=HK", None).await;
    let row = &body.as_array().expect("array")[0];
    approx(&row["shares_held"], 68000.0);
    approx(&row["buy_cost"], 208746.64);

    // Receiving flips the status and unlocks the second rate; the estimate
    // stays for comparison.
    let (status, body) = send(
        &app,
        "PATCH",
        &format!("/api/dividends/{id}"),
        Some(json!({ "received_amount": 13000.0, "received_price": 5.0 })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body = {body}");
    assert_eq!(body["status"], "RECEIVED");
    approx(&body["amount"], 13000.0);
    approx(&body["variance"], -600.0);
    approx(&body["yield_on_price"], 13000.0 / (5.0 * 68000.0));

    // refresh_snapshots re-derives from trades as of the pay date; moving the
    // pay date past the new buy picks it up.
    let (status, body) = send(
        &app,
        "PATCH",
        &format!("/api/dividends/{id}"),
        Some(json!({ "pay_date": "2025-02-01", "refresh_snapshots": true })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body = {body}");
    approx(&body["shares_held"], 69000.0);
    approx(&body["buy_cost"], 213746.64);

    let (_, body) = send(&app, "GET", "/api/dividends?status=received", None).await;
    assert_eq!(body.as_array().expect("array").len(), 1);
    let (_, body) = send(&app, "GET", "/api/dividends?status=pending", None).await;
    assert_eq!(body.as_array().expect("array").len(), 0);
    let (_, body) = send(&app, "GET", "/api/dividends?year=2025", None).await;
    assert_eq!(body.as_array().expect("array").len(), 1);
    let (_, body) = send(&app, "GET", "/api/dividends?market=US", None).await;
    assert_eq!(body.as_array().expect("array").len(), 0);

    // Summary: pending list and the yearly received rollup.
    let (status, body) = send(&app, "GET", "/api/dividends/summary?market=HK", None).await;
    assert_eq!(status, StatusCode::OK, "body = {body}");
    assert_eq!(body["pending"].as_array().expect("pending").len(), 0);
    let years = body["years"].as_array().expect("years");
    let bucket = years
        .iter()
        .find(|y| y["total"].as_f64().unwrap_or(0.0) > 0.0)
        .expect("received bucket");
    approx(&bucket["total"], 13000.0);
    assert_eq!(bucket["stocks"][0]["code"], "中國銀行");

    // A stock with dividends cannot be deleted.
    let (status, _) = send(&app, "DELETE", &format!("/api/stocks/{stock}"), None).await;
    assert_eq!(status, StatusCode::CONFLICT);

    let (status, _) = send(&app, "DELETE", &format!("/api/dividends/{id}"), None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = send(&app, "DELETE", &format!("/api/dividends/{id}"), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn coupon_and_dividend_receive_banks_in_and_records_the_item() {
    let app = app().await;

    // HS cash row + month rows for the receipts' months.
    let (status, hs) = send(
        &app,
        "POST",
        "/api/manual-assets",
        Some(json!({ "label": "HS", "kind": "cash", "amount": 100.0 })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "body = {hs}");
    for ym in ["2099-10", "2099-11"] {
        send(
            &app,
            "PATCH",
            &format!("/api/months/{ym}"),
            Some(json!({ "start_cash": 1.0 })),
        )
        .await;
    }

    // --- coupon: 收訖 banks into HS and stores the coupon:<id> item ---
    let (status, bond) = send(
        &app,
        "POST",
        "/api/bonds",
        Some(json!({ "label": "silver bond", "principal": 100000,
                     "maturity_date": "2099-12-23" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "body = {bond}");
    let bond_id = bond["id"].as_i64().expect("bond id");
    let (status, coupon) = send(
        &app,
        "POST",
        "/api/coupons",
        Some(json!({ "bond_id": bond_id, "pay_date": "2099-10-23",
                     "per_10k": 200.0 })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "body = {coupon}");
    let coupon_id = coupon["id"].as_i64().expect("coupon id");

    let (status, body) = send(
        &app,
        "PATCH",
        &format!("/api/coupons/{coupon_id}"),
        Some(json!({ "received_amount": 1000.0 })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body = {body}");

    let (_, assets) = send(&app, "GET", "/api/manual-assets", None).await;
    approx(&assets[0]["amount"], 1100.0);
    let coupon_key = format!("coupon:{coupon_id}");
    let (_, detail) = send(&app, "GET", "/api/months/2099-10", None).await;
    let item = detail["items"]
        .as_array()
        .expect("items")
        .iter()
        .find(|i| i["auto_key"] == coupon_key)
        .expect("coupon item");
    approx(&item["amount"], 1000.0);

    // Clearing received_amount reverses the credit and drops the item.
    let (status, _) = send(
        &app,
        "PATCH",
        &format!("/api/coupons/{coupon_id}"),
        Some(json!({ "received_amount": null })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (_, assets) = send(&app, "GET", "/api/manual-assets", None).await;
    approx(&assets[0]["amount"], 100.0);
    let (_, detail) = send(&app, "GET", "/api/months/2099-10", None).await;
    assert!(
        detail["items"]
            .as_array()
            .expect("items")
            .iter()
            .all(|i| i["auto_key"] != coupon_key)
    );

    // --- HK dividend: 收訖 banks into HS + div:<id> item ---
    let stock = create_stock(&app, "HK", "中國銀行", None).await;
    create_buy(&app, stock, "2099-10-01", 1000.0, 5000.0).await;
    let (status, div) = send(
        &app,
        "POST",
        "/api/dividends",
        Some(json!({ "market": "HK", "code": "中國銀行",
                     "pay_date": "2099-11-05", "estimated_amount": 480.0 })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "body = {div}");
    let div_id = div["id"].as_i64().expect("div id");

    let (status, body) = send(
        &app,
        "PATCH",
        &format!("/api/dividends/{div_id}"),
        Some(json!({ "received_amount": 500.0 })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body = {body}");
    let (_, assets) = send(&app, "GET", "/api/manual-assets", None).await;
    approx(&assets[0]["amount"], 600.0);
    let div_key = format!("div:{div_id}");
    let (_, detail) = send(&app, "GET", "/api/months/2099-11", None).await;
    assert!(
        detail["items"]
            .as_array()
            .expect("items")
            .iter()
            .any(|i| i["auto_key"] == div_key)
    );

    // --- US dividend: 收訖 banks into IBKR USD cash, no month item ---
    create_stock(&app, "US", "AAPL", None).await;
    let (status, div) = send(
        &app,
        "POST",
        "/api/dividends",
        Some(json!({ "market": "US", "code": "AAPL",
                     "pay_date": "2099-11-10", "estimated_amount": 200.0 })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "body = {div}");
    let us_id = div["id"].as_i64().expect("us div id");
    let (status, body) = send(
        &app,
        "PATCH",
        &format!("/api/dividends/{us_id}"),
        Some(json!({ "received_amount": 200.0 })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body = {body}");
    let (_, assets) = send(&app, "GET", "/api/manual-assets", None).await;
    approx(&assets[0]["amount"], 600.0); // HS untouched
    let (_, overview) = send(&app, "GET", "/api/overview", None).await;
    approx(&overview["ibkr"]["usd_cash"], 200.0);
    let (_, detail) = send(&app, "GET", "/api/months/2099-11", None).await;
    assert!(
        detail["items"]
            .as_array()
            .expect("items")
            .iter()
            .all(|i| i["auto_key"] != format!("div:{us_id}"))
    );

    // Un-receipt returns the IBKR cash.
    let (status, _) = send(
        &app,
        "PATCH",
        &format!("/api/dividends/{us_id}"),
        Some(json!({ "received_amount": null })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (_, overview) = send(&app, "GET", "/api/overview", None).await;
    approx(&overview["ibkr"]["usd_cash"], 0.0);
}

#[tokio::test]
async fn month_recapture_snapshots_start_cash_from_the_cash_sum() {
    let app = app().await;

    for (label, amount) in [("HS", 1000.0), ("渣打", 500.0)] {
        let (status, body) = send(
            &app,
            "POST",
            "/api/manual-assets",
            Some(json!({ "label": label, "kind": "cash", "amount": amount })),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "body = {body}");
    }

    // Creating a month stores no 月初.
    let (status, month) = send(&app, "PATCH", "/api/months/2099-03", Some(json!({}))).await;
    assert_eq!(status, StatusCode::OK, "body = {month}");
    assert!(month["start_cash"].is_null());

    // 重新擷取 fills 月初 with the live 活期 sum.
    let (status, month) = send(
        &app,
        "PATCH",
        "/api/months/2099-03",
        Some(json!({ "recapture": true })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body = {month}");
    approx(&month["start_cash"], 1500.0);

    // Editing a cash row then recapturing again refreshes 月初.
    let (_, assets) = send(&app, "GET", "/api/manual-assets", None).await;
    let hs_id = assets[0]["id"].as_i64().expect("asset id");
    let (status, _) = send(
        &app,
        "PATCH",
        &format!("/api/manual-assets/{hs_id}"),
        Some(json!({ "amount": 700.0 })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, month) = send(
        &app,
        "PATCH",
        "/api/months/2099-03",
        Some(json!({ "recapture": true })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body = {month}");
    approx(&month["start_cash"], 1200.0);

    // An explicit 月初 wins over recapture.
    let (status, month) = send(
        &app,
        "PATCH",
        "/api/months/2099-03",
        Some(json!({ "start_cash": 42.0, "recapture": true })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body = {month}");
    approx(&month["start_cash"], 42.0);
}

#[tokio::test]
async fn dividend_validation_errors_name_the_fields() {
    let app = app().await;
    create_stock(&app, "HK", "港燈", None).await;

    let (status, body) = send(
        &app,
        "POST",
        "/api/dividends",
        Some(json!({ "market": "HK", "code": "港燈", "pay_date": "2026/13/45", "per_share": 0.1 })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(
        body["fields"]
            .as_array()
            .expect("fields")
            .iter()
            .any(|f| f["field"] == "pay_date")
    );

    let (status, body) = send(
        &app,
        "POST",
        "/api/dividends",
        Some(json!({ "market": "HK", "code": "港燈", "pay_date": "2026-09-30" })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(
        body["fields"]
            .as_array()
            .expect("fields")
            .iter()
            .any(|f| f["field"] == "estimated_amount")
    );

    let (status, _) = send(
        &app,
        "POST",
        "/api/dividends",
        Some(
            json!({ "market": "HK", "code": "不存在", "pay_date": "2026-09-30", "per_share": 1.0 }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

// --- year review ---

#[tokio::test]
async fn year_review_reports_manual_figures_and_feeds_sold_pl() {
    let app = app().await;
    let stock_id = create_stock(&app, "HK", "中國銀行", None).await;
    create_buy(&app, stock_id, "2026-03-01", 100.0, 500.0).await;
    let (status, body) = send(
        &app,
        "PATCH",
        "/api/months/2026-03",
        Some(json!({ "start_cash": 10000.0, "salary": 5000.0 })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body = {body}");

    // Manual figures upsert; sold_pl may be negative.
    let (status, row) = send(
        &app,
        "PATCH",
        "/api/year-review/2026",
        Some(json!({
            "income": 120000.0,
            "invested_adjustment": 21000.0,
            "sold_pl": -1500.0,
            "bond_principal": 160000.0,
            "bond_interest": 6503.0,
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body = {row}");
    approx(&row["assets"]["income"], 120000.0);
    // invested = HK net invested (500) + the manual adjustment.
    approx(&row["investment"]["invested"], 21500.0);
    // 投資純利 = Σ interest (0 here) + sold_pl.
    approx(&row["investment"]["net_investment"], -1500.0);
    approx(&row["assets"]["bond_principal"], 160000.0);
    assert!(row["assets"]["bond_overridden"].as_bool().unwrap());

    // The seeded figures surface on the yearly summary and the Month Stat
    // yearly block's 投資純利.
    let (status, yearly) = send(&app, "GET", "/api/summary/yearly?market=HK", None).await;
    assert_eq!(status, StatusCode::OK);
    let year = yearly["years"]
        .as_array()
        .expect("years")
        .iter()
        .find(|row| row["year"] == 2026)
        .expect("2026 row");
    approx(&year["sold_pl"], -1500.0);

    let (status, summary) = send(&app, "GET", "/api/months/summary", None).await;
    assert_eq!(status, StatusCode::OK);
    let year = summary["years"]
        .as_array()
        .expect("years")
        .iter()
        .find(|row| row["year"] == 2026)
        .expect("2026 summary");
    approx(&year["net_investment"], -1500.0);

    // Clearing an override returns to the derived figure (no bonds → absent).
    let (status, row) = send(
        &app,
        "PATCH",
        "/api/year-review/2026",
        Some(json!({ "bond_principal": null, "bond_interest": null })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body = {row}");
    assert!(row["assets"]["bond_principal"].is_null());
    assert!(!row["assets"]["bond_overridden"].as_bool().unwrap());

    // raise stores an override, then clearing falls back to the salary-derived
    // figure — absent here, since no 2025 month carries a salary.
    let (status, row) = send(
        &app,
        "PATCH",
        "/api/year-review/2026",
        Some(json!({ "raise": 2500.0 })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body = {row}");
    approx(&row["investment"]["raise"], 2500.0);
    let (status, row) = send(
        &app,
        "PATCH",
        "/api/year-review/2026",
        Some(json!({ "raise": null })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body = {row}");
    assert!(row["investment"]["raise"].is_null());

    // Non-negative fields reject negatives; an empty patch is rejected.
    let (status, _) = send(
        &app,
        "PATCH",
        "/api/year-review/2026",
        Some(json!({ "income": -5.0 })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = send(&app, "PATCH", "/api/year-review/2026", Some(json!({}))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn yearly_patch_stores_and_clears_sold_pl() {
    let app = app().await;
    let stock_id = create_stock(&app, "HK", "中國銀行", None).await;
    create_buy(&app, stock_id, "2026-03-01", 100.0, 500.0).await;

    let (status, snapshot) = send(
        &app,
        "PATCH",
        "/api/summary/yearly/HK/2026",
        Some(json!({ "sold_pl": -14991.49 })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body = {snapshot}");
    approx(&snapshot["sold_pl"], -14991.49);

    let (status, yearly) = send(&app, "GET", "/api/summary/yearly?market=HK", None).await;
    assert_eq!(status, StatusCode::OK);
    let year = yearly["years"]
        .as_array()
        .expect("years")
        .iter()
        .find(|row| row["year"] == 2026)
        .expect("2026 row");
    approx(&year["sold_pl"], -14991.49);

    // Clearing the only stored figure removes the row.
    let (status, _) = send(
        &app,
        "PATCH",
        "/api/summary/yearly/HK/2026",
        Some(json!({ "sold_pl": null })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (_, yearly) = send(&app, "GET", "/api/summary/yearly?market=HK", None).await;
    let year = yearly["years"]
        .as_array()
        .expect("years")
        .iter()
        .find(|row| row["year"] == 2026)
        .expect("2026 row");
    assert!(year["sold_pl"].is_null());
    assert!(year["snapshot"].is_null());
}

// --- family deposits (家人 定期) ---

async fn create_family_deposit(app: &Router, body: Value) -> Value {
    let (status, body) = send(app, "POST", "/api/family/deposits", Some(body)).await;
    assert_eq!(status, StatusCode::CREATED, "body = {body}");
    body
}

#[tokio::test]
async fn family_deposit_crud_and_filters() {
    let app = app().await;

    let mum = create_family_deposit(
        &app,
        json!({ "holder": "媽媽", "label": "SC-9179", "bank": "SC",
                "principal": 200000, "interest": 1500.0,
                "start_date": "2026-01-02", "end_date": "2099-07-02",
                "note": "01 Jan to 02 Jul: 2.50%" }),
    )
    .await;
    let mum_id = mum["id"].as_i64().expect("id");
    assert_eq!(mum["holder"], "媽媽");
    assert_eq!(mum["status"], "ACTIVE");
    approx(&mum["total"], 201500.0);
    assert_eq!(mum["end_year"], 2099);
    assert_eq!(mum["end_month"], 7);

    let dad = create_family_deposit(
        &app,
        json!({ "holder": "爸爸", "label": "SC-9024", "bank": "SC",
                "principal": 300000, "interest": 920.29,
                "start_date": "2026-07-20", "end_date": "2099-08-31" }),
    )
    .await;
    assert_eq!(dad["sort_order"], 2);

    // Holder filter is an exact match; asc order is earliest end first.
    let (_, body) = send(&app, "GET", "/api/family/deposits?holder=媽媽", None).await;
    let rows = body.as_array().expect("array");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["label"], "SC-9179");

    let (_, body) = send(&app, "GET", "/api/family/deposits?status=active", None).await;
    assert_eq!(body.as_array().expect("array").len(), 2);
    let (_, body) = send(&app, "GET", "/api/family/deposits?status=ended", None).await;
    assert_eq!(body.as_array().expect("array").len(), 0);
    let (status, _) = send(&app, "GET", "/api/family/deposits?status=soon", None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let (_, body) = send(
        &app,
        "GET",
        "/api/family/deposits?year=2099&order=desc",
        None,
    )
    .await;
    let rows = body.as_array().expect("array");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["end_date"], "2099-08-31");

    // PATCH merges; a null clears an optional field; holder can move the row.
    let (status, body) = send(
        &app,
        "PATCH",
        &format!("/api/family/deposits/{mum_id}"),
        Some(json!({ "holder": "Irene", "interest": 1600.0, "note": null,
                     "end_date": "2099-07-05" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body = {body}");
    assert_eq!(body["holder"], "Irene");
    approx(&body["interest"], 1600.0);
    approx(&body["total"], 201600.0);
    assert!(body["note"].is_null());
    assert_eq!(body["end_date"], "2099-07-05");

    let (_, body) = send(&app, "GET", "/api/family/deposits?holder=媽媽", None).await;
    assert_eq!(body.as_array().expect("array").len(), 0);

    let (status, _) = send(
        &app,
        "DELETE",
        &format!("/api/family/deposits/{mum_id}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = send(
        &app,
        "DELETE",
        &format!("/api/family/deposits/{mum_id}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn family_deposit_validation_errors_name_the_fields() {
    let app = app().await;

    let (status, body) = send(
        &app,
        "POST",
        "/api/family/deposits",
        Some(json!({ "holder": "  ", "principal": 1000, "end_date": "2099-01-01" })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(
        body["fields"]
            .as_array()
            .expect("fields")
            .iter()
            .any(|f| f["field"] == "holder")
    );

    let (status, body) = send(
        &app,
        "POST",
        "/api/family/deposits",
        Some(json!({ "holder": "媽媽", "label": "SC-1", "end_date": "2026/13/45" })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(
        body["fields"]
            .as_array()
            .expect("fields")
            .iter()
            .any(|f| f["field"] == "end_date")
    );

    let (status, body) = send(
        &app,
        "POST",
        "/api/family/deposits",
        Some(
            json!({ "holder": "媽媽", "label": "SC-1", "principal": -5000,
                     "end_date": "2099-01-01" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(
        body["fields"]
            .as_array()
            .expect("fields")
            .iter()
            .any(|f| f["field"] == "principal")
    );

    let (status, body) = send(
        &app,
        "POST",
        "/api/family/deposits",
        Some(json!({ "holder": "媽媽", "end_date": "2099-01-01" })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(
        body["fields"]
            .as_array()
            .expect("fields")
            .iter()
            .any(|f| f["field"] == "label")
    );
}

#[tokio::test]
async fn family_deposit_receive_lifecycle_touches_no_ledger() {
    let app = app().await;

    // A deposit ending in the past is NOT auto-received, unlike 定期.
    let past = create_family_deposit(
        &app,
        json!({ "holder": "爸爸", "label": "SC-9024", "principal": 300000,
                "interest": 920.29, "end_date": "2026-01-05" }),
    )
    .await;
    let id = past["id"].as_i64().expect("id");
    assert_eq!(past["status"], "ACTIVE");
    assert!(past["received_at"].is_null());

    // 收訖 with the corrected interest actually paid.
    let (status, body) = send(
        &app,
        "POST",
        &format!("/api/family/deposits/{id}/receive"),
        Some(json!({ "interest": 920.29 })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body = {body}");
    assert_eq!(body["status"], "END");
    assert!(body["received_at"].is_string());
    approx(&body["interest"], 920.29);

    // Re-receiving conflicts.
    let (status, _) = send(
        &app,
        "POST",
        &format!("/api/family/deposits/{id}/receive"),
        Some(json!({})),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);

    // A malformed 收訖日 is a validation error.
    let (status, body) = send(
        &app,
        "POST",
        &format!("/api/family/deposits/{id}/unreceive"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body = {body}");
    assert_eq!(body["status"], "ACTIVE");
    assert!(body["received_at"].is_null());

    let (status, _) = send(
        &app,
        "POST",
        &format!("/api/family/deposits/{id}/unreceive"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
}

#[tokio::test]
async fn family_holder_note_and_summary() {
    let app = app().await;

    create_family_deposit(
        &app,
        json!({ "holder": "媽媽", "principal": 100000, "interest": 500.0,
                "end_date": "2099-03-01" }),
    )
    .await;
    let ended = create_family_deposit(
        &app,
        json!({ "holder": "媽媽", "principal": 80000, "interest": 300.0,
                "end_date": "2026-02-01" }),
    )
    .await;
    let ended_id = ended["id"].as_i64().expect("id");
    send(
        &app,
        "POST",
        &format!("/api/family/deposits/{ended_id}/receive"),
        Some(json!({})),
    )
    .await;

    // Save Mum's note; the summary returns it beside her figures.
    let (status, body) = send(
        &app,
        "PUT",
        "/api/family/holders/媽媽/note",
        Some(json!({ "note": "AIA 人壽保險 B027033487 / 危疾保險" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body = {body}");
    assert_eq!(body["note"], "AIA 人壽保險 B027033487 / 危疾保險");

    // A holder with only a note still appears, with no deposits.
    let (status, _) = send(
        &app,
        "PUT",
        "/api/family/holders/Irene/note",
        Some(json!({ "note": "irene note" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = send(&app, "GET", "/api/family/deposits/summary", None).await;
    assert_eq!(status, StatusCode::OK, "body = {body}");
    let holders = body["holders"].as_array().expect("holders");
    assert_eq!(holders.len(), 2);
    let mum = holders
        .iter()
        .find(|h| h["holder"] == "媽媽")
        .expect("媽媽 holder");
    assert_eq!(mum["note"], "AIA 人壽保險 B027033487 / 危疾保險");
    // Active principal counts only future end dates.
    approx(&mum["active_principal"], 100000.0);
    // Upcoming holds only the unreceived deposit.
    let upcoming = mum["upcoming"].as_array().expect("upcoming");
    assert_eq!(upcoming.len(), 1);
    assert_eq!(upcoming[0]["end_date"], "2099-03-01");
    let irene = holders
        .iter()
        .find(|h| h["holder"] == "Irene")
        .expect("Irene holder");
    assert_eq!(irene["note"], "irene note");
    assert_eq!(irene["upcoming"].as_array().expect("upcoming").len(), 0);
    approx(&irene["active_principal"], 0.0);

    let history_years = body["history_years"].as_array().expect("history_years");
    assert!(history_years.iter().any(|y| *y == 2026));
    assert!(history_years.iter().any(|y| *y == 2099));

    // The note survives deleting the holder's last deposit.
    let (status, _) = send(
        &app,
        "PUT",
        "/api/family/holders/Irene/note",
        Some(json!({ "note": null })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, body) = send(&app, "GET", "/api/family/deposits/summary", None).await;
    assert_eq!(status, StatusCode::OK, "body = {body}");
    let holders = body["holders"].as_array().expect("holders");
    assert_eq!(holders.len(), 1);
    assert_eq!(holders[0]["holder"], "媽媽");
}

#[tokio::test]
async fn family_deposits_do_not_touch_user_totals() {
    let app = app().await;

    // Some baseline user data so the snapshots are non-trivial.
    create_deposit(
        &app,
        json!({ "label": "SC-9632", "bank": "SC", "principal": 110000,
                "interest": 993, "end_date": "2099-10-12" }),
    )
    .await;
    let ym = wealth_backend::routes::today().to_string()[..7].to_string();
    let (status, _) = send(&app, "PATCH", &format!("/api/months/{ym}"), Some(json!({}))).await;
    assert_eq!(status, StatusCode::OK);

    // Snapshot the user-facing figures that must not move.
    let (_, deposits_before) = send(&app, "GET", "/api/deposits/summary", None).await;
    let (_, overview_before) = send(&app, "GET", "/api/overview", None).await;
    let (_, month_before) = send(&app, "GET", &format!("/api/months/{ym}"), None).await;

    // A family deposit ending this month, then 收訖 — record-only, so nothing
    // in the user's ledger may react to it.
    let end_this_month = format!("{ym}-15");
    let deposit = create_family_deposit(
        &app,
        json!({ "holder": "爸爸", "label": "SC-9024", "principal": 300000,
                "interest": 920.29, "end_date": end_this_month }),
    )
    .await;
    let id = deposit["id"].as_i64().expect("id");
    let (status, _) = send(
        &app,
        "POST",
        &format!("/api/family/deposits/{id}/receive"),
        Some(json!({})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (_, deposits_after) = send(&app, "GET", "/api/deposits/summary", None).await;
    let (_, overview_after) = send(&app, "GET", "/api/overview", None).await;
    let (_, month_after) = send(&app, "GET", &format!("/api/months/{ym}"), None).await;

    assert_eq!(deposits_before, deposits_after);
    assert_eq!(overview_before, overview_after);
    assert_eq!(month_before, month_after);
}
