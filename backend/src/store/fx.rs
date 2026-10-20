//! The stored exchange rates to HKD (`fx_rates` table). Only USD is
//! supported today — the schema CHECKs enforce that.

use sqlx::Row;
use sqlx::SqlitePool;
use sqlx::sqlite::SqliteRow;

use crate::models::{Currency, FxRate};
use crate::store::StoreError;

/// The stored USD→HKD rate; rate-dependent figures are absent while unset.
pub async fn usd_rate<'e, E>(exec: E) -> Result<Option<f64>, StoreError>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    Ok(
        sqlx::query_scalar("SELECT rate_to_hkd FROM fx_rates WHERE currency = 'USD'")
            .fetch_optional(exec)
            .await?,
    )
}

fn row_to_rate(row: &SqliteRow) -> Result<FxRate, StoreError> {
    let currency: String = row.try_get("currency")?;
    Ok(FxRate {
        currency: Currency::parse(&currency).ok_or_else(|| {
            StoreError::Corrupt(format!("stored currency {currency} is not valid"))
        })?,
        rate_to_hkd: row.try_get("rate_to_hkd")?,
        updated_at: row.try_get("updated_at")?,
    })
}

/// Every stored rate.
pub async fn list(pool: &SqlitePool) -> Result<Vec<FxRate>, StoreError> {
    let rows =
        sqlx::query("SELECT currency, rate_to_hkd, updated_at FROM fx_rates ORDER BY currency")
            .fetch_all(pool)
            .await?;
    rows.iter().map(row_to_rate).collect()
}

/// Store the rate for `currency`.
pub async fn set<'e, E>(exec: E, currency: Currency, rate: f64, now: &str) -> Result<(), StoreError>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    sqlx::query(
        "INSERT INTO fx_rates (currency, rate_to_hkd, updated_at) VALUES (?, ?, ?) \
         ON CONFLICT (currency) DO UPDATE SET rate_to_hkd = excluded.rate_to_hkd, \
         updated_at = excluded.updated_at",
    )
    .bind(currency.as_str())
    .bind(rate)
    .bind(now)
    .execute(exec)
    .await?;
    Ok(())
}

/// Clear the stored rate for `currency`.
pub async fn clear<'e, E>(exec: E, currency: Currency) -> Result<(), StoreError>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    sqlx::query("DELETE FROM fx_rates WHERE currency = ?")
        .bind(currency.as_str())
        .execute(exec)
        .await?;
    Ok(())
}
