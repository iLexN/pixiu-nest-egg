mod common;

use common::*;

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
