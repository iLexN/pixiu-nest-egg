mod common;

use common::*;

#[tokio::test]
async fn cash_account_crud_and_rules() {
    let app = app().await;

    let (status, body) = send(
        &app,
        "POST",
        "/api/cash-accounts",
        Some(json!({ "label": "HS", "currency": "HKD", "role": "bank",
                     "amount": 100.0 })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "body = {body}");
    let hs_id = body["id"].as_i64().expect("id");
    assert_eq!(body["currency"], "HKD");
    assert_eq!(body["role"], "bank");

    // A USD bank is meaningless.
    let (status, body) = send(
        &app,
        "POST",
        "/api/cash-accounts",
        Some(json!({ "label": "USD bank", "currency": "USD", "role": "bank" })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body = {body}");
    assert_eq!(body["fields"][0]["field"], "currency");

    // The migration seeds one USD broker (IBKR USD); a second conflicts.
    let (status, _) = send(
        &app,
        "POST",
        "/api/cash-accounts",
        Some(json!({ "label": "other USD broker", "currency": "USD",
                     "role": "broker" })),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);

    // Broker accounts cannot be deleted.
    let (status, body) = send(&app, "GET", "/api/cash-accounts", None).await;
    assert_eq!(status, StatusCode::OK);
    let broker_id = body
        .as_array()
        .expect("accounts")
        .iter()
        .find(|a| a["role"] == "broker")
        .expect("a broker account")["id"]
        .as_i64()
        .expect("broker id");
    let (status, _) = send(
        &app,
        "DELETE",
        &format!("/api/cash-accounts/{broker_id}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);

    // Flagging 渣打 as the HKD default unflags HS in the same transaction.
    let (status, body) = send(
        &app,
        "POST",
        "/api/cash-accounts",
        Some(json!({ "label": "渣打", "currency": "HKD", "role": "bank" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "body = {body}");
    let sc_id = body["id"].as_i64().expect("id");

    let (status, _) = send(
        &app,
        "PATCH",
        &format!("/api/cash-accounts/{hs_id}"),
        Some(json!({ "is_default_receipt": true })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, body) = send(
        &app,
        "PATCH",
        &format!("/api/cash-accounts/{sc_id}"),
        Some(json!({ "is_default_receipt": true })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body = {body}");
    assert_eq!(body["is_default_receipt"], true);

    let (status, body) = send(&app, "GET", "/api/cash-accounts", None).await;
    assert_eq!(status, StatusCode::OK);
    let accounts = body.as_array().expect("accounts");
    let hs = accounts.iter().find(|a| a["id"] == hs_id).expect("HS");
    assert_eq!(hs["is_default_receipt"], false);
}

// --- 5.1 receipts over HTTP ---
