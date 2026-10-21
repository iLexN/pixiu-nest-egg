use serde::Serialize;
use sqlx::SqlitePool;

use crate::calc::{YearReviewInputs, YearReviewRow, month_year_summaries, year_review_rows};
use crate::models::Market;
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
    let (stat_rows, live_tail) = super::months::load_stat_rows_live(pool).await?;
    let items = crate::store::months::load_all_items(pool).await?;
    let rates = crate::store::months::pool_rates(pool).await?;
    let sold_pl = crate::store::yearly::hk_sold_pl(pool).await?;
    let summaries = month_year_summaries(&stat_rows, &items, &rates, &sold_pl, live_tail);
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
