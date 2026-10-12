use std::collections::{BTreeMap, HashMap};

use sqlx::Row;
use sqlx::SqlitePool;

use crate::calc::YearSnapshot;
use crate::models::Market;
use crate::store::{StoreError, now_timestamp};

/// The stored year snapshots keyed by year.
pub async fn load_snapshots(
    pool: &SqlitePool,
    market: Market,
) -> Result<HashMap<i32, YearSnapshot>, StoreError> {
    let rows = sqlx::query(
        "SELECT year, invested, cost, market_value, sold_pl, updated_at \
         FROM year_snapshots WHERE market = ?",
    )
    .bind(market.as_str())
    .fetch_all(pool)
    .await?;

    let mut snapshots = HashMap::with_capacity(rows.len());
    for row in &rows {
        snapshots.insert(
            row.try_get("year")?,
            YearSnapshot {
                invested: row.try_get("invested")?,
                cost: row.try_get("cost")?,
                market_value: row.try_get("market_value")?,
                sold_pl: row.try_get("sold_pl")?,
                updated_at: row.try_get("updated_at")?,
            },
        );
    }
    Ok(snapshots)
}

pub async fn load_snapshot(
    pool: &SqlitePool,
    market: Market,
    year: i32,
) -> Result<Option<crate::models::YearSnapshot>, StoreError> {
    let row = sqlx::query(
        "SELECT invested, cost, market_value, sold_pl, updated_at \
         FROM year_snapshots WHERE market = ? AND year = ?",
    )
    .bind(market.as_str())
    .bind(year)
    .fetch_optional(pool)
    .await?;
    row.map(|row| {
        Ok(crate::models::YearSnapshot {
            market,
            year,
            invested: row.try_get("invested")?,
            cost: row.try_get("cost")?,
            market_value: row.try_get("market_value")?,
            sold_pl: row.try_get("sold_pl")?,
            updated_at: row.try_get("updated_at")?,
        })
    })
    .transpose()
}

/// HK `sold_pl` per year — feeds 投資純利 in the Month Stat yearly block and
/// 投資P/L in the year review.
pub async fn hk_sold_pl(pool: &SqlitePool) -> Result<BTreeMap<i32, f64>, StoreError> {
    let rows = sqlx::query(
        "SELECT year, sold_pl FROM year_snapshots \
         WHERE market = 'HK' AND sold_pl IS NOT NULL",
    )
    .fetch_all(pool)
    .await?;
    let mut map = BTreeMap::new();
    for row in &rows {
        map.insert(row.try_get("year")?, row.try_get("sold_pl")?);
    }
    Ok(map)
}

/// Clearing the last stored figure removes the row entirely.
pub async fn delete_snapshot(
    pool: &SqlitePool,
    market: Market,
    year: i32,
) -> Result<(), StoreError> {
    sqlx::query("DELETE FROM year_snapshots WHERE market = ? AND year = ?")
        .bind(market.as_str())
        .bind(year)
        .execute(pool)
        .await?;
    Ok(())
}

/// Upsert the full figure set for one snapshot row.
pub async fn upsert_snapshot(
    pool: &SqlitePool,
    market: Market,
    year: i32,
    invested: Option<f64>,
    cost: Option<f64>,
    market_value: Option<f64>,
    sold_pl: Option<f64>,
) -> Result<(), StoreError> {
    sqlx::query(
        "INSERT INTO year_snapshots (market, year, invested, cost, market_value, sold_pl, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?) \
         ON CONFLICT (market, year) DO UPDATE SET \
         invested = excluded.invested, cost = excluded.cost, \
         market_value = excluded.market_value, sold_pl = excluded.sold_pl, \
         updated_at = excluded.updated_at",
    )
    .bind(market.as_str())
    .bind(year)
    .bind(invested)
    .bind(cost)
    .bind(market_value)
    .bind(sold_pl)
    .bind(now_timestamp())
    .execute(pool)
    .await?;
    Ok(())
}

/// Computed year-end cost: Σ BUY total on or before Dec 31 of the year.
pub async fn buy_cost_sum(pool: &SqlitePool, market: Market, year: i32) -> Result<f64, StoreError> {
    Ok(sqlx::query_scalar(
        "SELECT COALESCE(SUM(t.total), 0.0) FROM trades t \
         JOIN stocks s ON s.id = t.stock_id \
         WHERE s.market = ? AND t.trade_type = 'BUY' AND t.trade_date <= ?",
    )
    .bind(market.as_str())
    .bind(format!("{year:04}-12-31"))
    .fetch_one(pool)
    .await?)
}

/// Freeze the year: store the computed cumulative 成本 and the live 總市值;
/// `invested` is left untouched.
pub async fn upsert_frozen(
    pool: &SqlitePool,
    market: Market,
    year: i32,
    cost: f64,
    market_value: Option<f64>,
) -> Result<(), StoreError> {
    sqlx::query(
        "INSERT INTO year_snapshots (market, year, cost, market_value, updated_at) \
         VALUES (?, ?, ?, ?, ?) \
         ON CONFLICT (market, year) DO UPDATE SET \
         cost = excluded.cost, market_value = excluded.market_value, \
         updated_at = excluded.updated_at",
    )
    .bind(market.as_str())
    .bind(year)
    .bind(cost)
    .bind(market_value)
    .bind(now_timestamp())
    .execute(pool)
    .await?;
    Ok(())
}

/// Set the HK `sold_pl` figure, inside the caller's transaction.
pub async fn upsert_hk_sold_pl(
    conn: &mut sqlx::SqliteConnection,
    year: i32,
    sold_pl: Option<f64>,
) -> Result<(), StoreError> {
    sqlx::query(
        "INSERT INTO year_snapshots (market, year, sold_pl, updated_at) \
         VALUES ('HK', ?, ?, ?) \
         ON CONFLICT (market, year) DO UPDATE SET \
         sold_pl = excluded.sold_pl, updated_at = excluded.updated_at",
    )
    .bind(year)
    .bind(sold_pl)
    .bind(now_timestamp())
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// A snapshot left with nothing stored goes away, like the yearly PATCH's
/// all-cleared case. Inside the caller's transaction.
pub async fn delete_empty_hk_snapshot(
    conn: &mut sqlx::SqliteConnection,
    year: i32,
) -> Result<(), StoreError> {
    sqlx::query(
        "DELETE FROM year_snapshots WHERE market = 'HK' AND year = ? \
         AND invested IS NULL AND cost IS NULL AND market_value IS NULL AND sold_pl IS NULL",
    )
    .bind(year)
    .execute(&mut *conn)
    .await?;
    Ok(())
}
