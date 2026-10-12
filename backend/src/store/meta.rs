//! `app_meta` key/value helpers shared by every feature.
//!
//! `exec` may be the pool or a transaction — e.g. a US dividend receipt
//! credits `ibkr.usd_cash` inside its own transaction.

use sqlx::SqlitePool;

use crate::store::StoreError;

pub async fn meta_get<'e, E>(exec: E, key: &str) -> Result<Option<String>, StoreError>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    Ok(
        sqlx::query_scalar("SELECT value FROM app_meta WHERE key = ?")
            .bind(key)
            .fetch_optional(exec)
            .await?,
    )
}

/// An empty note deletes the key rather than storing a blank row.
pub async fn meta_put<'e, E>(exec: E, key: &str, value: Option<&str>) -> Result<(), StoreError>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    match value {
        Some(value) if !value.trim().is_empty() => {
            sqlx::query(
                "INSERT INTO app_meta (key, value) VALUES (?, ?) \
                 ON CONFLICT (key) DO UPDATE SET value = excluded.value",
            )
            .bind(key)
            .bind(value)
            .execute(exec)
            .await?;
        }
        _ => {
            sqlx::query("DELETE FROM app_meta WHERE key = ?")
                .bind(key)
                .execute(exec)
                .await?;
        }
    }
    Ok(())
}

pub async fn meta_f64(pool: &SqlitePool, key: &str) -> Result<Option<f64>, StoreError> {
    Ok(meta_get(pool, key)
        .await?
        .and_then(|raw| raw.parse::<f64>().ok()))
}

/// Keys matching a `LIKE` pattern, sorted — e.g. `family.note.%`.
pub async fn keys_like(pool: &SqlitePool, pattern: &str) -> Result<Vec<String>, StoreError> {
    Ok(
        sqlx::query_scalar("SELECT key FROM app_meta WHERE key LIKE ? ORDER BY key")
            .bind(pattern)
            .fetch_all(pool)
            .await?,
    )
}
