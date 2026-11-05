use serde::Serialize;
use sqlx::SqlitePool;

use crate::calc::{
    FieldError, YearReviewInputs, YearReviewRecord, YearReviewRow, month_year_summaries,
    year_review_rows,
};
use crate::models::{Market, YearReviewPatch};
use crate::service::ServiceError;
use crate::store::{StoreError, today};

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct YearReviewResponse {
    /// The date the response was generated.
    pub today: String,
    pub years: Vec<YearReviewRow>,
}

/// Every figure here derives from stored data on each read except the manual
/// inputs and seeded overrides in `year_review`.
pub async fn build(pool: &SqlitePool) -> Result<YearReviewResponse, StoreError> {
    let now = today();
    let book = super::months::MonthBook::open(pool).await?;
    let rates = crate::store::months::pool_rates(pool).await?;
    let sold_pl = crate::store::yearly::hk_sold_pl(pool).await?;
    let summaries = month_year_summaries(&book.stat_rows, &book.items, &rates, &sold_pl, book.tail);
    let hk_years = super::yearly::build(pool, Market::Hk).await?.years;
    let us_years = super::yearly::build(pool, Market::Us).await?.years;
    let usd_hkd_rate = crate::store::fx::usd_rate(pool).await?;
    let records = crate::store::year_review::load_records(pool).await?;
    let bonds = crate::store::bonds::year_facts(pool).await?;
    let deposits = crate::store::deposits::year_facts(pool).await?;
    let transfers = crate::store::ibkr::transfers_by_year(pool).await?;
    let last_salaries = crate::store::months::last_salaries_by_year(pool).await?;

    Ok(YearReviewResponse {
        today: now.to_string(),
        years: year_review_rows(
            &YearReviewInputs {
                summaries: &summaries,
                hk_years: &hk_years,
                us_years: &us_years,
                usd_hkd_rate,
                records: &records,
                bonds: &bonds,
                deposits: &deposits,
                transfers: &transfers,
                last_salaries: &last_salaries,
            },
            now,
        ),
    })
}

fn validate_figure(field: &str, value: Option<f64>) -> Result<Option<f64>, ServiceError> {
    match value {
        Some(value) if !value.is_finite() || value < 0.0 => {
            Err(ServiceError::Validation(vec![FieldError::new(
                field,
                format!("{field} must be a non-negative number"),
            )]))
        }
        other => Ok(other),
    }
}

/// sold P/L and the invested adjustment may be negative — only finiteness is
/// enforced.
fn validate_signed_figure(field: &str, value: Option<f64>) -> Result<Option<f64>, ServiceError> {
    match value {
        Some(value) if !value.is_finite() => Err(ServiceError::Validation(vec![FieldError::new(
            field,
            format!("{field} must be a number"),
        )])),
        other => Ok(other),
    }
}

/// One `PATCH /api/year-review/{year}` in a single transaction: `sold_pl`
/// writes the HK `year_snapshots` row (an emptied snapshot is removed); the
/// rest merge against the stored record — absent fields keep their value,
/// `null` clears back to live derivation, and clearing every stored value
/// removes the row.
pub async fn update(
    pool: &SqlitePool,
    year: i32,
    patch: &YearReviewPatch,
) -> Result<(), ServiceError> {
    let mut tx = pool.begin().await.map_err(StoreError::from)?;

    if patch.sold_pl.is_some() {
        let sold_pl = validate_signed_figure("sold_pl", patch.sold_pl.flatten())?;
        crate::store::yearly::upsert_hk_sold_pl(&mut tx, year, sold_pl).await?;
        // A snapshot left with nothing stored goes away, like the yearly
        // PATCH's all-cleared case.
        crate::store::yearly::delete_empty_hk_snapshot(&mut tx, year).await?;
    }

    let touches_record = patch.income.is_some()
        || patch.invested_adjustment.is_some()
        || patch.raise.is_some()
        || patch.bond_principal.is_some()
        || patch.bond_interest.is_some()
        || patch.deposit_principal.is_some()
        || patch.deposit_interest.is_some();
    if touches_record {
        let existing = crate::store::year_review::record_row(&mut tx, year).await?;
        let merged = |patch: Option<Option<f64>>, field: fn(&YearReviewRecord) -> Option<f64>| {
            patch.unwrap_or_else(|| existing.as_ref().and_then(field))
        };
        let income = validate_figure("income", merged(patch.income, |r| r.income))?;
        let invested_adjustment = validate_signed_figure(
            "invested_adjustment",
            merged(patch.invested_adjustment, |r| r.invested_adjustment),
        )?;
        let raise = validate_signed_figure("raise", merged(patch.raise, |r| r.raise))?;
        let bond_principal = validate_figure(
            "bond_principal",
            merged(patch.bond_principal, |r| r.bond_principal),
        )?;
        let bond_interest = validate_figure(
            "bond_interest",
            merged(patch.bond_interest, |r| r.bond_interest),
        )?;
        let deposit_principal = validate_figure(
            "deposit_principal",
            merged(patch.deposit_principal, |r| r.deposit_principal),
        )?;
        let deposit_interest = validate_figure(
            "deposit_interest",
            merged(patch.deposit_interest, |r| r.deposit_interest),
        )?;

        if [
            income,
            invested_adjustment,
            raise,
            bond_principal,
            bond_interest,
            deposit_principal,
            deposit_interest,
        ]
        .iter()
        .all(Option::is_none)
        {
            crate::store::year_review::delete_record(&mut tx, year).await?;
        } else {
            crate::store::year_review::upsert_record(
                &mut tx,
                year,
                income,
                invested_adjustment,
                raise,
                bond_principal,
                bond_interest,
                deposit_principal,
                deposit_interest,
            )
            .await?;
        }
    }

    tx.commit().await.map_err(StoreError::from)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn income_patch(income: Option<f64>) -> YearReviewPatch {
        YearReviewPatch {
            income: Some(income),
            ..Default::default()
        }
    }

    async fn load_record(pool: &SqlitePool, year: i32) -> Option<crate::calc::YearReviewRecord> {
        crate::store::year_review::load_records(pool)
            .await
            .expect("records")
            .remove(&year)
    }

    #[tokio::test]
    async fn update_merges_and_deletes_when_every_value_clears() {
        let pool = crate::db::connect_memory().await.expect("memory db");

        update(&pool, 2025, &income_patch(Some(100.0)))
            .await
            .expect("write");
        update(
            &pool,
            2025,
            &YearReviewPatch {
                raise: Some(Some(0.1)),
                ..Default::default()
            },
        )
        .await
        .expect("merge");
        let record = load_record(&pool, 2025).await.expect("record");
        assert_eq!(record.income, Some(100.0));
        assert_eq!(record.raise, Some(0.1));

        update(
            &pool,
            2025,
            &YearReviewPatch {
                income: Some(None),
                raise: Some(None),
                ..Default::default()
            },
        )
        .await
        .expect("clear");
        assert!(load_record(&pool, 2025).await.is_none());
    }

    #[tokio::test]
    async fn update_writes_and_cleans_the_hk_sold_pl_snapshot() {
        let pool = crate::db::connect_memory().await.expect("memory db");
        async fn snapshot_count(pool: &SqlitePool) -> i64 {
            sqlx::query_scalar("SELECT COUNT(*) FROM year_snapshots WHERE year = 2025")
                .fetch_one(pool)
                .await
                .expect("count")
        }

        update(
            &pool,
            2025,
            &YearReviewPatch {
                sold_pl: Some(Some(50.0)),
                ..Default::default()
            },
        )
        .await
        .expect("write");
        assert_eq!(snapshot_count(&pool).await, 1);

        update(
            &pool,
            2025,
            &YearReviewPatch {
                sold_pl: Some(None),
                ..Default::default()
            },
        )
        .await
        .expect("clear");
        assert_eq!(snapshot_count(&pool).await, 0);
    }
}
