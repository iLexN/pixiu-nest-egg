use sqlx::Row;
use sqlx::SqlitePool;
use sqlx::sqlite::SqliteRow;

use crate::models::{ForecastItem, ForecastItemKind};
use crate::store::{StoreError, now_timestamp};

const ITEM_COLUMNS: &str = "id, month, kind, amount, return_month, note, sort_order, created_at";

pub fn row_to_item(row: &SqliteRow) -> Result<ForecastItem, StoreError> {
    let kind: String = row.try_get("kind")?;
    Ok(ForecastItem {
        id: row.try_get("id")?,
        month: row.try_get("month")?,
        kind: ForecastItemKind::parse(&kind)
            .ok_or_else(|| StoreError::Corrupt(format!("stored kind {kind} is not valid")))?,
        amount: row.try_get("amount")?,
        return_month: row.try_get("return_month")?,
        note: row.try_get("note")?,
        sort_order: row.try_get("sort_order")?,
        created_at: row.try_get("created_at")?,
    })
}

pub async fn load_item(pool: &SqlitePool, id: i64) -> Result<ForecastItem, StoreError> {
    let row = sqlx::query(sqlx::AssertSqlSafe(format!(
        "SELECT {ITEM_COLUMNS} FROM forecast_items WHERE id = ?"
    )))
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| StoreError::NotFound(format!("forecast item {id} not found")))?;
    row_to_item(&row)
}

pub async fn load_all_items(pool: &SqlitePool) -> Result<Vec<ForecastItem>, StoreError> {
    let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
        "SELECT {ITEM_COLUMNS} FROM forecast_items ORDER BY month, sort_order, id"
    )))
    .fetch_all(pool)
    .await?;
    rows.iter().map(row_to_item).collect()
}

pub async fn next_sort_order(pool: &SqlitePool, month: &str) -> Result<i64, StoreError> {
    Ok(sqlx::query_scalar(
        "SELECT COALESCE(MAX(sort_order), 0) + 1 FROM forecast_items WHERE month = ?",
    )
    .bind(month)
    .fetch_one(pool)
    .await?)
}

#[allow(clippy::too_many_arguments)]
pub async fn insert_item(
    pool: &SqlitePool,
    month: &str,
    kind: &str,
    amount: f64,
    return_month: &Option<String>,
    note: &Option<String>,
    sort_order: i64,
) -> Result<i64, StoreError> {
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO forecast_items (month, kind, amount, return_month, note, sort_order, \
         created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(month)
    .bind(kind)
    .bind(amount)
    .bind(return_month)
    .bind(note)
    .bind(sort_order)
    .bind(now_timestamp())
    .bind(now_timestamp())
    .fetch_one(pool)
    .await?;
    Ok(id)
}

pub async fn update_item(
    pool: &SqlitePool,
    id: i64,
    kind: &str,
    amount: f64,
    return_month: &Option<String>,
    note: &Option<String>,
) -> Result<(), StoreError> {
    sqlx::query(
        "UPDATE forecast_items SET kind = ?, amount = ?, return_month = ?, note = ?, \
         updated_at = ? WHERE id = ?",
    )
    .bind(kind)
    .bind(amount)
    .bind(return_month)
    .bind(note)
    .bind(now_timestamp())
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Rows deleted, so the handler can tell "not found" apart from success.
pub async fn delete_item(pool: &SqlitePool, id: i64) -> Result<u64, StoreError> {
    Ok(sqlx::query("DELETE FROM forecast_items WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?
        .rows_affected())
}

/// Inside the caller's transaction (the 轉為定期 conversion).
pub async fn delete_item_in_tx(
    conn: &mut sqlx::SqliteConnection,
    id: i64,
) -> Result<(), StoreError> {
    sqlx::query("DELETE FROM forecast_items WHERE id = ?")
        .bind(id)
        .execute(&mut *conn)
        .await?;
    Ok(())
}
