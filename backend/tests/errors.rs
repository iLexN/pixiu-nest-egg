mod common;

use common::*;

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
