mod common;

use common::*;

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
async fn moving_a_trade_across_markets_is_rejected() {
    let (app, pool) = app_with_pool().await;
    let hk = create_stock(&app, "HK", "中國銀行", None).await;
    set_price(&app, hk, 10.0).await;
    buy_trade(&app, hk, 100.0, 900.0).await;
    let moving = buy_trade(&app, hk, 100.0, 800.0).await;

    let us = create_stock(&app, "US", "VOO", None).await;
    set_price(&app, us, 100.0).await;
    let (status, body) = send(
        &app,
        "POST",
        "/api/trades",
        Some(json!({
            "stock_id": us, "trade_type": "BUY", "trade_date": "2026-01-05",
            "shares": 10, "unit_price": 90.0, "fee": 5.0
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "body = {body}");

    // A trade's market is fixed at creation — repointing it across markets
    // is a validation error and changes nothing.
    let (status, body) = send(
        &app,
        "PATCH",
        &format!("/api/trades/{moving}"),
        Some(json!({ "stock_id": us })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body = {body}");

    let stock_id: i64 = sqlx::query_scalar("SELECT stock_id FROM trades WHERE id = ?")
        .bind(moving)
        .fetch_one(&pool)
        .await
        .expect("trade row");
    assert_eq!(stock_id, hk);
}

#[tokio::test]
async fn us_trade_settles_the_usd_broker_cash() {
    let (app, _pool) = app_with_pool().await;
    let voo = create_stock(&app, "US", "VOO", None).await;
    let (status, body) = send(
        &app,
        "PATCH",
        "/api/ibkr",
        Some(json!({ "usd_cash": 5000.0 })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body = {body}");

    // A US BUY debits the USD broker account by its derived total.
    let (status, body) = send(
        &app,
        "POST",
        "/api/trades",
        Some(json!({
            "stock_id": voo, "trade_type": "BUY", "trade_date": "2026-11-05",
            "shares": 3, "unit_price": 696.04, "fee": 1.000009
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "body = {body}");
    let trade = body["id"].as_i64().expect("trade id");
    let (_, overview) = send(&app, "GET", "/api/overview", None).await;
    approx(
        &overview["ibkr"]["usd_cash"],
        5000.0 - (3.0 * 696.04 + 1.000009),
    );

    // Editing the total re-applies only the delta; deleting returns it all.
    let (status, body) = send(
        &app,
        "PATCH",
        &format!("/api/trades/{trade}"),
        Some(json!({ "shares": 4 })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body = {body}");
    let (_, overview) = send(&app, "GET", "/api/overview", None).await;
    approx(
        &overview["ibkr"]["usd_cash"],
        5000.0 - (4.0 * 696.04 + 1.000009),
    );
    let (status, _) = send(&app, "DELETE", &format!("/api/trades/{trade}"), None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (_, overview) = send(&app, "GET", "/api/overview", None).await;
    approx(&overview["ibkr"]["usd_cash"], 5000.0);
}
