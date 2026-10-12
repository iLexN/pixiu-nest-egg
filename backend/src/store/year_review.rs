use std::collections::BTreeMap;

use sqlx::Row;
use sqlx::SqlitePool;

use crate::calc::YearReviewRecord;
use crate::store::{StoreError, now_timestamp};

/// The stored `year_review` records keyed by year.
pub async fn load_records(
    pool: &SqlitePool,
) -> Result<BTreeMap<i32, YearReviewRecord>, StoreError> {
    let rows = sqlx::query(
        "SELECT year, income, invested_adjustment, salary_raise, bond_principal, bond_interest, \
         deposit_principal, deposit_interest FROM year_review",
    )
    .fetch_all(pool)
    .await?;
    let mut records = BTreeMap::new();
    for row in &rows {
        records.insert(
            row.try_get("year")?,
            YearReviewRecord {
                income: row.try_get("income")?,
                invested_adjustment: row.try_get("invested_adjustment")?,
                raise: row.try_get("salary_raise")?,
                bond_principal: row.try_get("bond_principal")?,
                bond_interest: row.try_get("bond_interest")?,
                deposit_principal: row.try_get("deposit_principal")?,
                deposit_interest: row.try_get("deposit_interest")?,
            },
        );
    }
    Ok(records)
}

/// One year's stored record, inside the caller's transaction — the patch
/// handler merges each field against it.
pub async fn record_row(
    conn: &mut sqlx::SqliteConnection,
    year: i32,
) -> Result<Option<sqlx::sqlite::SqliteRow>, StoreError> {
    Ok(sqlx::query(
        "SELECT income, invested_adjustment, salary_raise, bond_principal, bond_interest, \
         deposit_principal, deposit_interest FROM year_review WHERE year = ?",
    )
    .bind(year)
    .fetch_optional(&mut *conn)
    .await?)
}

/// Clearing every stored value removes the row. Inside the caller's
/// transaction.
pub async fn delete_record(conn: &mut sqlx::SqliteConnection, year: i32) -> Result<(), StoreError> {
    sqlx::query("DELETE FROM year_review WHERE year = ?")
        .bind(year)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

/// Upsert the year's stored manual figures, inside the caller's transaction.
#[allow(clippy::too_many_arguments)]
pub async fn upsert_record(
    conn: &mut sqlx::SqliteConnection,
    year: i32,
    income: Option<f64>,
    invested_adjustment: Option<f64>,
    raise: Option<f64>,
    bond_principal: Option<f64>,
    bond_interest: Option<f64>,
    deposit_principal: Option<f64>,
    deposit_interest: Option<f64>,
) -> Result<(), StoreError> {
    sqlx::query(
        "INSERT INTO year_review \
         (year, income, invested_adjustment, salary_raise, bond_principal, bond_interest, \
          deposit_principal, deposit_interest, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?) \
         ON CONFLICT (year) DO UPDATE SET \
         income = excluded.income, invested_adjustment = excluded.invested_adjustment, \
         salary_raise = excluded.salary_raise, \
         bond_principal = excluded.bond_principal, \
         bond_interest = excluded.bond_interest, \
         deposit_principal = excluded.deposit_principal, \
         deposit_interest = excluded.deposit_interest, \
         updated_at = excluded.updated_at",
    )
    .bind(year)
    .bind(income)
    .bind(invested_adjustment)
    .bind(raise)
    .bind(bond_principal)
    .bind(bond_interest)
    .bind(deposit_principal)
    .bind(deposit_interest)
    .bind(now_timestamp())
    .execute(&mut *conn)
    .await?;
    Ok(())
}
