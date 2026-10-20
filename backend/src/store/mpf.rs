//! MPF persistence helpers: the history-recording side of account updates.
//! Pure derivation lives in `calc/`; HTTP wiring lives in `routes/mpf.rs`.

use chrono::NaiveDate;
use sqlx::sqlite::SqliteRow;
use sqlx::{Row, SqlitePool};

use crate::calc::{MpfAccountFacts, MpfPoint, mpf_figures, mpf_gap_month_ends};
use crate::models::MpfHistoryRow;
use crate::store::{StoreError, now_timestamp};

const MPF_COLUMNS: &str = "id, label, trustee, contributions, balance, plan_name, \
     member_no, sort_order, seed_max_rate, seed_max_gain, created_at";

/// The stored row, before derived figures are attached.
#[derive(Debug, Clone)]
pub struct StoredAccount {
    pub id: i64,
    pub label: String,
    pub trustee: Option<String>,
    pub contributions: f64,
    pub balance: f64,
    pub plan_name: Option<String>,
    pub member_no: Option<String>,
    pub sort_order: i64,
    pub seed_max_rate: Option<f64>,
    pub seed_max_gain: Option<f64>,
    pub created_on: chrono::NaiveDate,
}

pub fn row_to_stored(row: &SqliteRow) -> Result<StoredAccount, StoreError> {
    let created_at: String = row.try_get("created_at")?;
    let created_on = chrono::DateTime::parse_from_rfc3339(&created_at)
        .map(|stamp| stamp.date_naive())
        .map_err(|_| StoreError::Corrupt(format!("stored created_at {created_at} is not valid")))?;
    Ok(StoredAccount {
        id: row.try_get("id")?,
        label: row.try_get("label")?,
        trustee: row.try_get("trustee")?,
        contributions: row.try_get("contributions")?,
        balance: row.try_get("balance")?,
        plan_name: row.try_get("plan_name")?,
        member_no: row.try_get("member_no")?,
        sort_order: row.try_get("sort_order")?,
        seed_max_rate: row.try_get("seed_max_rate")?,
        seed_max_gain: row.try_get("seed_max_gain")?,
        created_on,
    })
}

fn row_to_history(row: &SqliteRow) -> Result<MpfHistoryRow, StoreError> {
    let contributions: f64 = row.try_get("contributions")?;
    let balance: f64 = row.try_get("balance")?;
    let figures = mpf_figures(contributions, balance);
    Ok(MpfHistoryRow {
        id: row.try_get("id")?,
        account_id: row.try_get("account_id")?,
        recorded_on: row.try_get("recorded_on")?,
        contributions,
        balance,
        synthetic: row.try_get::<i64, _>("synthetic")? != 0,
        rate: figures.rate,
        gain: figures.gain,
    })
}

pub async fn load_stored(pool: &SqlitePool, id: i64) -> Result<StoredAccount, StoreError> {
    let row = sqlx::query(sqlx::AssertSqlSafe(format!(
        "SELECT {MPF_COLUMNS} FROM mpf_accounts WHERE id = ?"
    )))
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| StoreError::NotFound(format!("mpf account {id} not found")))?;
    row_to_stored(&row)
}

pub async fn load_all_stored(pool: &SqlitePool) -> Result<Vec<StoredAccount>, StoreError> {
    let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
        "SELECT {MPF_COLUMNS} FROM mpf_accounts ORDER BY sort_order, id"
    )))
    .fetch_all(pool)
    .await?;
    rows.iter().map(row_to_stored).collect()
}

pub async fn load_history(
    pool: &SqlitePool,
    account_id: Option<i64>,
) -> Result<Vec<MpfHistoryRow>, StoreError> {
    let sql = "SELECT id, account_id, recorded_on, contributions, balance, synthetic \
         FROM mpf_history";
    let rows = match account_id {
        Some(id) => {
            sqlx::query(sqlx::AssertSqlSafe(format!(
                "{sql} WHERE account_id = ? ORDER BY recorded_on, id"
            )))
            .bind(id)
            .fetch_all(pool)
            .await?
        }
        None => {
            sqlx::query(sqlx::AssertSqlSafe(format!(
                "{sql} ORDER BY recorded_on, id"
            )))
            .fetch_all(pool)
            .await?
        }
    };
    rows.iter().map(row_to_history).collect()
}

pub fn facts_of(account: &StoredAccount, history: &[MpfHistoryRow]) -> MpfAccountFacts {
    MpfAccountFacts {
        created_on: account.created_on,
        contributions: account.contributions,
        balance: account.balance,
        seed_max_rate: account.seed_max_rate,
        seed_max_gain: account.seed_max_gain,
        history: history
            .iter()
            .filter(|row| row.account_id == account.id)
            .map(|row| MpfPoint {
                recorded_on: chrono::NaiveDate::parse_from_str(&row.recorded_on, "%Y-%m-%d")
                    .unwrap_or(account.created_on),
                contributions: row.contributions,
                balance: row.balance,
            })
            .collect(),
    }
}

/// Record one contributions/balance update inside an open transaction:
/// backfill synthetic month-end rows for every fully elapsed month with no
/// records (carrying the pre-update values), then upsert today's row so
/// same-day corrections replace rather than append. `fallback_on` (the
/// account's creation date) anchors the gap scan when no history exists yet.
pub async fn record_history(
    conn: &mut sqlx::SqliteConnection,
    account_id: i64,
    previous: (f64, f64),
    current: (f64, f64),
    today: NaiveDate,
    fallback_on: NaiveDate,
) -> Result<(), StoreError> {
    let anchor: Option<String> =
        sqlx::query_scalar("SELECT MAX(recorded_on) FROM mpf_history WHERE account_id = ?")
            .bind(account_id)
            .fetch_one(&mut *conn)
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
        .execute(&mut *conn)
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
    .execute(&mut *conn)
    .await?;
    Ok(())
}

pub async fn next_sort_order(pool: &SqlitePool) -> Result<i64, StoreError> {
    Ok(
        sqlx::query_scalar("SELECT COALESCE(MAX(sort_order), 0) + 1 FROM mpf_accounts")
            .fetch_one(pool)
            .await?,
    )
}

#[allow(clippy::too_many_arguments)]
pub async fn insert_account(
    pool: &SqlitePool,
    label: &str,
    trustee: Option<&str>,
    contributions: f64,
    balance: f64,
    plan_name: Option<&str>,
    member_no: Option<&str>,
    sort_order: i64,
) -> Result<i64, StoreError> {
    let now = now_timestamp();
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO mpf_accounts (label, trustee, contributions, balance, plan_name, \
         member_no, sort_order, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(label)
    .bind(trustee)
    .bind(contributions)
    .bind(balance)
    .bind(plan_name)
    .bind(member_no)
    .bind(sort_order)
    .bind(&now)
    .bind(&now)
    .fetch_one(pool)
    .await?;
    Ok(id)
}

#[allow(clippy::too_many_arguments)]
pub async fn update_account(
    conn: &mut sqlx::SqliteConnection,
    id: i64,
    label: &str,
    trustee: &Option<String>,
    contributions: f64,
    balance: f64,
    plan_name: &Option<String>,
    member_no: &Option<String>,
) -> Result<(), StoreError> {
    sqlx::query(
        "UPDATE mpf_accounts SET label = ?, trustee = ?, contributions = ?, balance = ?, \
         plan_name = ?, member_no = ?, updated_at = ? WHERE id = ?",
    )
    .bind(label)
    .bind(trustee)
    .bind(contributions)
    .bind(balance)
    .bind(plan_name)
    .bind(member_no)
    .bind(now_timestamp())
    .bind(id)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

pub async fn count_history(pool: &SqlitePool, account_id: i64) -> Result<i64, StoreError> {
    Ok(
        sqlx::query_scalar("SELECT COUNT(*) FROM mpf_history WHERE account_id = ?")
            .bind(account_id)
            .fetch_one(pool)
            .await?,
    )
}

pub async fn delete_account(pool: &SqlitePool, id: i64) -> Result<(), StoreError> {
    sqlx::query("DELETE FROM mpf_accounts WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Rows deleted, so the handler can tell "not found" apart from success.
pub async fn delete_history(pool: &SqlitePool, id: i64) -> Result<u64, StoreError> {
    Ok(sqlx::query("DELETE FROM mpf_history WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?
        .rows_affected())
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
