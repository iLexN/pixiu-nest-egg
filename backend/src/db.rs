use std::path::Path;
use std::str::FromStr;

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;

pub const DEFAULT_DB_PATH: &str = "data/wealth.db";

/// Open (creating if needed) the SQLite database at `path` and run migrations.
pub async fn connect(path: &str) -> anyhow::Result<SqlitePool> {
    if let Some(parent) = Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;
    migrate(&pool).await?;
    Ok(pool)
}

/// An isolated in-memory database with migrations applied, for tests.
pub async fn connect_memory() -> anyhow::Result<SqlitePool> {
    let options = SqliteConnectOptions::from_str("sqlite::memory:")?.foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?;
    migrate(&pool).await?;
    Ok(pool)
}

/// Applying migrations twice is a no-op: sqlx records applied versions.
pub async fn migrate(pool: &SqlitePool) -> anyhow::Result<()> {
    sqlx::migrate!("./migrations").run(pool).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn table_names(pool: &SqlitePool) -> Vec<String> {
        sqlx::query_scalar::<_, String>(
            "SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name",
        )
        .fetch_all(pool)
        .await
        .expect("read schema")
    }

    #[tokio::test]
    async fn migrations_create_both_tables_and_rerun_cleanly() {
        let dir = std::env::temp_dir().join(format!("wealth-db-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("wealth.db");
        let path_str = path.to_str().expect("utf-8 path");

        let pool = connect(path_str).await.expect("first run");
        let tables = table_names(&pool).await;
        assert!(tables.contains(&"stocks".to_string()));
        assert!(tables.contains(&"trades".to_string()));
        assert!(tables.contains(&"deposits".to_string()));
        assert!(path.exists());

        // Second run against the same file must not fail or change the schema.
        migrate(&pool).await.expect("re-running migrations");
        let pool2 = connect(path_str).await.expect("second run");
        assert_eq!(table_names(&pool2).await, tables);

        pool.close().await;
        pool2.close().await;
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn in_memory_database_has_schema() {
        let pool = connect_memory().await.expect("memory db");
        let tables = table_names(&pool).await;
        assert!(tables.contains(&"stocks".to_string()));
        assert!(tables.contains(&"trades".to_string()));
        assert!(tables.contains(&"deposits".to_string()));
        assert!(tables.contains(&"market_history".to_string()));
    }
}
