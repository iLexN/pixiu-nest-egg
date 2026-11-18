mod common;

use common::*;

#[tokio::test]
async fn settings_round_trips_at_the_new_path() {
    let app = app().await;

    let (status, body) = send(&app, "GET", "/api/settings", None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["salary"].is_null());

    let (status, body) = send(
        &app,
        "PATCH",
        "/api/settings",
        Some(json!({ "salary": 60000 })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body = {body}");
    approx(&body["salary"], 60000.0);

    let (status, body) = send(&app, "GET", "/api/settings", None).await;
    assert_eq!(status, StatusCode::OK);
    approx(&body["salary"], 60000.0);

    // The old path is gone: /api/months/{ym} treats "settings" as a bad month
    // key, so both methods now fail as client errors instead of serving.
    let (status, _) = send(&app, "GET", "/api/months/settings", None).await;
    assert!(status.is_client_error(), "status = {status}");
    let (status, _) = send(
        &app,
        "PATCH",
        "/api/months/settings",
        Some(json!({ "salary": 1 })),
    )
    .await;
    assert!(status.is_client_error(), "status = {status}");
}

// --- record-history-on-write ---

#[tokio::test]
async fn fx_rate_round_trips_and_clears() {
    let app = app().await;

    let (status, body) = send(
        &app,
        "PATCH",
        "/api/fx-rates/USD",
        Some(json!({ "rate": 7.8 })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body = {body}");
    assert_eq!(body["currency"], "USD");
    approx(&body["rate_to_hkd"], 7.8);
    assert!(body["updated_at"].is_string());

    let (status, body) = send(&app, "GET", "/api/fx-rates", None).await;
    assert_eq!(status, StatusCode::OK);
    let rates = body.as_array().expect("rates");
    assert_eq!(rates.len(), 1);
    assert_eq!(rates[0]["currency"], "USD");
    approx(&rates[0]["rate_to_hkd"], 7.8);

    // `null` clears the rate: AIA's HKD figures drop out with it.
    let (status, body) = send(
        &app,
        "PATCH",
        "/api/fx-rates/USD",
        Some(json!({ "rate": null })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body = {body}");
    assert!(body["rate_to_hkd"].is_null());
    assert!(body["updated_at"].is_null());

    let (status, body) = send(&app, "GET", "/api/aia/summary", None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["rate"].is_null());
    assert!(body["totals"]["total_value_hkd"].is_null());
}

#[tokio::test]
async fn fx_rate_rejects_bad_values_and_currencies() {
    let app = app().await;

    for rate in [json!(0), json!(-1.5)] {
        let (status, body) = send(
            &app,
            "PATCH",
            "/api/fx-rates/USD",
            Some(json!({ "rate": rate })),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "rate = {rate}");
        assert_eq!(body["fields"][0]["field"], "rate");
    }

    let (status, body) = send(
        &app,
        "PATCH",
        "/api/fx-rates/EUR",
        Some(json!({ "rate": 8.0 })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["fields"][0]["field"], "currency");
}

// --- 5.1 cash accounts ---
