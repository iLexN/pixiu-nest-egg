mod common;

use common::*;

#[tokio::test]
async fn manual_price_patch_records_market_history() {
    let (app, pool) = app_with_pool().await;
    let stock = create_stock(&app, "HK", "中國銀行", None).await;
    buy_trade(&app, stock, 100.0, 900.0).await;
    assert!(history_rows(&pool, "HK").await.is_empty());

    set_price(&app, stock, 10.0).await;

    let rows = history_rows(&pool, "HK").await;
    assert_eq!(rows.len(), 1);
    approx(&rows[0].1.into(), 900.0);
    approx(&rows[0].2.into(), 1000.0);
}

#[tokio::test]
async fn trade_create_and_delete_record_history() {
    let (app, pool) = app_with_pool().await;
    let stock = create_stock(&app, "HK", "中國銀行", None).await;
    set_price(&app, stock, 10.0).await;
    buy_trade(&app, stock, 100.0, 900.0).await;
    let rows = history_rows(&pool, "HK").await;
    assert_eq!(rows.len(), 1);
    approx(&rows[0].1.into(), 900.0);

    let trade = buy_trade(&app, stock, 100.0, 800.0).await;
    let rows = history_rows(&pool, "HK").await;
    assert_eq!(rows.len(), 1);
    approx(&rows[0].1.into(), 1700.0);
    approx(&rows[0].2.into(), 2000.0);

    let (status, _) = send(&app, "DELETE", &format!("/api/trades/{trade}"), None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let rows = history_rows(&pool, "HK").await;
    // Same-day updates replace the row rather than append.
    assert_eq!(rows.len(), 1);
    approx(&rows[0].1.into(), 900.0);
    approx(&rows[0].2.into(), 1000.0);
}

#[tokio::test]
async fn note_only_stock_patch_records_nothing() {
    let (app, pool) = app_with_pool().await;
    let stock = create_stock(&app, "HK", "中國銀行", None).await;
    set_price(&app, stock, 10.0).await;
    buy_trade(&app, stock, 100.0, 900.0).await;
    let before = history_count(&pool).await;
    assert!(before > 0);

    let (status, body) = send(
        &app,
        "PATCH",
        &format!("/api/stocks/{stock}"),
        Some(json!({ "note": "watch" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body = {body}");

    assert_eq!(history_count(&pool).await, before);
}

#[tokio::test]
async fn summary_reads_and_freeze_write_no_history() {
    let (app, pool) = app_with_pool().await;
    let stock = create_stock(&app, "HK", "中國銀行", None).await;
    set_price(&app, stock, 10.0).await;
    buy_trade(&app, stock, 100.0, 900.0).await;
    let before = history_count(&pool).await;
    assert!(before > 0);

    let (status, _) = send(&app, "GET", "/api/summary?market=HK", None).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = send(&app, "GET", "/api/summary/yearly?market=HK", None).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = send(&app, "GET", "/api/year-review", None).await;
    assert_eq!(status, StatusCode::OK);
    let year = &wealth_backend::store::today().to_string()[..4];
    let (status, body) = send(
        &app,
        "POST",
        &format!("/api/summary/yearly/HK/{year}/freeze"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body = {body}");

    assert_eq!(history_count(&pool).await, before);
}
