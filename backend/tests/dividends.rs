mod common;

use common::*;

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
async fn dividend_receipt_defaults_price_to_current_stock_price() {
    let app = app().await;
    let stock = create_stock(&app, "HK", "新華保險", None).await;
    create_buy(&app, stock, "2024-01-10", 1000.0, 5000.0).await;

    let (status, _) = send(
        &app,
        "PATCH",
        &format!("/api/stocks/{stock}"),
        Some(json!({ "manual_price": 5.41 })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = send(
        &app,
        "POST",
        "/api/dividends",
        Some(json!({ "stock_id": stock, "pay_date": "2025-03-05", "estimated_amount": 900.0 })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "body = {body}");
    let id = body["id"].as_i64().expect("id");

    // Blank 當時現價 (null) snapshots the stock's current 現價.
    let (status, body) = send(
        &app,
        "PATCH",
        &format!("/api/dividends/{id}"),
        Some(json!({ "received_amount": 900.0, "received_price": null, "bank_in": false })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body = {body}");
    approx(&body["received_price"], 5.41);
    approx(&body["yield_on_price"], 900.0 / (5.41 * 1000.0));

    // Clearing the price on an already-received record stays cleared:
    // the default applies only to the pending → received transition.
    let (status, body) = send(
        &app,
        "PATCH",
        &format!("/api/dividends/{id}"),
        Some(json!({ "received_price": null })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body = {body}");
    assert!(body["received_price"].is_null());
}

#[tokio::test]
async fn dividend_receipt_without_stock_price_stores_null() {
    let app = app().await;
    let stock = create_stock(&app, "HK", "無現價", None).await;
    create_buy(&app, stock, "2024-01-10", 1000.0, 5000.0).await;

    let (status, body) = send(
        &app,
        "POST",
        "/api/dividends",
        Some(json!({ "stock_id": stock, "pay_date": "2025-03-05", "estimated_amount": 900.0 })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "body = {body}");
    let id = body["id"].as_i64().expect("id");

    let (status, body) = send(
        &app,
        "PATCH",
        &format!("/api/dividends/{id}"),
        Some(json!({ "received_amount": 900.0, "bank_in": false })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body = {body}");
    assert!(body["received_price"].is_null());
    assert!(body["yield_on_price"].is_null());
}

#[tokio::test]
async fn dividend_us_receipt_with_withholding_tax() {
    let app = app().await;
    create_stock(&app, "US", "VOO", None).await;
    let (status, div) = send(
        &app,
        "POST",
        "/api/dividends",
        Some(json!({ "market": "US", "code": "VOO",
                     "pay_date": "2099-12-10", "estimated_amount": 12.76 })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "body = {div}");
    let id = div["id"].as_i64().expect("div id");
    assert!(div["withholding_tax"].is_null());
    assert!(div["gross_amount"].is_null());

    // (e) Tax on a pending dividend is rejected.
    let (status, body) = send(
        &app,
        "PATCH",
        &format!("/api/dividends/{id}"),
        Some(json!({ "withholding_tax": 3.83 })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body = {body}");
    assert!(
        body["fields"]
            .as_array()
            .expect("fields")
            .iter()
            .any(|f| f["field"] == "withholding_tax")
    );

    // (c) Tax larger than the gross is rejected on withholding_tax.
    let (status, body) = send(
        &app,
        "PATCH",
        &format!("/api/dividends/{id}"),
        Some(json!({ "gross_amount": 10.0, "withholding_tax": 12.0 })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body = {body}");
    assert!(
        body["fields"]
            .as_array()
            .expect("fields")
            .iter()
            .any(|f| f["field"] == "withholding_tax")
    );

    // (d) Gross and net together are ambiguous.
    let (status, body) = send(
        &app,
        "PATCH",
        &format!("/api/dividends/{id}"),
        Some(json!({ "gross_amount": 12.76, "received_amount": 8.93 })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body = {body}");
    assert!(
        body["fields"]
            .as_array()
            .expect("fields")
            .iter()
            .any(|f| f["field"] == "gross_amount")
    );

    // The failed patches changed nothing.
    let (_, body) = send(&app, "GET", "/api/dividends?market=US", None).await;
    assert!(body[0]["received_amount"].is_null());
    assert!(body[0]["withholding_tax"].is_null());

    // (a) 收訖 with gross + tax stores the net and banks it into IBKR USD.
    let (status, body) = send(
        &app,
        "PATCH",
        &format!("/api/dividends/{id}"),
        Some(json!({ "gross_amount": 12.76, "withholding_tax": 3.83 })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body = {body}");
    approx(&body["received_amount"], 8.93);
    approx(&body["withholding_tax"], 3.83);
    approx(&body["gross_amount"], 12.76);
    let (_, overview) = send(&app, "GET", "/api/overview", None).await;
    approx(&overview["ibkr"]["usd_cash"], 8.93);

    // (g) Editing the received record with a new gross + tax recomputes net.
    let (status, body) = send(
        &app,
        "PATCH",
        &format!("/api/dividends/{id}"),
        Some(json!({ "gross_amount": 12.80, "withholding_tax": 3.84 })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body = {body}");
    approx(&body["received_amount"], 8.96);
    approx(&body["withholding_tax"], 3.84);
    approx(&body["gross_amount"], 12.80);
    // No re-banking on an amount edit — the credit stays at 8.93.
    let (_, overview) = send(&app, "GET", "/api/overview", None).await;
    approx(&overview["ibkr"]["usd_cash"], 8.93);

    // (f) Un-receive reverses the IBKR credit and clears the tax.
    let (status, body) = send(
        &app,
        "PATCH",
        &format!("/api/dividends/{id}"),
        Some(json!({ "received_amount": null })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body = {body}");
    assert!(body["received_amount"].is_null());
    assert!(body["withholding_tax"].is_null());
    assert!(body["gross_amount"].is_null());
    let (_, overview) = send(&app, "GET", "/api/overview", None).await;
    approx(&overview["ibkr"]["usd_cash"], 0.0);

    // (b) Gross only: received = gross, tax stays null.
    let (status, body) = send(
        &app,
        "PATCH",
        &format!("/api/dividends/{id}"),
        Some(json!({ "gross_amount": 200.0 })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body = {body}");
    approx(&body["received_amount"], 200.0);
    assert!(body["withholding_tax"].is_null());
    assert!(body["gross_amount"].is_null());
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
