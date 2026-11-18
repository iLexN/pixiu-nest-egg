mod common;

use common::*;

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
    approx(&jan["payout"], 60000.0);
    approx(&jan["total"], 60362.96);
    let apr = &y2026["months"][3];
    approx(&apr["interest"], 15.27);

    let history_years = body["history_years"].as_array().expect("history_years");
    assert!(history_years.iter().any(|y| *y == 2026));
    assert!(history_years.iter().any(|y| *y == 2099));
}

#[tokio::test]
async fn deposit_placement_debits_the_account_and_delete_restores_it() {
    let (app, pool) = app_with_pool().await;
    let hs = seed_cash(&pool, "HS", 200000.0).await;

    let deposit = create_deposit(
        &app,
        json!({ "label": "HS-1", "bank": "HS", "principal": 130000,
                "interest": 1200, "start_date": "2099-10-13",
                "end_date": "2100-02-02", "debit_account_id": hs }),
    )
    .await;
    let id = deposit["id"].as_i64().expect("id");
    assert_eq!(deposit["debited_account_id"], hs);
    approx(&deposit["debited_amount"], 130000.0);
    approx(&cash_amount(&pool, hs).await.into(), 70000.0);

    let (status, _) = send(&app, "DELETE", &format!("/api/deposits/{id}"), None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    approx(&cash_amount(&pool, hs).await.into(), 200000.0);
}
