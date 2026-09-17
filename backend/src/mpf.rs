//! MPF persistence helpers: the history-recording side of account updates.
//! Pure derivation lives in `calc.rs`; HTTP wiring lives in `routes/mpf.rs`.

use chrono::NaiveDate;
use sqlx::{Sqlite, SqlitePool, Transaction};

use crate::calc::mpf_gap_month_ends;

pub const NOTE_KEY: &str = "mpf.note";
pub const SEED_MAX_RATE_KEY: &str = "mpf.seed_max_rate";
pub const SEED_MAX_GAIN_KEY: &str = "mpf.seed_max_gain";

/// Record one contributions/balance update inside an open transaction:
/// backfill synthetic month-end rows for every fully elapsed month with no
/// records (carrying the pre-update values), then upsert today's row so
/// same-day corrections replace rather than append. `fallback_on` (the
/// account's creation date) anchors the gap scan when no history exists yet.
pub async fn record_history(
    tx: &mut Transaction<'_, Sqlite>,
    account_id: i64,
    previous: (f64, f64),
    current: (f64, f64),
    today: NaiveDate,
    fallback_on: NaiveDate,
) -> Result<(), sqlx::Error> {
    let anchor: Option<String> =
        sqlx::query_scalar("SELECT MAX(recorded_on) FROM mpf_history WHERE account_id = ?")
            .bind(account_id)
            .fetch_one(&mut **tx)
            .await?;
    let anchor_date = anchor
        .and_then(|raw| NaiveDate::parse_from_str(&raw, "%Y-%m-%d").ok())
        .unwrap_or(fallback_on);

    for date in mpf_gap_month_ends(anchor_date, today) {
        sqlx::query(
            "INSERT OR IGNORE INTO mpf_history \
             (account_id, recorded_on, contributions, balance, synthetic) \
             VALUES (?, ?, ?, ?, 1)",
        )
        .bind(account_id)
        .bind(date.to_string())
        .bind(previous.0)
        .bind(previous.1)
        .execute(&mut **tx)
        .await?;
    }

    sqlx::query(
        "INSERT INTO mpf_history \
         (account_id, recorded_on, contributions, balance, synthetic) \
         VALUES (?, ?, ?, ?, 0) \
         ON CONFLICT (account_id, recorded_on) DO UPDATE SET \
         contributions = excluded.contributions, balance = excluded.balance, synthetic = 0",
    )
    .bind(account_id)
    .bind(today.to_string())
    .bind(current.0)
    .bind(current.1)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub async fn meta_get(pool: &SqlitePool, key: &str) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar("SELECT value FROM app_meta WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await
}

/// An empty note deletes the key rather than storing a blank row.
pub async fn meta_put(
    pool: &SqlitePool,
    key: &str,
    value: Option<&str>,
) -> Result<(), sqlx::Error> {
    match value {
        Some(value) if !value.trim().is_empty() => {
            sqlx::query(
                "INSERT INTO app_meta (key, value) VALUES (?, ?) \
                 ON CONFLICT (key) DO UPDATE SET value = excluded.value",
            )
            .bind(key)
            .bind(value)
            .execute(pool)
            .await?;
        }
        _ => {
            sqlx::query("DELETE FROM app_meta WHERE key = ?")
                .bind(key)
                .execute(pool)
                .await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connect_memory;

    async fn insert_account(pool: &SqlitePool, created_at: &str) -> i64 {
        sqlx::query_scalar(
            "INSERT INTO mpf_accounts (label, sort_order, created_at, updated_at) \
             VALUES ('acc', 1, ?, ?) RETURNING id",
        )
        .bind(created_at)
        .bind(created_at)
        .fetch_one(pool)
        .await
        .expect("insert account")
    }

    async fn history(pool: &SqlitePool, account_id: i64) -> Vec<(String, f64, f64, i64)> {
        sqlx::query_as(
            "SELECT recorded_on, contributions, balance, synthetic FROM mpf_history \
             WHERE account_id = ? ORDER BY recorded_on",
        )
        .bind(account_id)
        .fetch_all(pool)
        .await
        .expect("read history")
    }

    async fn record(
        pool: &SqlitePool,
        account_id: i64,
        previous: (f64, f64),
        current: (f64, f64),
        today: &str,
        fallback_on: &str,
    ) {
        let mut tx = pool.begin().await.expect("begin");
        record_history(
            &mut tx,
            account_id,
            previous,
            current,
            NaiveDate::parse_from_str(today, "%Y-%m-%d").unwrap(),
            NaiveDate::parse_from_str(fallback_on, "%Y-%m-%d").unwrap(),
        )
        .await
        .expect("record history");
        tx.commit().await.expect("commit");
    }

    #[tokio::test]
    async fn same_day_updates_replace_the_row() {
        let pool = connect_memory().await.unwrap();
        let id = insert_account(&pool, "2026-09-01T00:00:00+08:00").await;

        record(
            &pool,
            id,
            (0.0, 0.0),
            (100.0, 110.0),
            "2026-09-17",
            "2026-09-01",
        )
        .await;
        record(
            &pool,
            id,
            (100.0, 110.0),
            (100.0, 120.0),
            "2026-09-17",
            "2026-09-01",
        )
        .await;

        let rows = history(&pool, id).await;
        assert_eq!(rows, vec![("2026-09-17".to_string(), 100.0, 120.0, 0)]);
    }

    #[tokio::test]
    async fn gap_months_backfill_with_pre_update_values() {
        let pool = connect_memory().await.unwrap();
        let id = insert_account(&pool, "2026-08-01T00:00:00+08:00").await;

        record(
            &pool,
            id,
            (0.0, 0.0),
            (100.0, 110.0),
            "2026-08-28",
            "2026-08-01",
        )
        .await;
        record(
            &pool,
            id,
            (100.0, 110.0),
            (100.0, 130.0),
            "2026-10-05",
            "2026-08-01",
        )
        .await;

        let rows = history(&pool, id).await;
        assert_eq!(
            rows,
            vec![
                ("2026-08-28".to_string(), 100.0, 110.0, 0),
                ("2026-09-30".to_string(), 100.0, 110.0, 1),
                ("2026-10-05".to_string(), 100.0, 130.0, 0),
            ]
        );
    }

    #[tokio::test]
    async fn adjacent_months_need_no_backfill() {
        let pool = connect_memory().await.unwrap();
        let id = insert_account(&pool, "2026-08-01T00:00:00+08:00").await;

        record(
            &pool,
            id,
            (0.0, 0.0),
            (100.0, 110.0),
            "2026-08-28",
            "2026-08-01",
        )
        .await;
        record(
            &pool,
            id,
            (100.0, 110.0),
            (100.0, 120.0),
            "2026-09-05",
            "2026-08-01",
        )
        .await;

        let rows = history(&pool, id).await;
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().all(|(_, _, _, synthetic)| *synthetic == 0));
    }

    #[tokio::test]
    async fn first_update_backfills_from_creation_month() {
        let pool = connect_memory().await.unwrap();
        let id = insert_account(&pool, "2026-07-10T00:00:00+08:00").await;

        record(
            &pool,
            id,
            (0.0, 0.0),
            (100.0, 150.0),
            "2026-10-05",
            "2026-07-10",
        )
        .await;

        let rows = history(&pool, id).await;
        assert_eq!(
            rows,
            vec![
                ("2026-08-31".to_string(), 0.0, 0.0, 1),
                ("2026-09-30".to_string(), 0.0, 0.0, 1),
                ("2026-10-05".to_string(), 100.0, 150.0, 0),
            ]
        );
    }
}
