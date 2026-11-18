mod common;

use common::*;

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
