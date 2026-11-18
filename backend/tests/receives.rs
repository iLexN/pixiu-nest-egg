mod common;

use common::*;

#[tokio::test]
async fn deposit_receive_credits_cash_and_records_the_month_item() {
    let (app, pool) = app_with_pool().await;

    // The cash account the returned money credits into, and the month the
    // dep-end item lands in.
    let asset_id = seed_cash(&pool, "渣打", 1000.0).await;
    let (status, _) = send(
        &app,
        "PATCH",
        "/api/months/2099-10",
        Some(json!({ "start_cash": 5000 })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let deposit = create_deposit(
        &app,
        json!({ "label": "SC-9632", "bank": "SC", "principal": 110000,
                "interest": 993, "end_date": "2099-10-12" }),
    )
    .await;
    let id = deposit["id"].as_i64().expect("id");
    assert_eq!(deposit["status"], "ACTIVE");
    assert!(deposit["received_at"].is_null());

    // Before 收訖 the deposit's interest previews but does not count.
    let (_, detail) = send(&app, "GET", "/api/months/2099-10", None).await;
    approx(&detail["month"]["interest"], 0.0);
    let auto = detail["interest_auto"].as_array().expect("interest_auto");
    assert_eq!(auto.len(), 1);
    assert_eq!(auto[0]["received"], false);

    let (status, body) = send(
        &app,
        "POST",
        &format!("/api/deposits/{id}/receive"),
        Some(json!({ "account_id": asset_id })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body = {body}");
    assert_eq!(body["status"], "END");
    assert!(body["received_at"].is_string());

    approx(&cash_amount(&pool, asset_id).await.into(), 111993.0);
    // The exact account credited is recorded for reversal.
    let credited_account: i64 =
        sqlx::query_scalar("SELECT credited_account_id FROM deposits WHERE id = ?")
            .bind(id)
            .fetch_one(&pool)
            .await
            .expect("credited account");
    assert_eq!(credited_account, asset_id);

    let (_, detail) = send(&app, "GET", "/api/months/2099-10", None).await;
    approx(&detail["month"]["interest"], 993.0);
    let dep_end_key = format!("dep-end:{id}");
    let items = detail["items"].as_array().expect("items");
    let dep_end = items
        .iter()
        .find(|item| item["auto_key"] == dep_end_key)
        .expect("dep-end item");
    approx(&dep_end["amount"], 110993.0);

    // Re-receiving conflicts.
    let (status, _) = send(
        &app,
        "POST",
        &format!("/api/deposits/{id}/receive"),
        Some(json!({})),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
}

#[tokio::test]
async fn bond_receive_credits_principal_and_records_the_month_item() {
    let (app, pool) = app_with_pool().await;

    let asset_id = seed_cash(&pool, "HS", 500.0).await;
    let (status, _) = send(
        &app,
        "PATCH",
        "/api/months/2099-12",
        Some(json!({ "start_cash": 5000 })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, bond) = send(
        &app,
        "POST",
        "/api/bonds",
        Some(json!({ "label": "silver bond", "principal": 100000,
                     "maturity_date": "2099-12-23" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "body = {bond}");
    let id = bond["id"].as_i64().expect("id");
    assert!(bond["received_at"].is_null());

    // 收訖 the principal return: credit the cash row, record the bond-end item.
    let (status, body) = send(
        &app,
        "POST",
        &format!("/api/bonds/{id}/receive"),
        Some(json!({ "account_id": asset_id })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body = {body}");
    assert!(body["received_at"].is_string());

    approx(&cash_amount(&pool, asset_id).await.into(), 100500.0);
    let bond_end_key = format!("bond-end:{id}");
    let (_, detail) = send(&app, "GET", "/api/months/2099-12", None).await;
    let item = detail["items"]
        .as_array()
        .expect("items")
        .iter()
        .find(|item| item["auto_key"] == bond_end_key)
        .expect("bond-end item");
    approx(&item["amount"], 100000.0);
}

// --- 4.7 dividends (派息) ---

#[tokio::test]
async fn coupon_and_dividend_receive_banks_in_and_records_the_item() {
    let (app, pool) = app_with_pool().await;

    // HS cash account + month rows for the receipts' months.
    let hs_id = seed_cash(&pool, "HS", 100.0).await;
    for ym in ["2099-10", "2099-11"] {
        send(
            &app,
            "PATCH",
            &format!("/api/months/{ym}"),
            Some(json!({ "start_cash": 1.0 })),
        )
        .await;
    }

    // --- coupon: 收訖 banks into HS and stores the coupon:<id> item ---
    let (status, bond) = send(
        &app,
        "POST",
        "/api/bonds",
        Some(json!({ "label": "silver bond", "principal": 100000,
                     "maturity_date": "2099-12-23" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "body = {bond}");
    let bond_id = bond["id"].as_i64().expect("bond id");
    let (status, coupon) = send(
        &app,
        "POST",
        "/api/coupons",
        Some(json!({ "bond_id": bond_id, "pay_date": "2099-10-23",
                     "per_10k": 200.0 })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "body = {coupon}");
    let coupon_id = coupon["id"].as_i64().expect("coupon id");

    let (status, body) = send(
        &app,
        "PATCH",
        &format!("/api/coupons/{coupon_id}"),
        Some(json!({ "received_amount": 1000.0 })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body = {body}");

    approx(&cash_amount(&pool, hs_id).await.into(), 1100.0);
    let coupon_key = format!("coupon:{coupon_id}");
    let (_, detail) = send(&app, "GET", "/api/months/2099-10", None).await;
    let item = detail["items"]
        .as_array()
        .expect("items")
        .iter()
        .find(|i| i["auto_key"] == coupon_key)
        .expect("coupon item");
    approx(&item["amount"], 1000.0);

    // Clearing received_amount reverses the credit and drops the item.
    let (status, _) = send(
        &app,
        "PATCH",
        &format!("/api/coupons/{coupon_id}"),
        Some(json!({ "received_amount": null })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    approx(&cash_amount(&pool, hs_id).await.into(), 100.0);
    let (_, detail) = send(&app, "GET", "/api/months/2099-10", None).await;
    assert!(
        detail["items"]
            .as_array()
            .expect("items")
            .iter()
            .all(|i| i["auto_key"] != coupon_key)
    );

    // --- HK dividend: 收訖 banks into HS + div:<id> item ---
    let stock = create_stock(&app, "HK", "中國銀行", None).await;
    create_buy(&app, stock, "2099-10-01", 1000.0, 5000.0).await;
    let (status, div) = send(
        &app,
        "POST",
        "/api/dividends",
        Some(json!({ "market": "HK", "code": "中國銀行",
                     "pay_date": "2099-11-05", "estimated_amount": 480.0 })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "body = {div}");
    let div_id = div["id"].as_i64().expect("div id");

    let (status, body) = send(
        &app,
        "PATCH",
        &format!("/api/dividends/{div_id}"),
        Some(json!({ "received_amount": 500.0 })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body = {body}");
    approx(&cash_amount(&pool, hs_id).await.into(), 600.0);
    let div_key = format!("div:{div_id}");
    let (_, detail) = send(&app, "GET", "/api/months/2099-11", None).await;
    assert!(
        detail["items"]
            .as_array()
            .expect("items")
            .iter()
            .any(|i| i["auto_key"] == div_key)
    );

    // --- US dividend: 收訖 banks into IBKR USD cash, no month item ---
    create_stock(&app, "US", "AAPL", None).await;
    let (status, div) = send(
        &app,
        "POST",
        "/api/dividends",
        Some(json!({ "market": "US", "code": "AAPL",
                     "pay_date": "2099-11-10", "estimated_amount": 200.0 })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "body = {div}");
    let us_id = div["id"].as_i64().expect("us div id");
    let (status, body) = send(
        &app,
        "PATCH",
        &format!("/api/dividends/{us_id}"),
        Some(json!({ "received_amount": 200.0 })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body = {body}");
    approx(&cash_amount(&pool, hs_id).await.into(), 600.0); // HS untouched
    let (_, overview) = send(&app, "GET", "/api/overview", None).await;
    approx(&overview["ibkr"]["usd_cash"], 200.0);
    // The recorded credit points at the IBKR USD broker account.
    let ibkr_usd: i64 = sqlx::query_scalar(
        "SELECT id FROM cash_accounts WHERE role = 'broker' AND currency = 'USD'",
    )
    .fetch_one(&pool)
    .await
    .expect("IBKR USD account");
    let credited_account: i64 =
        sqlx::query_scalar("SELECT credited_account_id FROM dividends WHERE id = ?")
            .bind(us_id)
            .fetch_one(&pool)
            .await
            .expect("credited account");
    assert_eq!(credited_account, ibkr_usd);
    let (_, detail) = send(&app, "GET", "/api/months/2099-11", None).await;
    assert!(
        detail["items"]
            .as_array()
            .expect("items")
            .iter()
            .all(|i| i["auto_key"] != format!("div:{us_id}"))
    );

    // Un-receipt returns the IBKR cash.
    let (status, _) = send(
        &app,
        "PATCH",
        &format!("/api/dividends/{us_id}"),
        Some(json!({ "received_amount": null })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (_, overview) = send(&app, "GET", "/api/overview", None).await;
    approx(&overview["ibkr"]["usd_cash"], 0.0);
}

/// A coupon receipt banks into the default account; after the default moves,
/// clearing the receipt reverses the ORIGINAL account, not the new default.
#[tokio::test]
async fn coupon_reversal_follows_the_recorded_account_after_default_moves() {
    let (app, pool) = app_with_pool().await;

    let hs_id = seed_cash(&pool, "HS", 100.0).await;
    let sc_id = seed_cash(&pool, "渣打", 0.0).await;
    send(
        &app,
        "PATCH",
        "/api/months/2099-10",
        Some(json!({ "start_cash": 1.0 })),
    )
    .await;

    let (status, bond) = send(
        &app,
        "POST",
        "/api/bonds",
        Some(json!({ "label": "b", "principal": 100000, "maturity_date": "2099-12-23" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "body = {bond}");
    let bond_id = bond["id"].as_i64().expect("bond id");
    let (status, coupon) = send(
        &app,
        "POST",
        "/api/coupons",
        Some(json!({ "bond_id": bond_id, "pay_date": "2099-10-23", "per_10k": 200.0 })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "body = {coupon}");
    let coupon_id = coupon["id"].as_i64().expect("coupon id");

    // 收訖 into the HS default.
    let (status, body) = send(
        &app,
        "PATCH",
        &format!("/api/coupons/{coupon_id}"),
        Some(json!({ "received_amount": 1000.0 })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body = {body}");
    approx(&cash_amount(&pool, hs_id).await.into(), 1100.0);
    let credited_account: i64 =
        sqlx::query_scalar("SELECT credited_account_id FROM bond_coupons WHERE id = ?")
            .bind(coupon_id)
            .fetch_one(&pool)
            .await
            .expect("credited account");
    assert_eq!(credited_account, hs_id);

    // Move the default to 渣打.
    sqlx::query("UPDATE cash_accounts SET is_default_receipt = 0 WHERE id = ?")
        .bind(hs_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE cash_accounts SET is_default_receipt = 1 WHERE id = ?")
        .bind(sc_id)
        .execute(&pool)
        .await
        .unwrap();

    // Clearing the receipt subtracts from HS — the recorded account.
    let (status, body) = send(
        &app,
        "PATCH",
        &format!("/api/coupons/{coupon_id}"),
        Some(json!({ "received_amount": null })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body = {body}");
    approx(&cash_amount(&pool, hs_id).await.into(), 100.0);
    approx(&cash_amount(&pool, sc_id).await.into(), 0.0);
}

// --- 5.1 fx rates ---

#[tokio::test]
async fn deposit_receive_rejects_a_wrong_currency_account() {
    let (app, pool) = app_with_pool().await;

    let usd_broker: i64 = sqlx::query_scalar(
        "SELECT id FROM cash_accounts WHERE role = 'broker' AND currency = 'USD'",
    )
    .fetch_one(&pool)
    .await
    .expect("IBKR USD account");

    let deposit = create_deposit(
        &app,
        json!({ "label": "SC-9632", "bank": "SC", "principal": 110000,
                "interest": 993, "end_date": "2099-10-12" }),
    )
    .await;
    let id = deposit["id"].as_i64().expect("id");

    let (status, body) = send(
        &app,
        "POST",
        &format!("/api/deposits/{id}/receive"),
        Some(json!({ "account_id": usd_broker })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body = {body}");
    assert_eq!(body["fields"][0]["field"], "account_id");
}

#[tokio::test]
async fn deposit_receive_without_bank_in_changes_no_account() {
    let (app, pool) = app_with_pool().await;

    let hs_id = seed_cash(&pool, "HS", 1000.0).await;
    let deposit = create_deposit(
        &app,
        json!({ "label": "SC-9632", "bank": "SC", "principal": 110000,
                "interest": 993, "end_date": "2099-10-12" }),
    )
    .await;
    let id = deposit["id"].as_i64().expect("id");

    let (status, body) = send(
        &app,
        "POST",
        &format!("/api/deposits/{id}/receive"),
        Some(json!({ "bank_in": false })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body = {body}");
    assert!(body["credited_account_id"].is_null());
    approx(&cash_amount(&pool, hs_id).await.into(), 1000.0);
}

#[tokio::test]
async fn hk_dividend_receive_into_a_chosen_account_credits_it() {
    let (app, pool) = app_with_pool().await;

    seed_cash(&pool, "HS", 100.0).await;
    let sc_id = seed_cash(&pool, "渣打", 500.0).await;

    create_stock(&app, "HK", "中國銀行", None).await;
    let (status, div) = send(
        &app,
        "POST",
        "/api/dividends",
        Some(json!({ "market": "HK", "code": "中國銀行",
                     "pay_date": "2099-11-05", "estimated_amount": 480.0 })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "body = {div}");
    let div_id = div["id"].as_i64().expect("div id");

    let (status, body) = send(
        &app,
        "PATCH",
        &format!("/api/dividends/{div_id}"),
        Some(json!({ "received_amount": 500.0, "account_id": sc_id })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body = {body}");
    assert_eq!(body["credited_account_id"], sc_id);
    approx(&body["credited_amount"], 500.0);
    approx(&cash_amount(&pool, sc_id).await.into(), 1000.0);
}

// --- 5.1 settings move ---
