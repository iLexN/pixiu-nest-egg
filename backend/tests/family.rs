mod common;

use common::*;

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
    let ym = wealth_backend::store::today().to_string()[..7].to_string();
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

// --- forecast ---
