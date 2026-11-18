mod common;

use common::*;

#[tokio::test]
async fn forecast_items_crud_and_validation() {
    let app = app().await;
    let ym = wealth_backend::store::today().to_string()[..7].to_string();
    let next = ym_add(&ym, 1);

    // return_month on a non-deposit kind → 400.
    let (status, body) = send(
        &app,
        "POST",
        &format!("/api/forecast/{next}/items"),
        Some(json!({ "kind": "tax", "amount": -30000, "return_month": "2027-01" })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body = {body}");
    assert!(
        body["fields"].as_array().expect("fields")[0]["field"]
            .as_str()
            .unwrap()
            .contains("return_month")
    );

    // A zero amount is not a plan.
    let (status, _) = send(
        &app,
        "POST",
        &format!("/api/forecast/{next}/items"),
        Some(json!({ "kind": "tax", "amount": 0 })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // Two placements in the same month (the sheet's =7000+6000 cell) — each
    // is its own row, the column sums them.
    let mut ids = Vec::new();
    for amount in [-7000.0, -6000.0] {
        let (status, body) = send(
            &app,
            "POST",
            &format!("/api/forecast/{next}/items"),
            Some(json!({ "kind": "hs_deposit", "amount": amount })),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "body = {body}");
        ids.push(body["id"].as_i64().expect("id"));
    }

    let (status, body) = send(&app, "GET", "/api/forecast", None).await;
    assert_eq!(status, StatusCode::OK, "body = {body}");
    let months = body["months"].as_array().expect("months");
    assert_eq!(months.len(), 7);
    assert_eq!(months[0]["month"].as_str().unwrap(), format!("{ym}-01"));
    let hs_cell = months[1]["cells"]
        .as_array()
        .expect("cells")
        .iter()
        .find(|cell| cell["kind"] == "hs_deposit")
        .expect("hs_deposit cell");
    assert_eq!(hs_cell["plan_items"].as_array().expect("items").len(), 2);
    approx(&hs_cell["figure"], -13_000.0);
    // Both placements lock: locked gains 13000, then returns at +3 months.
    approx(&months[1]["locked"], 13_000.0);
    approx(&months[4]["deposit_return"], 13_000.0);

    // Patch the amount, then delete one row.
    let (status, body) = send(
        &app,
        "PATCH",
        &format!("/api/forecast-items/{}", ids[0]),
        Some(json!({ "amount": -7001.5 })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body = {body}");
    approx(&body["amount"], -7001.5);
    let (status, _) = send(
        &app,
        "DELETE",
        &format!("/api/forecast-items/{}", ids[1]),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = send(
        &app,
        "DELETE",
        &format!("/api/forecast-items/{}", ids[1]),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn forecast_bill_amount_setting_round_trips() {
    let app = app().await;

    let (status, body) = send(&app, "GET", "/api/settings", None).await;
    assert_eq!(status, StatusCode::OK);
    approx(&body["bill_amount"], 2158.0);

    let (status, body) = send(
        &app,
        "PATCH",
        "/api/settings",
        Some(json!({ "bill_amount": 2260.75 })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body = {body}");
    approx(&body["bill_amount"], 2260.75);

    // Negative bills are rejected.
    let (status, _) = send(
        &app,
        "PATCH",
        "/api/settings",
        Some(json!({ "bill_amount": -1 })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // The forecast's 繳費 row uses the setting on the quarter month.
    let (status, body) = send(&app, "GET", "/api/forecast", None).await;
    assert_eq!(status, StatusCode::OK);
    approx(&body["bill_amount"], 2260.75);
    let quarter = body["months"]
        .as_array()
        .expect("months")
        .iter()
        .find(|m| {
            ["-01-01", "-04-01", "-07-01", "-10-01"]
                .iter()
                .any(|suffix| m["month"].as_str().unwrap().ends_with(suffix))
        })
        .expect("a quarter month in the window");
    approx(&quarter["bill"], -2260.75);

    // Clearing restores the default.
    let (status, body) = send(
        &app,
        "PATCH",
        "/api/settings",
        Some(json!({ "bill_amount": null })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    approx(&body["bill_amount"], 2158.0);
}

#[tokio::test]
async fn forecast_convert_turns_a_plan_into_a_deposit() {
    let app = app().await;
    let ym = wealth_backend::store::today().to_string()[..7].to_string();
    let end_ym = ym_add(&ym, 6);

    let (status, body) = send(
        &app,
        "POST",
        &format!("/api/forecast/{ym}/items"),
        Some(json!({ "kind": "sc_deposit", "amount": -180000, "note": "高息馬拉松" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "body = {body}");
    let item_id = body["id"].as_i64().expect("id");

    // Before conversion the plan locks 180k in month 0 and returns it at +4.
    let (_, body) = send(&app, "GET", "/api/forecast", None).await;
    let months = body["months"].as_array().expect("months");
    approx(&months[0]["locked"], 180_000.0);
    approx(&months[4]["deposit_return"], 180_000.0);

    // A non-deposit kind cannot convert.
    let (status, body) = send(
        &app,
        "POST",
        &format!("/api/forecast/{ym}/items"),
        Some(json!({ "kind": "tax", "amount": -100 })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let tax_id = body["id"].as_i64().expect("id");
    let (status, _) = send(
        &app,
        "POST",
        &format!("/api/forecast-items/{tax_id}/convert"),
        Some(json!({ "end_date": format!("{end_ym}-15") })),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);

    // Convert: bank and principal default from the plan.
    let (status, body) = send(
        &app,
        "POST",
        &format!("/api/forecast-items/{item_id}/convert"),
        Some(json!({ "label": "SC-9999", "rate": 0.031, "interest": 1395,
                     "end_date": format!("{end_ym}-15") })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "body = {body}");
    assert_eq!(body["bank"].as_str().unwrap(), "SC");
    approx(&body["principal"], 180_000.0);

    // The deposit exists and the plan is gone — no derived return remains.
    let (_, body) = send(&app, "GET", "/api/deposits", None).await;
    assert_eq!(body.as_array().expect("deposits").len(), 1);
    let (_, body) = send(&app, "GET", "/api/forecast", None).await;
    let months = body["months"].as_array().expect("months");
    assert!(months.iter().all(|m| {
        m["cells"].as_array().unwrap().iter().all(|cell| {
            cell["plan_items"]
                .as_array()
                .unwrap()
                .iter()
                .all(|i| i["kind"] == "tax")
        })
    }));
    assert!(months.iter().all(|m| m["deposit_return"] == 0.0));
    // The deposit itself is the locked anchor now; it finishes at end month.
    approx(&months[0]["locked"], 180_000.0);
    approx(&months[6]["deposit_finish"], 180_000.0);
}
