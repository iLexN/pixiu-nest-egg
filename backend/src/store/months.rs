use std::collections::{BTreeMap, HashMap, HashSet};

use sqlx::Row;
use sqlx::SqlitePool;
use sqlx::sqlite::SqliteRow;

use crate::calc::MonthItemFacts;
use crate::models::{ManualAsset, ManualAssetLiquidity, MonthItem, MonthItemCategory};
use crate::store::{StoreError, now_timestamp};

/// The 差餉 figure the forecast falls back to while unset.
pub const DEFAULT_BILL_AMOUNT: f64 = 2158.0;

/// The stored `month_stats` row, before derived columns are attached.
#[derive(Debug, Clone)]
pub struct StoredMonth {
    pub(crate) month: String,
    pub(crate) start_cash: Option<f64>,
    pub(crate) salary: Option<f64>,
    pub(crate) total_assets: Option<f64>,
    pub(crate) liquid_assets: Option<f64>,
    pub(crate) pool_input: f64,
    pub(crate) end_cash_override: Option<f64>,
    pub(crate) note: Option<String>,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

const MONTH_COLUMNS: &str = "month, start_cash, salary, total_assets, liquid_assets, \
     pool_input, end_cash_override, note, created_at, updated_at";

fn row_to_month(row: &SqliteRow) -> Result<StoredMonth, StoreError> {
    Ok(StoredMonth {
        month: row.try_get("month")?,
        start_cash: row.try_get("start_cash")?,
        salary: row.try_get("salary")?,
        total_assets: row.try_get("total_assets")?,
        liquid_assets: row.try_get("liquid_assets")?,
        pool_input: row.try_get("pool_input")?,
        end_cash_override: row.try_get("end_cash_override")?,
        note: row.try_get("note")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

pub(crate) fn parse_stored_month(month: &str) -> Result<chrono::NaiveDate, StoreError> {
    chrono::NaiveDate::parse_from_str(month, "%Y-%m-%d")
        .map_err(|_| StoreError::Corrupt(format!("stored month {month} is not valid")))
}

pub(crate) fn row_to_item(row: &SqliteRow) -> Result<MonthItem, StoreError> {
    let category: String = row.try_get("category")?;
    Ok(MonthItem {
        id: row.try_get("id")?,
        month: row.try_get("month")?,
        category: MonthItemCategory::parse(&category).ok_or_else(|| {
            StoreError::Corrupt(format!("stored category {category} is not valid"))
        })?,
        label: row.try_get("label")?,
        amount: row.try_get("amount")?,
        exclude_from_living: row.try_get::<i64, _>("exclude_from_living")? != 0,
        auto_key: row.try_get("auto_key")?,
        note: row.try_get("note")?,
        created_at: row.try_get("created_at")?,
    })
}

const ITEM_COLUMNS: &str =
    "id, month, category, label, amount, exclude_from_living, auto_key, note, created_at";

pub(crate) async fn load_all_months(pool: &SqlitePool) -> Result<Vec<StoredMonth>, StoreError> {
    let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
        "SELECT {MONTH_COLUMNS} FROM month_stats ORDER BY month"
    )))
    .fetch_all(pool)
    .await?;
    rows.iter().map(row_to_month).collect()
}

pub(crate) async fn load_month(
    pool: &SqlitePool,
    month: &str,
) -> Result<Option<StoredMonth>, StoreError> {
    let row = sqlx::query(sqlx::AssertSqlSafe(format!(
        "SELECT {MONTH_COLUMNS} FROM month_stats WHERE month = ?"
    )))
    .bind(month)
    .fetch_optional(pool)
    .await?;
    row.as_ref().map(row_to_month).transpose()
}

pub async fn load_all_items(
    pool: &SqlitePool,
) -> Result<HashMap<chrono::NaiveDate, Vec<MonthItemFacts>>, StoreError> {
    let rows = sqlx::query(
        "SELECT month, category, amount, exclude_from_living FROM month_items \
         ORDER BY month, id",
    )
    .fetch_all(pool)
    .await?;
    let mut items: HashMap<chrono::NaiveDate, Vec<MonthItemFacts>> = HashMap::new();
    for row in &rows {
        let month: String = row.try_get("month")?;
        let category: String = row.try_get("category")?;
        let category = MonthItemCategory::parse(&category).ok_or_else(|| {
            StoreError::Corrupt(format!("stored category {category} is not valid"))
        })?;
        items
            .entry(parse_stored_month(&month)?)
            .or_default()
            .push(MonthItemFacts {
                category,
                amount: row.try_get("amount")?,
                exclude_from_living: row.try_get::<i64, _>("exclude_from_living")? != 0,
            });
    }
    Ok(items)
}

/// The per-year 開心Pool rates (`pool_rates` table).
pub(crate) async fn pool_rates(pool: &SqlitePool) -> Result<BTreeMap<i32, f64>, StoreError> {
    let rows = sqlx::query("SELECT year, rate FROM pool_rates")
        .fetch_all(pool)
        .await?;
    let mut rates = BTreeMap::new();
    for row in &rows {
        rates.insert(row.try_get("year")?, row.try_get("rate")?);
    }
    Ok(rates)
}

/// The stored rate for one year; `None` while unset.
pub async fn pool_rate(pool: &SqlitePool, year: i32) -> Result<Option<f64>, StoreError> {
    Ok(
        sqlx::query_scalar("SELECT rate FROM pool_rates WHERE year = ?")
            .bind(year)
            .fetch_optional(pool)
            .await?,
    )
}

/// Store (or clear, on `None`) the rate for one year.
pub async fn set_pool_rate(
    pool: &SqlitePool,
    year: i32,
    rate: Option<f64>,
) -> Result<(), StoreError> {
    match rate {
        Some(rate) => {
            sqlx::query(
                "INSERT INTO pool_rates (year, rate) VALUES (?, ?) \
                 ON CONFLICT (year) DO UPDATE SET rate = excluded.rate",
            )
            .bind(year)
            .bind(rate)
            .execute(pool)
            .await?;
        }
        None => {
            sqlx::query("DELETE FROM pool_rates WHERE year = ?")
                .bind(year)
                .execute(pool)
                .await?;
        }
    }
    Ok(())
}

/// Each year's last stored month salary — the input the derived 月薪增幅
/// compares against the prior year's.
pub(crate) async fn last_salaries_by_year(
    pool: &SqlitePool,
) -> Result<BTreeMap<i32, f64>, StoreError> {
    let rows = sqlx::query(
        "SELECT month, salary FROM month_stats WHERE salary IS NOT NULL ORDER BY month",
    )
    .fetch_all(pool)
    .await?;
    let mut map = BTreeMap::new();
    for row in &rows {
        let month: String = row.try_get("month")?;
        let year: i32 = month
            .get(..4)
            .and_then(|y| y.parse().ok())
            .ok_or_else(|| StoreError::Corrupt(format!("stored month {month} is not valid")))?;
        map.insert(year, row.try_get::<f64, _>("salary")?);
    }
    Ok(map)
}

/// Σ amount over the `manual_assets` asset rows.
pub(crate) async fn manual_asset_sum(pool: &SqlitePool) -> Result<f64, StoreError> {
    Ok(
        sqlx::query_scalar("SELECT COALESCE(SUM(amount), 0.0) FROM manual_assets")
            .fetch_one(pool)
            .await?,
    )
}

pub(crate) async fn load_items(
    pool: &SqlitePool,
    month: &str,
) -> Result<Vec<MonthItem>, StoreError> {
    let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
        "SELECT {ITEM_COLUMNS} FROM month_items WHERE month = ? ORDER BY id"
    )))
    .bind(month)
    .fetch_all(pool)
    .await?;
    rows.iter().map(row_to_item).collect()
}

pub(crate) async fn dismissed_keys(
    pool: &SqlitePool,
    month: &str,
) -> Result<HashSet<String>, StoreError> {
    let keys = sqlx::query_scalar::<_, String>(
        "SELECT auto_key FROM month_item_dismissals WHERE month = ?",
    )
    .bind(month)
    .fetch_all(pool)
    .await?;
    Ok(keys.into_iter().collect())
}

/// Insert a new month_stats row (the route computes the live snapshot first).
#[allow(clippy::too_many_arguments)]
pub(crate) async fn insert_month(
    pool: &SqlitePool,
    month_key: &str,
    start_cash: Option<f64>,
    salary: Option<f64>,
    total_assets: Option<f64>,
    liquid_assets: Option<f64>,
    pool_input: f64,
    end_cash_override: Option<f64>,
    note: Option<String>,
    now: &str,
) -> Result<(), StoreError> {
    sqlx::query(
        "INSERT INTO month_stats (month, start_cash, salary, total_assets, \
         liquid_assets, pool_input, \
         end_cash_override, note, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(month_key)
    .bind(start_cash)
    .bind(salary)
    .bind(total_assets)
    .bind(liquid_assets)
    .bind(pool_input)
    .bind(end_cash_override)
    .bind(note)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(())
}

/// Update an existing month_stats row.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn update_month(
    pool: &SqlitePool,
    month_key: &str,
    start_cash: Option<f64>,
    salary: Option<f64>,
    total_assets: Option<f64>,
    liquid_assets: Option<f64>,
    pool_input: f64,
    end_cash_override: Option<f64>,
    note: Option<String>,
    now: &str,
) -> Result<(), StoreError> {
    sqlx::query(
        "UPDATE month_stats SET start_cash = ?, salary = ?, total_assets = ?, \
         liquid_assets = ?, pool_input = ?, \
         end_cash_override = ?, note = ?, updated_at = ? WHERE month = ?",
    )
    .bind(start_cash)
    .bind(salary)
    .bind(total_assets)
    .bind(liquid_assets)
    .bind(pool_input)
    .bind(end_cash_override)
    .bind(note)
    .bind(now)
    .bind(month_key)
    .execute(pool)
    .await?;
    Ok(())
}

/// Rows deleted, so the handler can tell "not found" apart from success.
pub(crate) async fn delete_month(pool: &SqlitePool, month_key: &str) -> Result<u64, StoreError> {
    Ok(sqlx::query("DELETE FROM month_stats WHERE month = ?")
        .bind(month_key)
        .execute(pool)
        .await?
        .rows_affected())
}

/// Accepting a suggestion clears any dismissal of the same key.
pub(crate) async fn clear_dismissal(
    conn: &mut sqlx::SqliteConnection,
    month: &str,
    auto_key: &str,
) -> Result<(), StoreError> {
    sqlx::query("DELETE FROM month_item_dismissals WHERE month = ? AND auto_key = ?")
        .bind(month)
        .bind(auto_key)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

/// The item's new id. A duplicate (month, auto_key) hits the partial unique
/// index.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn insert_item(
    conn: &mut sqlx::SqliteConnection,
    month: &str,
    category: &str,
    label: &Option<String>,
    amount: f64,
    exclude_from_living: bool,
    auto_key: &Option<String>,
    note: &Option<String>,
) -> Result<i64, StoreError> {
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO month_items (month, category, label, amount, exclude_from_living, \
         auto_key, note, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(month)
    .bind(category)
    .bind(label)
    .bind(amount)
    .bind(exclude_from_living as i64)
    .bind(auto_key)
    .bind(note)
    .bind(now_timestamp())
    .fetch_one(&mut *conn)
    .await?;
    Ok(id)
}

pub(crate) async fn load_item(pool: &SqlitePool, id: i64) -> Result<MonthItem, StoreError> {
    let row = sqlx::query(sqlx::AssertSqlSafe(format!(
        "SELECT {ITEM_COLUMNS} FROM month_items WHERE id = ?"
    )))
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| StoreError::NotFound(format!("month item {id} not found")))?;
    row_to_item(&row)
}

/// Like `load_item` but `fetch_one` — used right after the row was written.
pub(crate) async fn item_row(pool: &SqlitePool, id: i64) -> Result<MonthItem, StoreError> {
    let row = sqlx::query(sqlx::AssertSqlSafe(format!(
        "SELECT {ITEM_COLUMNS} FROM month_items WHERE id = ?"
    )))
    .bind(id)
    .fetch_one(pool)
    .await?;
    row_to_item(&row)
}

pub(crate) async fn update_item(
    pool: &SqlitePool,
    id: i64,
    category: &str,
    label: &Option<String>,
    amount: f64,
    exclude_from_living: bool,
    note: &Option<String>,
) -> Result<(), StoreError> {
    sqlx::query(
        "UPDATE month_items SET category = ?, label = ?, amount = ?, \
         exclude_from_living = ?, note = ? WHERE id = ?",
    )
    .bind(category)
    .bind(label)
    .bind(amount)
    .bind(exclude_from_living as i64)
    .bind(note)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Rows deleted, so the handler can tell "not found" apart from success.
pub(crate) async fn delete_item(pool: &SqlitePool, id: i64) -> Result<u64, StoreError> {
    Ok(sqlx::query("DELETE FROM month_items WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?
        .rows_affected())
}

/// Tombstone a suggestion key so it never reappears for the month.
pub(crate) async fn dismiss_item(
    pool: &SqlitePool,
    month: &str,
    auto_key: &str,
) -> Result<(), StoreError> {
    sqlx::query(
        "INSERT OR IGNORE INTO month_item_dismissals (month, auto_key, created_at) \
         VALUES (?, ?, ?)",
    )
    .bind(month)
    .bind(auto_key)
    .bind(now_timestamp())
    .execute(pool)
    .await?;
    Ok(())
}

fn row_to_asset(row: &SqliteRow) -> Result<ManualAsset, StoreError> {
    let liquidity: String = row.try_get("liquidity")?;
    Ok(ManualAsset {
        id: row.try_get("id")?,
        label: row.try_get("label")?,
        liquidity: ManualAssetLiquidity::parse(&liquidity).ok_or_else(|| {
            StoreError::Corrupt(format!("stored liquidity {liquidity} is not valid"))
        })?,
        amount: row.try_get("amount")?,
        sort_order: row.try_get("sort_order")?,
        updated_at: row.try_get("updated_at")?,
    })
}

const ASSET_COLUMNS: &str = "id, label, liquidity, amount, sort_order, updated_at";

pub(crate) async fn load_asset(pool: &SqlitePool, id: i64) -> Result<ManualAsset, StoreError> {
    let row = sqlx::query(sqlx::AssertSqlSafe(format!(
        "SELECT {ASSET_COLUMNS} FROM manual_assets WHERE id = ?"
    )))
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| StoreError::NotFound(format!("manual asset {id} not found")))?;
    row_to_asset(&row)
}

pub async fn load_assets(pool: &SqlitePool) -> Result<Vec<ManualAsset>, StoreError> {
    let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
        "SELECT {ASSET_COLUMNS} FROM manual_assets ORDER BY sort_order, id"
    )))
    .fetch_all(pool)
    .await?;
    rows.iter().map(row_to_asset).collect::<Result<Vec<_>, _>>()
}

pub(crate) async fn next_asset_sort_order(pool: &SqlitePool) -> Result<i64, StoreError> {
    Ok(
        sqlx::query_scalar("SELECT COALESCE(MAX(sort_order), 0) + 1 FROM manual_assets")
            .fetch_one(pool)
            .await?,
    )
}

pub(crate) async fn insert_asset(
    pool: &SqlitePool,
    label: &str,
    liquidity: &str,
    amount: f64,
    sort_order: i64,
) -> Result<i64, StoreError> {
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO manual_assets (label, liquidity, amount, sort_order, updated_at) \
         VALUES (?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(label)
    .bind(liquidity)
    .bind(amount)
    .bind(sort_order)
    .bind(now_timestamp())
    .fetch_one(pool)
    .await?;
    Ok(id)
}

pub(crate) async fn update_asset(
    pool: &SqlitePool,
    id: i64,
    label: &str,
    liquidity: &str,
    amount: f64,
) -> Result<(), StoreError> {
    sqlx::query(
        "UPDATE manual_assets SET label = ?, liquidity = ?, amount = ?, updated_at = ? \
         WHERE id = ?",
    )
    .bind(label)
    .bind(liquidity)
    .bind(amount)
    .bind(now_timestamp())
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Rows deleted, so the handler can tell "not found" apart from success.
pub(crate) async fn delete_asset(pool: &SqlitePool, id: i64) -> Result<u64, StoreError> {
    Ok(sqlx::query("DELETE FROM manual_assets WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?
        .rows_affected())
}

/// Record an auto `month_items` row for a received amount — bond ends,
/// coupons, deposits all use it. An already-stored keyed item (e.g. an
/// accepted pending suggestion) is refreshed to the actual amount; a
/// monthless month records nothing. Inside the caller's transaction.
pub async fn record_receipt_item(
    conn: &mut sqlx::SqliteConnection,
    month: &str,
    auto_key: &str,
    label: &str,
    amount: f64,
) -> Result<(), StoreError> {
    let item_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM month_items WHERE month = ? AND auto_key = ?)",
    )
    .bind(month)
    .bind(auto_key)
    .fetch_one(&mut *conn)
    .await?;
    let month_exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM month_stats WHERE month = ?)")
            .bind(month)
            .fetch_one(&mut *conn)
            .await?;
    if item_exists {
        sqlx::query(
            "UPDATE month_items SET label = ?, amount = ? WHERE month = ? AND auto_key = ?",
        )
        .bind(label)
        .bind(amount)
        .bind(month)
        .bind(auto_key)
        .execute(&mut *conn)
        .await?;
    } else if month_exists {
        sqlx::query(
            "INSERT INTO month_items (month, category, label, amount, auto_key, created_at) \
             VALUES (?, 'adjustment', ?, ?, ?, ?)",
        )
        .bind(month)
        .bind(label)
        .bind(amount)
        .bind(auto_key)
        .bind(now_timestamp())
        .execute(&mut *conn)
        .await?;
    }
    sqlx::query("DELETE FROM month_item_dismissals WHERE month = ? AND auto_key = ?")
        .bind(month)
        .bind(auto_key)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

/// Drop the auto `month_items` row for one key, inside the caller's
/// transaction.
pub async fn delete_auto_item(
    conn: &mut sqlx::SqliteConnection,
    month: &str,
    auto_key: &str,
) -> Result<(), StoreError> {
    sqlx::query("DELETE FROM month_items WHERE month = ? AND auto_key = ?")
        .bind(month)
        .bind(auto_key)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connect_memory;

    /// An already-stored keyed item (an accepted pending suggestion) is
    /// refreshed to the receipt amount; a monthless month writes nothing.
    #[tokio::test]
    async fn receipt_item_refresh_updates_the_stored_row() {
        let pool = connect_memory().await.expect("memory db");
        sqlx::query(
            "INSERT INTO month_stats (month, created_at, updated_at) \
             VALUES ('2026-10-01', '2026-01-01T00:00:00', '2026-01-01T00:00:00')",
        )
        .execute(&pool)
        .await
        .unwrap();

        let mut tx = pool.begin().await.unwrap();
        record_receipt_item(&mut tx, "2026-10-01", "coupon:4", "silver bond", 1002.75)
            .await
            .unwrap();
        // The 收訖 write with the 實收 amount refreshes the stored row.
        record_receipt_item(&mut tx, "2026-10-01", "coupon:4", "silver bond", 1000.0)
            .await
            .unwrap();
        tx.commit().await.unwrap();

        let (label, amount): (String, f64) = sqlx::query_as(
            "SELECT label, amount FROM month_items \
             WHERE month = '2026-10-01' AND auto_key = 'coupon:4'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!((label.as_str(), amount), ("silver bond", 1000.0));
    }

    #[tokio::test]
    async fn receipt_item_monthless_writes_nothing() {
        let pool = connect_memory().await.expect("memory db");
        let mut tx = pool.begin().await.unwrap();
        record_receipt_item(&mut tx, "2027-01-01", "coupon:9", "bond", 5.0)
            .await
            .unwrap();
        tx.commit().await.unwrap();

        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM month_items WHERE auto_key = 'coupon:9'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(count, 0);
    }
}
