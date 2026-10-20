use axum::Json;
use axum::extract::{Path, State};
use sqlx::Row;

use super::AppState;
use super::patch::merge_num;
use crate::calc::YearReviewRow;
use crate::error::{ApiError, ErrorBody};
use crate::models::YearReviewPatch;
use crate::service::year_review::{YearReviewResponse, build};
use crate::store::year_review as store;

/// Every figure here derives from stored data on each read except the manual
/// inputs and seeded overrides in `year_review`.
#[utoipa::path(
    get,
    path = "/api/year-review",
    tag = "year-review",
    responses(
        (status = 200, description = "Per-year ledger/investment/asset review rows", body = YearReviewResponse),
    )
)]
pub async fn list(State(state): State<AppState>) -> Result<Json<YearReviewResponse>, ApiError> {
    Ok(Json(build(&state.pool).await?))
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
#[utoipa::path(
    patch,
    path = "/api/year-review/{year}",
    tag = "year-review",
    params(("year" = i32, Path, description = "Review year")),
    request_body = YearReviewPatch,
    responses(
        (status = 200, description = "The year's updated review row", body = YearReviewRow),
        (status = 400, description = "Invalid year or figure", body = ErrorBody),
    )
)]
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
        let existing = store::record_row(&mut tx, year).await?;
        let field = |patch: Option<Option<f64>>, column: &str| -> Result<Option<f64>, ApiError> {
            let existing = existing
                .as_ref()
                .map(|row| row.try_get::<Option<f64>, _>(column))
                .transpose()
                .map_err(crate::store::StoreError::from)?
                .flatten();
            Ok(merge_num(patch, existing))
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
            store::delete_record(&mut tx, year).await?;
        } else {
            store::upsert_record(
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
