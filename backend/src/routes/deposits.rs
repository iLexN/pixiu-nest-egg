use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde::Deserialize;

use super::patch::{merge_num, merge_text};
use super::{AppState, finite_or, parse_order, parse_received_at, parse_status};
use crate::calc::{DepositInput, validate_deposit};
use crate::error::{ApiError, ErrorBody};
use crate::models::{Deposit, DepositPatch, NewDeposit};
use crate::store::deposits as store;

pub use crate::service::deposits::DepositSummaryResponse;

#[derive(Debug, Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
pub struct ListQuery {
    /// `active` (not yet 收訖, even past end date) or `ended` (received).
    pub status: Option<String>,
    /// Restrict to deposits ending in this year.
    pub year: Option<i32>,
    /// `asc` (default) or `desc`, by end date.
    pub order: Option<String>,
}

#[utoipa::path(
    operation_id = "deposits_list",
    get,
    path = "/api/deposits",
    tag = "deposits",
    params(ListQuery),
    responses(
        (status = 200, description = "List 定期 deposits", body = [Deposit]),
    )
)]
pub async fn list(
    State(state): State<AppState>,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<Deposit>>, ApiError> {
    let received = parse_status(query.status.as_deref(), "active", "ended")?;
    let descending = parse_order(query.order.as_deref());
    store::list(&state.pool, received, query.year, descending)
        .await
        .map(Json)
        .map_err(ApiError::from)
}

#[utoipa::path(
    operation_id = "deposits_create",
    post,
    path = "/api/deposits",
    tag = "deposits",
    request_body = NewDeposit,
    responses(
        (status = 201, description = "Deposit created", body = Deposit),
        (status = 400, description = "Missing or invalid fields", body = ErrorBody),
    )
)]
pub async fn create(
    State(state): State<AppState>,
    Json(body): Json<NewDeposit>,
) -> Result<(StatusCode, Json<Deposit>), ApiError> {
    let id = crate::service::deposits::create(&state.pool, &body).await?;
    Ok((
        StatusCode::CREATED,
        Json(store::load_one(&state.pool, id).await?),
    ))
}

#[utoipa::path(
    operation_id = "deposits_update",
    patch,
    path = "/api/deposits/{id}",
    tag = "deposits",
    params(("id" = i64, Path, description = "Deposit id")),
    request_body = DepositPatch,
    responses(
        (status = 200, description = "Updated deposit", body = Deposit),
        (status = 400, description = "Missing or invalid fields", body = ErrorBody),
        (status = 404, description = "Deposit not found", body = ErrorBody),
    )
)]
pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(patch): Json<DepositPatch>,
) -> Result<Json<Deposit>, ApiError> {
    let existing = store::load_one(&state.pool, id).await?;

    let label = merge_text(patch.label, existing.label);
    let bank = merge_text(patch.bank, existing.bank);
    let note1 = merge_text(patch.note1, existing.note1);
    let note2 = merge_text(patch.note2, existing.note2);
    let principal = merge_num(patch.principal, existing.principal);
    // A derived rate is never persisted: an absent patch keeps rate NULL.
    let rate = merge_num(
        patch.rate,
        if existing.rate_derived {
            None
        } else {
            existing.rate
        },
    );
    let interest = merge_num(patch.interest, existing.interest);
    let start_date = merge_text(patch.start_date, existing.start_date);
    let end_date = patch.end_date.unwrap_or(existing.end_date);

    let validated = validate_deposit(DepositInput {
        label: label.as_deref(),
        bank: bank.as_deref(),
        principal,
        rate,
        interest,
        start_date: start_date.as_deref(),
        end_date: &end_date,
    })
    .map_err(ApiError::Validation)?;

    store::update_deposit(&state.pool, id, &validated, &note1, &note2).await?;

    Ok(Json(store::load_one(&state.pool, id).await?))
}

#[utoipa::path(
    operation_id = "deposits_remove",
    delete,
    path = "/api/deposits/{id}",
    tag = "deposits",
    params(("id" = i64, Path, description = "Deposit id")),
    responses(
        (status = 204, description = "Deposit deleted"),
        (status = 404, description = "Deposit not found", body = ErrorBody),
        (status = 409, description = "Deposit already received", body = ErrorBody),
    )
)]
pub async fn remove(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    crate::service::deposits::delete(&state.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
pub struct ReceiveDeposit {
    /// 收訖日 (`YYYY-MM-DD`); defaults to today.
    pub received_at: Option<String>,
    /// Corrects the stored interest to the amount actually received.
    pub interest: Option<f64>,
    /// Bank principal + interest into a cash account. Defaults to true.
    #[serde(default)]
    pub bank_in: Option<bool>,
    /// The chosen receipt account; falls back to the HKD default.
    #[serde(default)]
    pub account_id: Option<i64>,
}

/// 收訖: mark the deposit received, optionally credit its principal +
/// (corrected) interest to a cash manual asset, and record the month's
/// `dep-end` adjustment item — the whole sheet "定期 end step" in one action.
#[utoipa::path(
    operation_id = "deposits_receive",
    post,
    path = "/api/deposits/{id}/receive",
    tag = "deposits",
    params(("id" = i64, Path, description = "Deposit id")),
    request_body = ReceiveDeposit,
    responses(
        (status = 200, description = "Deposit marked 收訖 with optional bank-in", body = Deposit),
        (status = 400, description = "Missing or invalid fields", body = ErrorBody),
        (status = 404, description = "Deposit not found", body = ErrorBody),
        (status = 409, description = "Deposit already received", body = ErrorBody),
    )
)]
pub async fn receive(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<ReceiveDeposit>,
) -> Result<Json<Deposit>, ApiError> {
    let existing = store::load_one(&state.pool, id).await?;
    let received_at = parse_received_at(body.received_at.as_deref())?;
    let interest = finite_or("interest", body.interest, existing.interest)?;
    crate::service::deposits::receive(
        &state.pool,
        id,
        &crate::service::deposits::ReceiveInput {
            received_at,
            interest,
            bank_in: body.bank_in.unwrap_or(true),
            account_id: body.account_id,
        },
    )
    .await?;

    Ok(Json(store::load_one(&state.pool, id).await?))
}

/// Every figure here is derived from the stored deposits on each read.
#[utoipa::path(
    operation_id = "deposits_summary",
    get,
    path = "/api/deposits/summary",
    tag = "deposits",
    responses(
        (status = 200, description = "Active deposits plus month/bank/year rollups", body = DepositSummaryResponse),
    )
)]
pub async fn summary(
    State(state): State<AppState>,
) -> Result<Json<DepositSummaryResponse>, ApiError> {
    Ok(Json(crate::service::deposits::summary(&state.pool).await?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connect_memory;
    use sqlx::SqlitePool;

    fn new_deposit(
        principal: Option<f64>,
        rate: Option<f64>,
        interest: Option<f64>,
        start_date: Option<&str>,
        end_date: &str,
    ) -> NewDeposit {
        NewDeposit {
            label: Some("SC-TEST".to_string()),
            bank: Some("SC".to_string()),
            principal,
            rate,
            interest,
            start_date: start_date.map(str::to_string),
            end_date: end_date.to_string(),
            note1: None,
            note2: None,
            debit_account_id: None,
        }
    }

    async fn stored_rate(pool: &SqlitePool, id: i64) -> Option<f64> {
        sqlx::query_scalar("SELECT rate FROM deposits WHERE id = ?")
            .bind(id)
            .fetch_one(pool)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn blank_rate_is_derived_on_read_and_never_stored() {
        let pool = connect_memory().await.expect("memory db");
        let state = AppState { pool: pool.clone() };

        // Stored rate wins and stays unflagged.
        let (status, created) = create(
            State(state.clone()),
            Json(new_deposit(
                Some(55000.0),
                Some(0.03),
                Some(435.0),
                Some("2026-09-30"),
                "2027-01-05",
            )),
        )
        .await
        .expect("create stored-rate deposit");
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(created.rate, Some(0.03));
        assert!(!created.rate_derived);

        // Blank rate with enough inputs derives on read; the column stays NULL.
        let (_, created) = create(
            State(state.clone()),
            Json(new_deposit(
                Some(140000.0),
                None,
                Some(1279.0),
                Some("2026-10-13"),
                "2027-02-02",
            )),
        )
        .await
        .expect("create blank-rate deposit");
        let id = created.id;
        let expected = 1279.0 / 140000.0 * 365.0 / 112.0;
        assert!((created.rate.expect("derived rate") - expected).abs() < 1e-9);
        assert!(created.rate_derived);
        assert_eq!(stored_rate(&pool, id).await, None);

        // A PATCH that leaves rate absent does not persist the derived rate.
        let updated = update(
            State(state),
            Path(id),
            Json(DepositPatch {
                interest: Some(Some(1300.0)),
                ..DepositPatch::default()
            }),
        )
        .await
        .expect("patch interest")
        .0;
        let expected = 1300.0 / 140000.0 * 365.0 / 112.0;
        assert!((updated.rate.expect("re-derived rate") - expected).abs() < 1e-9);
        assert!(updated.rate_derived);
        assert_eq!(stored_rate(&pool, id).await, None);
    }

    /// The corrected 實收利息 — not a user-entered amount — sets both the
    /// cash credit and the dep-end item: principal + 1000 = 111000.
    #[tokio::test]
    async fn corrected_interest_flows_into_the_cash_credit() {
        let pool = connect_memory().await.expect("memory db");
        let state = AppState { pool: pool.clone() };

        let asset_id: i64 = sqlx::query_scalar(
            "INSERT INTO cash_accounts (label, currency, role, amount, sort_order, updated_at) \
             VALUES ('渣打', 'HKD', 'bank', 1000, 1, '2026-01-01T00:00:00') RETURNING id",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO month_stats (month, created_at, updated_at) \
             VALUES ('2099-10-01', '2026-01-01T00:00:00', '2026-01-01T00:00:00')",
        )
        .execute(&pool)
        .await
        .unwrap();

        let (_, created) = create(
            State(state.clone()),
            Json(new_deposit(
                Some(110000.0),
                None,
                Some(993.0),
                None,
                "2099-10-12",
            )),
        )
        .await
        .expect("create deposit");
        let id = created.id;

        let received = receive(
            State(state),
            Path(id),
            Json(ReceiveDeposit {
                received_at: None,
                interest: Some(1000.0),
                bank_in: None,
                account_id: Some(asset_id),
            }),
        )
        .await
        .expect("receive")
        .0;
        assert_eq!(received.interest, Some(1000.0));

        let amount: f64 = sqlx::query_scalar("SELECT amount FROM cash_accounts WHERE id = ?")
            .bind(asset_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(amount, 1000.0 + 111000.0);
        let credited: f64 = sqlx::query_scalar("SELECT credited_amount FROM deposits WHERE id = ?")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(credited, 111000.0);
        let item: f64 = sqlx::query_scalar(
            "SELECT amount FROM month_items WHERE month = '2099-10-01' AND auto_key = ?",
        )
        .bind(format!("dep-end:{id}"))
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(item, 111000.0);
    }

    #[tokio::test]
    async fn blank_rate_without_start_date_stays_null() {
        let pool = connect_memory().await.expect("memory db");
        let state = AppState { pool };

        let (_, created) = create(
            State(state),
            Json(new_deposit(
                Some(140000.0),
                None,
                Some(1279.0),
                None,
                "2027-02-02",
            )),
        )
        .await
        .expect("create no-start deposit");
        assert_eq!(created.rate, None);
        assert!(!created.rate_derived);
    }
}
