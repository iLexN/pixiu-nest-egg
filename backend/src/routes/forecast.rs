use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use sqlx::SqlitePool;

use super::{AppState, parse_ym};
use crate::calc::{self, FieldError};
use crate::error::{ApiError, ErrorBody};
use crate::models::{
    Deposit, ForecastItem, ForecastItemKind, ForecastItemPatch, ForecastResponse, NewDeposit,
    NewForecastItem,
};
use crate::store::forecast as store;

async fn load_item(pool: &SqlitePool, id: i64) -> Result<ForecastItem, ApiError> {
    store::load_item(pool, id).await.map_err(ApiError::from)
}

/// `YYYY-MM`/`YYYY-MM-01` → `YYYY-MM-01`, deposit kinds only.
fn validate_return_month(
    kind: ForecastItemKind,
    value: Option<String>,
) -> Result<Option<String>, Vec<FieldError>> {
    let Some(value) = value else {
        return Ok(None);
    };
    if kind.deposit_lag().is_none() {
        return Err(vec![FieldError::new(
            "return_month",
            "return_month only applies to 定期 plan items",
        )]);
    }
    parse_ym(&value)
        .map(|date| date.to_string())
        .map(Some)
        .map_err(|_| {
            vec![FieldError::new(
                "return_month",
                "return_month must be in YYYY-MM form",
            )]
        })
}

fn validate_amount(amount: f64) -> Result<(), Vec<FieldError>> {
    if !amount.is_finite() || amount == 0.0 {
        return Err(vec![FieldError::new(
            "amount",
            "amount must be a non-zero number",
        )]);
    }
    Ok(())
}

/// The seven-month 預測 grid (`Overview!A20:H36`), fully derived on read.
#[utoipa::path(
    operation_id = "forecast_forecast",
    get,
    path = "/api/forecast",
    tag = "forecast",
    responses(
        (status = 200, description = "Seven-month rolling cash forecast", body = ForecastResponse),
    )
)]
pub async fn forecast(State(state): State<AppState>) -> Result<Json<ForecastResponse>, ApiError> {
    Ok(Json(crate::service::forecast::build(&state.pool).await?))
}

#[utoipa::path(
    operation_id = "forecast_create_item",
    post,
    path = "/api/forecast/{ym}/items",
    tag = "forecast",
    params(("ym" = String, Path, description = "Month key YYYY-MM")),
    request_body = NewForecastItem,
    responses(
        (status = 201, description = "Forecast item created", body = ForecastItem),
        (status = 400, description = "Invalid month key, kind or fields", body = ErrorBody),
    )
)]
pub async fn create_item(
    State(state): State<AppState>,
    Path(ym): Path<String>,
    Json(body): Json<NewForecastItem>,
) -> Result<(StatusCode, Json<ForecastItem>), ApiError> {
    let month = parse_ym(&ym)?.to_string();

    let mut errors = Vec::new();
    if let Err(mut fields) = validate_amount(body.amount) {
        errors.append(&mut fields);
    }
    let return_month = match validate_return_month(body.kind, body.return_month) {
        Ok(value) => value,
        Err(mut fields) => {
            errors.append(&mut fields);
            None
        }
    };
    if !errors.is_empty() {
        return Err(ApiError::Validation(errors));
    }

    let note = body
        .note
        .map(|note| note.trim().to_string())
        .filter(|note| !note.is_empty());
    let sort_order = store::next_sort_order(&state.pool, &month).await?;
    let id = store::insert_item(
        &state.pool,
        &month,
        body.kind.as_str(),
        body.amount,
        &return_month,
        &note,
        sort_order,
    )
    .await?;
    Ok((StatusCode::CREATED, Json(load_item(&state.pool, id).await?)))
}

#[utoipa::path(
    operation_id = "forecast_update_item",
    patch,
    path = "/api/forecast-items/{id}",
    tag = "forecast",
    params(("id" = i64, Path, description = "Forecast item id")),
    request_body = ForecastItemPatch,
    responses(
        (status = 200, description = "Updated forecast item", body = ForecastItem),
        (status = 400, description = "Missing or invalid fields", body = ErrorBody),
        (status = 404, description = "Forecast item not found", body = ErrorBody),
    )
)]
pub async fn update_item(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(patch): Json<ForecastItemPatch>,
) -> Result<Json<ForecastItem>, ApiError> {
    let existing = load_item(&state.pool, id).await?;

    let kind = patch.kind.unwrap_or(existing.kind);
    let amount = patch.amount.unwrap_or(existing.amount);
    let note = match patch.note {
        Some(value) => value
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        None => existing.note,
    };

    let mut errors = Vec::new();
    if let Err(mut fields) = validate_amount(amount) {
        errors.append(&mut fields);
    }
    // An explicit return_month validates against the resulting kind; switching
    // to a non-deposit kind clears a stored override (the exclude_from_living
    // rule's shape).
    let return_month = match patch.return_month {
        Some(value) => match validate_return_month(kind, value) {
            Ok(parsed) => parsed,
            Err(mut fields) => {
                errors.append(&mut fields);
                existing.return_month.clone()
            }
        },
        None if kind.deposit_lag().is_none() => None,
        None => existing.return_month.clone(),
    };
    if !errors.is_empty() {
        return Err(ApiError::Validation(errors));
    }

    store::update_item(&state.pool, id, kind.as_str(), amount, &return_month, &note).await?;
    Ok(Json(load_item(&state.pool, id).await?))
}

#[utoipa::path(
    operation_id = "forecast_remove_item",
    delete,
    path = "/api/forecast-items/{id}",
    tag = "forecast",
    params(("id" = i64, Path, description = "Forecast item id")),
    responses(
        (status = 204, description = "Forecast item deleted"),
        (status = 404, description = "Forecast item not found", body = ErrorBody),
    )
)]
pub async fn remove_item(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    if store::delete_item(&state.pool, id).await? == 0 {
        return Err(ApiError::NotFound(format!("forecast item {id} not found")));
    }
    Ok(StatusCode::NO_CONTENT)
}

/// 轉為定期: convert a deposit plan into a real `deposits` row — the same
/// placement as `POST /api/deposits` (optional 活期 debit, `dep-start:<id>`
/// item) — and delete the plan, in one transaction, so the plan's derived
/// return/lock effects can never double-count against the deposit.
#[utoipa::path(
    operation_id = "forecast_convert",
    post,
    path = "/api/forecast-items/{id}/convert",
    tag = "forecast",
    params(("id" = i64, Path, description = "Forecast item id")),
    request_body = NewDeposit,
    responses(
        (status = 201, description = "Deposit created and plan consumed", body = Deposit),
        (status = 400, description = "Missing or invalid fields", body = ErrorBody),
        (status = 404, description = "Forecast item not found", body = ErrorBody),
        (status = 409, description = "Item is not a deposit plan", body = ErrorBody),
    )
)]
pub async fn convert(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(mut body): Json<NewDeposit>,
) -> Result<(StatusCode, Json<Deposit>), ApiError> {
    let item = load_item(&state.pool, id).await?;
    if item.kind.deposit_lag().is_none() {
        return Err(ApiError::Conflict(format!(
            "forecast item {id} is not a deposit plan"
        )));
    }
    if body.bank.is_none() {
        body.bank = item.kind.default_bank().map(str::to_string);
    }
    if body.principal.is_none() {
        body.principal = Some(-item.amount);
    }
    let validated = calc::validate_deposit(calc::DepositInput {
        label: body.label.as_deref(),
        bank: body.bank.as_deref(),
        principal: body.principal,
        rate: body.rate,
        interest: body.interest,
        start_date: body.start_date.as_deref(),
        end_date: &body.end_date,
    })
    .map_err(ApiError::Validation)?;

    let deposit_id =
        crate::service::deposits::convert_plan(&state.pool, id, &validated, &body).await?;
    Ok((
        StatusCode::CREATED,
        Json(crate::store::deposits::load_one(&state.pool, deposit_id).await?),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connect_memory;

    async fn seed_bank(pool: &SqlitePool, label: &str, amount: f64) -> i64 {
        sqlx::query_scalar(
            "INSERT INTO cash_accounts (label, currency, role, amount, is_default_receipt, \
             sort_order, updated_at) VALUES (?, 'HKD', 'bank', ?, 1, 1, \
             '2026-01-01T00:00:00') RETURNING id",
        )
        .bind(label)
        .bind(amount)
        .fetch_one(pool)
        .await
        .unwrap()
    }

    async fn seed_month(pool: &SqlitePool, month: &str) {
        sqlx::query("INSERT INTO month_stats (month, created_at, updated_at) VALUES (?, ?, ?)")
            .bind(month)
            .bind("2026-01-01T00:00:00")
            .bind("2026-01-01T00:00:00")
            .execute(pool)
            .await
            .unwrap();
    }

    /// The minimal convert body: bank/principal default from the plan.
    fn deposit_body() -> NewDeposit {
        NewDeposit {
            label: Some("HS-1".to_string()),
            bank: None,
            principal: None,
            rate: None,
            interest: None,
            start_date: Some("2099-10-13".to_string()),
            end_date: "2100-02-10".to_string(),
            note1: None,
            note2: None,
            debit_account_id: None,
        }
    }

    /// Converting a deposit plan runs the shared placement: bank/principal
    /// default from the plan, the start month's `dep-start:<id>` item of
    /// −principal lands, and the plan is gone.
    #[tokio::test]
    async fn convert_records_the_dep_start_item_and_deletes_the_plan() {
        let pool = connect_memory().await.expect("memory db");
        seed_month(&pool, "2099-10-01").await;
        let item = store::insert_item(
            &pool,
            "2099-10-01",
            "hs_deposit",
            -130000.0,
            &None,
            &None,
            1,
        )
        .await
        .unwrap();

        let (status, deposit) = convert(
            State(AppState { pool: pool.clone() }),
            Path(item),
            Json(deposit_body()),
        )
        .await
        .expect("convert");

        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(deposit.bank.as_deref(), Some("HS"));
        assert_eq!(deposit.principal, Some(130000.0));
        let item_amount: f64 = sqlx::query_scalar(
            "SELECT amount FROM month_items WHERE month = '2099-10-01' AND auto_key = ?",
        )
        .bind(format!("dep-start:{}", deposit.id))
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(item_amount, -130000.0);
        assert!(matches!(
            store::load_item(&pool, item).await,
            Err(crate::store::StoreError::NotFound(_))
        ));
    }

    /// `debit_account_id` on the convert body debits the principal, and the
    /// deposit records where it went — same as `POST /api/deposits`.
    #[tokio::test]
    async fn convert_debits_the_chosen_account() {
        let pool = connect_memory().await.expect("memory db");
        let hs = seed_bank(&pool, "HS", 200000.0).await;
        let item = store::insert_item(
            &pool,
            "2099-10-01",
            "hs_deposit",
            -130000.0,
            &None,
            &None,
            1,
        )
        .await
        .unwrap();

        let mut body = deposit_body();
        body.debit_account_id = Some(hs);
        let (_, deposit) = convert(
            State(AppState { pool: pool.clone() }),
            Path(item),
            Json(body),
        )
        .await
        .expect("convert");

        assert_eq!(deposit.debited_account_id, Some(hs));
        assert_eq!(deposit.debited_amount, Some(130000.0));
        let amount: f64 = sqlx::query_scalar("SELECT amount FROM cash_accounts WHERE id = ?")
            .bind(hs)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(amount, 70000.0);
    }

    /// A debit the account can't cover is a validation error: the plan
    /// survives and no deposit is created.
    #[tokio::test]
    async fn convert_with_an_overdraft_keeps_the_plan() {
        let pool = connect_memory().await.expect("memory db");
        let hs = seed_bank(&pool, "HS", 50000.0).await;
        let item = store::insert_item(
            &pool,
            "2099-10-01",
            "hs_deposit",
            -130000.0,
            &None,
            &None,
            1,
        )
        .await
        .unwrap();

        let mut body = deposit_body();
        body.debit_account_id = Some(hs);
        let err = convert(
            State(AppState { pool: pool.clone() }),
            Path(item),
            Json(body),
        )
        .await
        .expect_err("overdraft");
        let ApiError::Validation(fields) = err else {
            panic!("expected validation");
        };
        assert_eq!(fields[0].field, "debit_account_id");
        assert!(store::load_item(&pool, item).await.is_ok());
        let deposits: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM deposits")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(deposits, 0);
    }

    /// A start month with no month row records no item (the dep-start
    /// suggestion covers it); the deposit and the plan delete still land.
    #[tokio::test]
    async fn convert_without_a_month_row_records_no_item() {
        let pool = connect_memory().await.expect("memory db");
        let item = store::insert_item(&pool, "2099-10-01", "sc_deposit", -55000.0, &None, &None, 1)
            .await
            .unwrap();

        let (status, deposit) = convert(
            State(AppState { pool: pool.clone() }),
            Path(item),
            Json(deposit_body()),
        )
        .await
        .expect("convert");

        assert_eq!(status, StatusCode::CREATED);
        let items: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM month_items WHERE auto_key = ?")
            .bind(format!("dep-start:{}", deposit.id))
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(items, 0);
        assert!(matches!(
            store::load_item(&pool, item).await,
            Err(crate::store::StoreError::NotFound(_))
        ));
    }
}
