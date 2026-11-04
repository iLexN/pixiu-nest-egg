//! The `ibkr_transfers` log and the reported IBKR totals — the 美股 sheet's
//! IBKR account block (A1:B5). The cash figures are the `broker`
//! cash_accounts rows; the reported total is the `ibkr.reported_value`
//! settings key.

use std::collections::BTreeMap;

use sqlx::Row;
use sqlx::sqlite::SqliteRow;

use crate::calc::SuggestionIbkrTransfer;
use crate::models::IbkrTransfer;
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

/// Append a dated transfer row and return its id. `from_account_id` /
/// `broker_account_id` record the accounts a cash-moving transfer touched;
/// both `None` writes a log-only row.
pub async fn insert_transfer<'e, E>(
    exec: E,
    transfer_date: &str,
    amount_hkd: f64,
    from_account_id: Option<i64>,
    broker_account_id: Option<i64>,
) -> Result<i64, StoreError>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    let id = sqlx::query_scalar(
        "INSERT INTO ibkr_transfers \
         (transfer_date, amount_hkd, from_account_id, broker_account_id, created_at) \
         VALUES (?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(transfer_date)
    .bind(amount_hkd)
    .bind(from_account_id)
    .bind(broker_account_id)
    .bind(now_timestamp())
    .fetch_one(exec)
    .await?;
    Ok(id)
}

const TRANSFER_COLUMNS: &str = "t.id, t.transfer_date, t.amount_hkd, \
     t.from_account_id, a.label AS from_account_label, t.broker_account_id, \
     t.created_at";

fn row_to_transfer(row: &SqliteRow) -> Result<IbkrTransfer, StoreError> {
    Ok(IbkrTransfer {
        id: row.try_get("id")?,
        transfer_date: row.try_get("transfer_date")?,
        amount_hkd: row.try_get("amount_hkd")?,
        from_account_id: row.try_get("from_account_id")?,
        from_account_label: row.try_get("from_account_label")?,
        broker_account_id: row.try_get("broker_account_id")?,
        created_at: row.try_get("created_at")?,
    })
}

/// Every log row, newest `transfer_date` then newest id first; the source
/// label joins `cash_accounts` (NULL for log-only rows and deleted sources).
pub async fn list_transfers(pool: &sqlx::SqlitePool) -> Result<Vec<IbkrTransfer>, StoreError> {
    let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
        "SELECT {TRANSFER_COLUMNS} FROM ibkr_transfers t \
         LEFT JOIN cash_accounts a ON a.id = t.from_account_id \
         ORDER BY t.transfer_date DESC, t.id DESC"
    )))
    .fetch_all(pool)
    .await?;
    rows.iter().map(row_to_transfer).collect()
}

/// `Err(NotFound)` while the id names no transfer.
pub async fn load_transfer<'e, E>(exec: E, id: i64) -> Result<IbkrTransfer, StoreError>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    let row = sqlx::query(sqlx::AssertSqlSafe(format!(
        "SELECT {TRANSFER_COLUMNS} FROM ibkr_transfers t \
         LEFT JOIN cash_accounts a ON a.id = t.from_account_id WHERE t.id = ?"
    )))
    .bind(id)
    .fetch_optional(exec)
    .await?
    .ok_or_else(|| StoreError::NotFound(format!("ibkr transfer {id} not found")))?;
    row_to_transfer(&row)
}

/// Drop the row, inside the caller's transaction.
pub async fn delete_transfer(conn: &mut sqlx::SqliteConnection, id: i64) -> Result<(), StoreError> {
    sqlx::query("DELETE FROM ibkr_transfers WHERE id = ?")
        .bind(id)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

/// Transfers dated in `[first, next)` as suggestion/forecast sources. The
/// 月結 candidates pass `cash_moving_only` (rows carrying a source account —
/// log-only rows already have their manual items); the forecast passes
/// `false` because every recorded transfer moved cash.
pub async fn transfers_in_range(
    pool: &sqlx::SqlitePool,
    first: &str,
    next: &str,
    cash_moving_only: bool,
) -> Result<Vec<SuggestionIbkrTransfer>, StoreError> {
    let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
        "SELECT id, transfer_date, amount_hkd FROM ibkr_transfers \
         WHERE transfer_date >= ? AND transfer_date < ?{}",
        if cash_moving_only {
            " AND from_account_id IS NOT NULL"
        } else {
            ""
        }
    )))
    .bind(first)
    .bind(next)
    .fetch_all(pool)
    .await?;
    let mut transfers = Vec::with_capacity(rows.len());
    for row in &rows {
        let transfer_date: String = row.try_get("transfer_date")?;
        transfers.push(SuggestionIbkrTransfer {
            id: row.try_get("id")?,
            date: chrono::NaiveDate::parse_from_str(&transfer_date, "%Y-%m-%d").map_err(|_| {
                StoreError::Corrupt(format!("stored date {transfer_date} is not valid"))
            })?,
            amount: row.try_get("amount_hkd")?,
        });
    }
    Ok(transfers)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connect_memory;
    use crate::models::Currency;
    use crate::store::cash_accounts;

    async fn seed_bank(pool: &sqlx::SqlitePool) -> i64 {
        sqlx::query_scalar(
            "INSERT INTO cash_accounts (label, currency, role, amount, \
             is_default_receipt, sort_order, updated_at) \
             VALUES ('HS', 'HKD', 'bank', 1000.0, 1, 100, '2026-01-01T00:00:00') \
             RETURNING id",
        )
        .fetch_one(pool)
        .await
        .unwrap()
    }

    async fn broker_id(pool: &sqlx::SqlitePool) -> i64 {
        cash_accounts::broker(pool, Currency::Hkd)
            .await
            .unwrap()
            .expect("broker account")
            .id
    }

    /// Newest `transfer_date` then newest id first; account-less rows (the
    /// legacy shape) read NULL account fields and no label.
    #[tokio::test]
    async fn list_orders_newest_first_and_legacy_rows_read_null() {
        let pool = connect_memory().await.expect("memory db");
        sqlx::query(
            "INSERT INTO ibkr_transfers (transfer_date, amount_hkd, created_at) \
             VALUES ('2026-06-02', 131000, '2026-01-01T00:00:00'), \
                    ('2026-10-30', 90000, '2026-01-01T00:00:00')",
        )
        .execute(&pool)
        .await
        .unwrap();
        let hs = seed_bank(&pool).await;
        let broker = broker_id(&pool).await;
        let older = insert_transfer(&pool, "2026-11-03", 10000.0, Some(hs), Some(broker))
            .await
            .unwrap();
        let newer = insert_transfer(&pool, "2026-11-03", 50000.0, Some(hs), Some(broker))
            .await
            .unwrap();

        let rows = list_transfers(&pool).await.unwrap();
        assert_eq!(rows.len(), 4);
        // Same date: the newer id wins; then by date descending.
        assert_eq!(
            rows.iter().map(|row| row.id).collect::<Vec<_>>(),
            vec![newer, older, rows[2].id, rows[3].id]
        );
        assert_eq!(rows[0].from_account_label.as_deref(), Some("HS"));
        assert_eq!(rows[0].broker_account_id, Some(broker));
        assert_eq!(rows[2].transfer_date, "2026-10-30");
        assert_eq!(rows[3].transfer_date, "2026-06-02");
        for legacy in &rows[2..] {
            assert!(legacy.from_account_id.is_none());
            assert!(legacy.from_account_label.is_none());
            assert!(legacy.broker_account_id.is_none());
        }
    }

    /// `load_transfer` is `NotFound` on unknown ids, and again after delete.
    #[tokio::test]
    async fn load_transfer_not_found_then_deleted() {
        let pool = connect_memory().await.expect("memory db");
        assert!(matches!(
            load_transfer(&pool, 9999).await,
            Err(StoreError::NotFound(_))
        ));

        let id = insert_transfer(&pool, "2026-11-03", 50000.0, None, None)
            .await
            .unwrap();
        let row = load_transfer(&pool, id).await.unwrap();
        assert_eq!(row.amount_hkd, 50000.0);
        assert!(row.from_account_id.is_none());

        let mut conn = pool.acquire().await.unwrap();
        delete_transfer(&mut conn, id).await.unwrap();
        drop(conn);
        assert!(matches!(
            load_transfer(&pool, id).await,
            Err(StoreError::NotFound(_))
        ));
    }

    /// The range loader buckets by `transfer_date`; `cash_moving_only` drops
    /// rows with no source account.
    #[tokio::test]
    async fn transfers_in_range_filters_dates_and_cash_moving() {
        let pool = connect_memory().await.expect("memory db");
        let hs = seed_bank(&pool).await;
        let broker = broker_id(&pool).await;
        let cash = insert_transfer(&pool, "2026-10-03", 50000.0, Some(hs), Some(broker))
            .await
            .unwrap();
        insert_transfer(&pool, "2026-10-30", 90000.0, None, None)
            .await
            .unwrap();
        insert_transfer(&pool, "2026-11-01", 1.0, Some(hs), Some(broker))
            .await
            .unwrap();

        let all = transfers_in_range(&pool, "2026-10-01", "2026-11-01", false)
            .await
            .unwrap();
        assert_eq!(all.len(), 2);
        let cash_only = transfers_in_range(&pool, "2026-10-01", "2026-11-01", true)
            .await
            .unwrap();
        assert_eq!(cash_only.len(), 1);
        assert_eq!(cash_only[0].id, cash);
        assert_eq!(cash_only[0].amount, 50000.0);
    }
}
