mod common;

use common::*;

#[tokio::test]
async fn month_recapture_snapshots_start_cash_from_the_cash_sum() {
    let (app, pool) = app_with_pool().await;

    let hs_id = seed_cash(&pool, "HS", 1000.0).await;
    seed_cash(&pool, "渣打", 500.0).await;

    // Creating a month stores no 月初.
    let (status, month) = send(&app, "PATCH", "/api/months/2099-03", Some(json!({}))).await;
    assert_eq!(status, StatusCode::OK, "body = {month}");
    assert!(month["start_cash"].is_null());

    // 重新擷取 fills 月初 with the live 活期 sum.
    let (status, month) = send(
        &app,
        "PATCH",
        "/api/months/2099-03",
        Some(json!({ "recapture": true })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body = {month}");
    approx(&month["start_cash"], 1500.0);

    // Editing a cash account then recapturing again refreshes 月初.
    sqlx::query("UPDATE cash_accounts SET amount = 700 WHERE id = ?")
        .bind(hs_id)
        .execute(&pool)
        .await
        .expect("edit cash account");
    let (status, month) = send(
        &app,
        "PATCH",
        "/api/months/2099-03",
        Some(json!({ "recapture": true })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body = {month}");
    approx(&month["start_cash"], 1200.0);

    // An explicit 月初 wins over recapture.
    let (status, month) = send(
        &app,
        "PATCH",
        "/api/months/2099-03",
        Some(json!({ "start_cash": 42.0, "recapture": true })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body = {month}");
    approx(&month["start_cash"], 42.0);
}
