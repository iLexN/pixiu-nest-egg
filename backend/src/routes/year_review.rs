use std::collections::BTreeMap;

use axum::Json;
use axum::extract::{Path, State};
use serde::Serialize;
use sqlx::{Row, SqlitePool};

use super::{AppState, now_timestamp, today};
use crate::calc::{
    BondYearFacts, DepositFacts, YearReviewInputs, YearReviewRecord, YearReviewRow,
    month_year_summaries, year_review_rows,
};
use crate::error::ApiError;
use crate::models::{Market, YearReviewPatch};

#[derive(Debug, Serialize)]
pub struct YearReviewResponse {
    /// The date the response was generated.
    pub today: String,
    pub years: Vec<YearReviewRow>,
}

/// Every figure here derives from stored data on each read except the manual
/// inputs and seeded overrides in `year_review`.
pub async fn list(State(state): State<AppState>) -> Result<Json<YearReviewResponse>, ApiError> {
    Ok(Json(build(&state.pool).await?))
}

pub async fn build(pool: &SqlitePool) -> Result<YearReviewResponse, ApiError> {
    let now = today();
    let (stat_rows, live_tail) = super::months::load_stat_rows_live(pool).await?;
    let items = super::months::load_all_items(pool).await?;
    let rates = super::months::pool_rates(pool).await?;
    let sold_pl = super::yearly::hk_sold_pl(pool).await?;
    let summaries = month_year_summaries(&stat_rows, &items, &rates, &sold_pl, live_tail);
    let hk_years = super::yearly::build(pool, Market::Hk).await?.years;
    let records = load_records(pool).await?;
    let bonds = bond_facts(pool).await?;
    let deposits = deposit_facts(pool).await?;
    let transfers = ibkr_transfers_by_year(pool).await?;
    let last_salaries = last_salaries_by_year(pool).await?;

    Ok(YearReviewResponse {
        today: now.to_string(),
        years: year_review_rows(
            &YearReviewInputs {
                summaries: &summaries,
                hk_years: &hk_years,
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

/// Each year's last stored month salary — the input the derived 月薪增幅
/// compares against the prior year's.
async fn last_salaries_by_year(pool: &SqlitePool) -> Result<BTreeMap<i32, f64>, ApiError> {
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
            .ok_or_else(|| ApiError::Conflict(format!("stored month {month} is not valid")))?;
        map.insert(year, row.try_get::<f64, _>("salary")?);
    }
    Ok(map)
}

/// Σ `ibkr_transfers` per calendar year — the year's IBKR 轉入.
async fn ibkr_transfers_by_year(pool: &SqlitePool) -> Result<BTreeMap<i32, f64>, ApiError> {
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

/// The stored `year_review` records keyed by year.
async fn load_records(pool: &SqlitePool) -> Result<BTreeMap<i32, YearReviewRecord>, ApiError> {
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

/// Bonds reduced to what the year review needs: principal, maturity, the
/// coupon schedule's first pay year (the purchase proxy), and each received
/// coupon's (pay year, amount).
async fn bond_facts(pool: &SqlitePool) -> Result<Vec<BondYearFacts>, ApiError> {
    let rows = sqlx::query(
        "SELECT b.id, b.principal, b.maturity_date, c.pay_date, c.received_amount \
         FROM bonds b LEFT JOIN bond_coupons c ON c.bond_id = b.id \
         ORDER BY b.sort_order, b.id, c.pay_date",
    )
    .fetch_all(pool)
    .await?;

    let mut facts: Vec<BondYearFacts> = Vec::new();
    let mut last_id: Option<i64> = None;
    for row in &rows {
        let id: i64 = row.try_get("id")?;
        if last_id != Some(id) {
            let maturity_date: String = row.try_get("maturity_date")?;
            facts.push(BondYearFacts {
                principal: row.try_get("principal")?,
                first_coupon_year: None,
                maturity_date: chrono::NaiveDate::parse_from_str(&maturity_date, "%Y-%m-%d")
                    .map_err(|_| {
                        ApiError::Conflict(format!(
                            "stored maturity_date {maturity_date} is not valid"
                        ))
                    })?,
                received: Vec::new(),
            });
            last_id = Some(id);
        }
        let pay_date: Option<String> = row.try_get("pay_date")?;
        let Some(pay_date) = pay_date else { continue };
        let parsed = chrono::NaiveDate::parse_from_str(&pay_date, "%Y-%m-%d")
            .map_err(|_| ApiError::Conflict(format!("stored pay_date {pay_date} is not valid")))?;
        let fact = facts.last_mut().expect("just pushed");
        let year = chrono::Datelike::year(&parsed);
        fact.first_coupon_year = Some(fact.first_coupon_year.map_or(year, |first| first.min(year)));
        if let Some(amount) = row.try_get::<Option<f64>, _>("received_amount")? {
            fact.received.push((year, amount));
        }
    }
    Ok(facts)
}

async fn deposit_facts(pool: &SqlitePool) -> Result<Vec<DepositFacts<'static>>, ApiError> {
    let rows = sqlx::query("SELECT principal, interest, end_date FROM deposits")
        .fetch_all(pool)
        .await?;
    let mut facts = Vec::with_capacity(rows.len());
    for row in &rows {
        let end_date: String = row.try_get("end_date")?;
        facts.push(DepositFacts {
            bank: None,
            principal: row.try_get("principal")?,
            interest: row.try_get("interest")?,
            end_date: chrono::NaiveDate::parse_from_str(&end_date, "%Y-%m-%d").map_err(|_| {
                ApiError::Conflict(format!("stored end_date {end_date} is not valid"))
            })?,
        });
    }
    Ok(facts)
}

fn validate_figure(field: &str, value: Option<f64>) -> Result<Option<f64>, ApiError> {
    match value {
        Some(value) if !value.is_finite() || value < 0.0 => Err(ApiError::field(
            field,
            format!("{field} must be a non-negative number"),
        )),
        other => Ok(other),
    }
}

/// sold P/L and the invested adjustment may be negative — only finiteness is
/// enforced.
fn validate_signed_figure(field: &str, value: Option<f64>) -> Result<Option<f64>, ApiError> {
    match value {
        Some(value) if !value.is_finite() => {
            Err(ApiError::field(field, format!("{field} must be a number")))
        }
        other => Ok(other),
    }
}

/// Set or clear the year's stored manual figures. `sold_pl` writes the HK
/// `year_snapshots` row; the rest upsert `year_review`. A field left absent
/// stays as-is; `null` clears it back to live derivation; clearing every
/// stored value removes the row.
pub async fn update(
    State(state): State<AppState>,
    Path(year): Path<i32>,
    Json(patch): Json<YearReviewPatch>,
) -> Result<Json<YearReviewRow>, ApiError> {
    if !(2000..=2100).contains(&year) {
        return Err(ApiError::field("year", "year must be a plausible year"));
    }
    if patch.income.is_none()
        && patch.invested_adjustment.is_none()
        && patch.raise.is_none()
        && patch.sold_pl.is_none()
        && patch.bond_principal.is_none()
        && patch.bond_interest.is_none()
        && patch.deposit_principal.is_none()
        && patch.deposit_interest.is_none()
    {
        return Err(ApiError::field("patch", "at least one field is required"));
    }

    let mut tx = state.pool.begin().await?;

    if patch.sold_pl.is_some() {
        let sold_pl = validate_signed_figure("sold_pl", patch.sold_pl.flatten())?;
        sqlx::query(
            "INSERT INTO year_snapshots (market, year, sold_pl, updated_at) \
             VALUES ('HK', ?, ?, ?) \
             ON CONFLICT (market, year) DO UPDATE SET \
             sold_pl = excluded.sold_pl, updated_at = excluded.updated_at",
        )
        .bind(year)
        .bind(sold_pl)
        .bind(now_timestamp())
        .execute(&mut *tx)
        .await?;
        // A snapshot left with nothing stored goes away, like the yearly
        // PATCH's all-cleared case.
        sqlx::query(
            "DELETE FROM year_snapshots WHERE market = 'HK' AND year = ? \
             AND invested IS NULL AND cost IS NULL AND market_value IS NULL AND sold_pl IS NULL",
        )
        .bind(year)
        .execute(&mut *tx)
        .await?;
    }

    let touches_record = patch.income.is_some()
        || patch.invested_adjustment.is_some()
        || patch.raise.is_some()
        || patch.bond_principal.is_some()
        || patch.bond_interest.is_some()
        || patch.deposit_principal.is_some()
        || patch.deposit_interest.is_some();
    if touches_record {
        let existing = sqlx::query(
            "SELECT income, invested_adjustment, salary_raise, bond_principal, bond_interest, \
             deposit_principal, deposit_interest FROM year_review WHERE year = ?",
        )
        .bind(year)
        .fetch_optional(&mut *tx)
        .await?;
        let field = |patch: Option<Option<f64>>, column: &str| -> Result<Option<f64>, ApiError> {
            let existing = existing
                .as_ref()
                .map(|row| row.try_get::<Option<f64>, _>(column))
                .transpose()?
                .flatten();
            Ok(patch.unwrap_or(existing))
        };
        let income = validate_figure("income", field(patch.income, "income")?)?;
        let invested_adjustment = validate_signed_figure(
            "invested_adjustment",
            field(patch.invested_adjustment, "invested_adjustment")?,
        )?;
        let raise = validate_signed_figure("raise", field(patch.raise, "salary_raise")?)?;
        let bond_principal = validate_figure(
            "bond_principal",
            field(patch.bond_principal, "bond_principal")?,
        )?;
        let bond_interest = validate_figure(
            "bond_interest",
            field(patch.bond_interest, "bond_interest")?,
        )?;
        let deposit_principal = validate_figure(
            "deposit_principal",
            field(patch.deposit_principal, "deposit_principal")?,
        )?;
        let deposit_interest = validate_figure(
            "deposit_interest",
            field(patch.deposit_interest, "deposit_interest")?,
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
            sqlx::query("DELETE FROM year_review WHERE year = ?")
                .bind(year)
                .execute(&mut *tx)
                .await?;
        } else {
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
            .execute(&mut *tx)
            .await?;
        }
    }

    tx.commit().await?;

    let response = build(&state.pool).await?;
    let row = response
        .years
        .into_iter()
        .find(|row| row.year == year)
        .unwrap_or(YearReviewRow {
            year,
            ledger: Default::default(),
            investment: Default::default(),
            assets: Default::default(),
            record: None,
        });
    Ok(Json(row))
}
