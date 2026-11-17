use sqlx::Row;
use sqlx::SqlitePool;
use sqlx::sqlite::SqliteRow;

use crate::models::{ManualAsset, ManualAssetLiquidity};
use crate::store::{StoreError, now_timestamp};

fn row_to_asset(row: &SqliteRow) -> Result<ManualAsset, StoreError> {
    let liquidity: String = row.try_get("liquidity")?;
    Ok(ManualAsset {
        id: row.try_get("id")?,
        label: row.try_get("label")?,
        liquidity: ManualAssetLiquidity::parse(&liquidity).ok_or_else(|| {
            StoreError::Corrupt(format!("stored liquidity {liquidity} is not valid"))
        })?,
        amount: row.try_get("amount")?,
        note: row.try_get("note")?,
        sort_order: row.try_get("sort_order")?,
        updated_at: row.try_get("updated_at")?,
    })
}

const COLUMNS: &str = "id, label, liquidity, amount, note, sort_order, updated_at";

pub(crate) async fn load_one(pool: &SqlitePool, id: i64) -> Result<ManualAsset, StoreError> {
    let row = sqlx::query(sqlx::AssertSqlSafe(format!(
        "SELECT {COLUMNS} FROM manual_assets WHERE id = ?"
    )))
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| StoreError::NotFound(format!("manual asset {id} not found")))?;
    row_to_asset(&row)
}

pub async fn load_all(pool: &SqlitePool) -> Result<Vec<ManualAsset>, StoreError> {
    let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
        "SELECT {COLUMNS} FROM manual_assets ORDER BY sort_order, id"
    )))
    .fetch_all(pool)
    .await?;
    rows.iter().map(row_to_asset).collect::<Result<Vec<_>, _>>()
}

pub(crate) async fn next_sort_order(pool: &SqlitePool) -> Result<i64, StoreError> {
    Ok(
        sqlx::query_scalar("SELECT COALESCE(MAX(sort_order), 0) + 1 FROM manual_assets")
            .fetch_one(pool)
            .await?,
    )
}

pub(crate) async fn insert(
    pool: &SqlitePool,
    label: &str,
    liquidity: &str,
    amount: f64,
    note: Option<&str>,
    sort_order: i64,
) -> Result<i64, StoreError> {
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO manual_assets (label, liquidity, amount, note, sort_order, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(label)
    .bind(liquidity)
    .bind(amount)
    .bind(note)
    .bind(sort_order)
    .bind(now_timestamp())
    .fetch_one(pool)
    .await?;
    Ok(id)
}

pub(crate) async fn update(
    pool: &SqlitePool,
    id: i64,
    label: &str,
    liquidity: &str,
    amount: f64,
    note: Option<&str>,
) -> Result<(), StoreError> {
    sqlx::query(
        "UPDATE manual_assets SET label = ?, liquidity = ?, amount = ?, note = ?, \
         updated_at = ? WHERE id = ?",
    )
    .bind(label)
    .bind(liquidity)
    .bind(amount)
    .bind(note)
    .bind(now_timestamp())
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Rows deleted, so the handler can tell "not found" apart from success.
pub(crate) async fn delete(pool: &SqlitePool, id: i64) -> Result<u64, StoreError> {
    Ok(sqlx::query("DELETE FROM manual_assets WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?
        .rows_affected())
}

/// Σ amount over the `manual_assets` asset rows.
pub(crate) async fn sum(pool: &SqlitePool) -> Result<f64, StoreError> {
    Ok(
        sqlx::query_scalar("SELECT COALESCE(SUM(amount), 0.0) FROM manual_assets")
            .fetch_one(pool)
            .await?,
    )
}
