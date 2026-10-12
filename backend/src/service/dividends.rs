use crate::store::StoreError;

/// `ibkr.usd_cash += amount` — a US dividend receipt lands there directly.
pub async fn credit_ibkr_usd(
    conn: &mut sqlx::SqliteConnection,
    amount: f64,
    _now: &str,
) -> Result<(), StoreError> {
    let current =
        crate::store::meta::meta_get(&mut *conn, crate::store::overview::IBKR_USD_CASH_KEY).await?;
    let total = current.and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0) + amount;
    crate::store::meta::meta_put(
        &mut *conn,
        crate::store::overview::IBKR_USD_CASH_KEY,
        Some(&total.to_string()),
    )
    .await?;
    Ok(())
}
