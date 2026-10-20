//! The `ibkr_transfers` log and the reported IBKR totals — the 美股 sheet's
//! IBKR account block (A1:B5). The cash figures are the `broker`
//! cash_accounts rows; the reported total is the `ibkr.reported_value`
//! settings key.

use std::collections::BTreeMap;

use sqlx::Row;

use crate::store::{StoreError, now_timestamp};

/// Σ amount over the whole `ibkr_transfers` log (the B1 cumulative figure).
pub async fn transferred_hkd<'e, E>(exec: E) -> Result<Option<f64>, StoreError>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    Ok(
        sqlx::query_scalar("SELECT SUM(amount_hkd) FROM ibkr_transfers")
            .fetch_one(exec)
            .await?,
    )
}

/// Append a dated transfer row (a negative amount records a withdrawal or
/// undoes a mistyped entry).
pub async fn insert_transfer<'e, E>(
    exec: E,
    transfer_date: &str,
    amount_hkd: f64,
) -> Result<(), StoreError>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    sqlx::query(
        "INSERT INTO ibkr_transfers (transfer_date, amount_hkd, created_at) \
         VALUES (?, ?, ?)",
    )
    .bind(transfer_date)
    .bind(amount_hkd)
    .bind(now_timestamp())
    .execute(exec)
    .await?;
    Ok(())
}

/// Σ `ibkr_transfers` per calendar year — the year's IBKR 轉入.
pub async fn transfers_by_year(pool: &sqlx::SqlitePool) -> Result<BTreeMap<i32, f64>, StoreError> {
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
