use std::collections::BTreeMap;

use sqlx::Row;
use sqlx::SqlitePool;

use crate::store::{StoreError, now_timestamp};

/// `app_meta` keys holding the 美股 sheet's IBKR account block (A1:B5); the
/// cumulative transfer (B1) lives in the `ibkr_transfers` log instead.
pub const IBKR_NOW_VALUE_KEY: &str = "ibkr.now_value";
pub const IBKR_HKD_CASH_KEY: &str = "ibkr.hkd_cash";
pub const IBKR_USD_CASH_KEY: &str = "ibkr.usd_cash";

/// Σ amount over the whole `ibkr_transfers` log (the B1 cumulative figure).
pub async fn transferred_hkd(pool: &SqlitePool) -> Result<Option<f64>, StoreError> {
    Ok(
        sqlx::query_scalar("SELECT SUM(amount_hkd) FROM ibkr_transfers")
            .fetch_one(pool)
            .await?,
    )
}

/// Append a dated transfer row (a negative amount records a withdrawal or
/// undoes a mistyped entry).
pub async fn insert_transfer(
    pool: &SqlitePool,
    transfer_date: &str,
    amount_hkd: f64,
) -> Result<(), StoreError> {
    sqlx::query(
        "INSERT INTO ibkr_transfers (transfer_date, amount_hkd, created_at) \
         VALUES (?, ?, ?)",
    )
    .bind(transfer_date)
    .bind(amount_hkd)
    .bind(now_timestamp())
    .execute(pool)
    .await?;
    Ok(())
}

/// Σ `ibkr_transfers` per calendar year — the year's IBKR 轉入.
pub async fn transfers_by_year(pool: &SqlitePool) -> Result<BTreeMap<i32, f64>, StoreError> {
    let rows = sqlx::query(
        "SELECT CAST(strftime('%Y', transfer_date) AS INTEGER) AS year, \
         SUM(amount_hkd) AS total FROM ibkr_transfers GROUP BY year",
    )
    .fetch_all(pool)
    .await?;
    let mut map = BTreeMap::new();
    for row in &rows {
        map.insert(row.try_get("year")?, row.try_get("total")?);
    }
    Ok(map)
}
