//! Per-market totals history: the recording side of the 上月/最高 figures.
//! Derivation reuses the MPF math in `calc.rs` unchanged — `load` maps each
//! row to `MpfPoint` with `buy_cost_priced` as `contributions` and
//! `market_value` as `balance`.

use chrono::NaiveDate;
use sqlx::{Row, SqlitePool};

use crate::calc::{mpf_gap_month_ends, MpfPoint};
use crate::models::Market;

/// `app_meta` key holding the workbook's seeded max 未實現報酬率 for a
/// market — a floor for the derived 最高, exactly like `mpf.seed_max_rate`.
pub fn seed_max_percent_key(market: Market) -> String {
    format!("market.{}.seed_max_percent", market.as_str())
}

/// `app_meta` key holding the workbook's seeded max 未實現金額 for a market.
pub fn seed_max_amount_key(market: Market) -> String {
    format!("market.{}.seed_max_amount", market.as_str())
}

/// Record the market's totals for `today`: backfill a synthetic month-end
/// row for every fully elapsed month with no records (carrying the last
/// stored values), then upsert today's row so same-day rebuilds replace
/// rather than append. With no previous record there is no backfill —
/// nothing is fabricated for months that predate any observation.
pub async fn record(
    pool: &SqlitePool,
    market: Market,
    current: (f64, f64),
    today: NaiveDate,
) -> Result<(), sqlx::Error> {
    let anchor = sqlx::query(
        "SELECT recorded_on, buy_cost_priced, market_value FROM market_history \
         WHERE market = ? ORDER BY recorded_on DESC, id DESC LIMIT 1",
    )
    .bind(market.as_str())
    .fetch_optional(pool)
    .await?;

    if let Some(anchor) = anchor {
        let recorded_on: String = anchor.try_get("recorded_on")?;
        let anchor_date = NaiveDate::parse_from_str(&recorded_on, "%Y-%m-%d")
            .map_err(|err| sqlx::Error::Decode(Box::new(err)))?;
        let previous: (f64, f64) = (
            anchor.try_get("buy_cost_priced")?,
            anchor.try_get("market_value")?,
        );
        for date in mpf_gap_month_ends(anchor_date, today) {
            sqlx::query(
                "INSERT OR IGNORE INTO market_history \
                 (market, recorded_on, buy_cost_priced, market_value, synthetic) \
                 VALUES (?, ?, ?, ?, 1)",
            )
            .bind(market.as_str())
            .bind(date.to_string())
            .bind(previous.0)
            .bind(previous.1)
            .execute(pool)
            .await?;
        }
    }

    sqlx::query(
        "INSERT INTO market_history \
         (market, recorded_on, buy_cost_priced, market_value, synthetic) \
         VALUES (?, ?, ?, ?, 0) \
         ON CONFLICT (market, recorded_on) DO UPDATE SET \
         buy_cost_priced = excluded.buy_cost_priced, \
         market_value = excluded.market_value, synthetic = 0",
    )
    .bind(market.as_str())
    .bind(today.to_string())
    .bind(current.0)
    .bind(current.1)
    .execute(pool)
    .await?;
    Ok(())
}

/// The market's recorded totals as `MpfPoint`s so `mpf_last_month` and
/// `mpf_max` apply unchanged.
pub async fn load(pool: &SqlitePool, market: Market) -> Result<Vec<MpfPoint>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT recorded_on, buy_cost_priced, market_value FROM market_history \
         WHERE market = ? ORDER BY recorded_on, id",
    )
    .bind(market.as_str())
    .fetch_all(pool)
    .await?;

    let mut points = Vec::with_capacity(rows.len());
    for row in &rows {
        let recorded_on: String = row.try_get("recorded_on")?;
        points.push(MpfPoint {
            recorded_on: NaiveDate::parse_from_str(&recorded_on, "%Y-%m-%d")
                .map_err(|err| sqlx::Error::Decode(Box::new(err)))?,
            contributions: row.try_get("buy_cost_priced")?,
            balance: row.try_get("market_value")?,
        });
    }
    Ok(points)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connect_memory;

    fn day(text: &str) -> NaiveDate {
        NaiveDate::parse_from_str(text, "%Y-%m-%d").unwrap()
    }

    async fn record(pool: &SqlitePool, current: (f64, f64), today: &str) {
        super::record(pool, Market::Hk, current, day(today))
            .await
            .expect("record");
    }

    async fn history(pool: &SqlitePool) -> Vec<(String, f64, f64, i64)> {
        sqlx::query_as(
            "SELECT recorded_on, buy_cost_priced, market_value, synthetic \
             FROM market_history WHERE market = 'HK' ORDER BY recorded_on",
        )
        .fetch_all(pool)
        .await
        .expect("read history")
    }

    #[tokio::test]
    async fn first_record_writes_no_backfill() {
        let pool = connect_memory().await.unwrap();

        record(&pool, (100.0, 150.0), "2026-10-05").await;

        let rows = history(&pool).await;
        assert_eq!(rows, vec![("2026-10-05".to_string(), 100.0, 150.0, 0)]);
    }

    #[tokio::test]
    async fn same_day_rebuilds_replace_the_row() {
        let pool = connect_memory().await.unwrap();

        record(&pool, (100.0, 110.0), "2026-09-17").await;
        record(&pool, (100.0, 120.0), "2026-09-17").await;

        let rows = history(&pool).await;
        assert_eq!(rows, vec![("2026-09-17".to_string(), 100.0, 120.0, 0)]);
    }

    #[tokio::test]
    async fn gap_months_backfill_with_last_stored_values() {
        let pool = connect_memory().await.unwrap();

        record(&pool, (100.0, 110.0), "2026-08-28").await;
        record(&pool, (100.0, 130.0), "2026-10-05").await;

        let rows = history(&pool).await;
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

        record(&pool, (100.0, 110.0), "2026-08-28").await;
        record(&pool, (100.0, 120.0), "2026-09-05").await;

        let rows = history(&pool).await;
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().all(|(_, _, _, synthetic)| *synthetic == 0));
    }

    #[tokio::test]
    async fn load_maps_columns_to_mpf_points() {
        let pool = connect_memory().await.unwrap();

        record(&pool, (500.0, 600.0), "2026-09-17").await;
        record(&pool, (500.0, 610.0), "2026-09-18").await;

        let points = load(&pool, Market::Hk).await.expect("load");
        assert_eq!(points.len(), 2);
        assert_eq!(points[0].recorded_on, day("2026-09-17"));
        assert_eq!(points[0].contributions, 500.0);
        assert_eq!(points[0].balance, 600.0);
        assert_eq!(points[1].balance, 610.0);
    }
}
