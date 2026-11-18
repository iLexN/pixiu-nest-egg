mod common;

use common::*;

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
