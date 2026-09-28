use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use sqlx::sqlite::SqliteRow;
use sqlx::{Row, SqlitePool};

use super::{AppState, deposits, months, mpf, now_timestamp};
use crate::calc::{self, FieldError, ForecastItemFacts};
use crate::error::{ApiError, ErrorBody};
use crate::models::{
    Deposit, ForecastItem, ForecastItemKind, ForecastItemPatch, ForecastMonth, ForecastResponse,
    NewDeposit, NewForecastItem,
};

const ITEM_COLUMNS: &str = "id, month, kind, amount, return_month, note, sort_order, created_at";

fn row_to_item(row: &SqliteRow) -> Result<ForecastItem, ApiError> {
    let kind: String = row.try_get("kind")?;
    Ok(ForecastItem {
        id: row.try_get("id")?,
        month: row.try_get("month")?,
        kind: ForecastItemKind::parse(&kind)
            .ok_or_else(|| ApiError::Conflict(format!("stored kind {kind} is not valid")))?,
        amount: row.try_get("amount")?,
        return_month: row.try_get("return_month")?,
        note: row.try_get("note")?,
        sort_order: row.try_get("sort_order")?,
        created_at: row.try_get("created_at")?,
    })
}

async fn load_item(pool: &SqlitePool, id: i64) -> Result<ForecastItem, ApiError> {
    let row = sqlx::query(sqlx::AssertSqlSafe(format!(
        "SELECT {ITEM_COLUMNS} FROM forecast_items WHERE id = ?"
    )))
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| ApiError::NotFound(format!("forecast item {id} not found")))?;
    row_to_item(&row)
}

async fn load_all_items(pool: &SqlitePool) -> Result<Vec<ForecastItem>, ApiError> {
    let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
        "SELECT {ITEM_COLUMNS} FROM forecast_items ORDER BY month, sort_order, id"
    )))
    .fetch_all(pool)
    .await?;
    rows.iter().map(row_to_item).collect()
}

fn parse_stored_ym(value: &str, field: &str) -> Result<chrono::NaiveDate, ApiError> {
    chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|_| ApiError::Conflict(format!("stored {field} {value} is not valid")))
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
    months::parse_ym(&value)
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
    get,
    path = "/api/forecast",
    tag = "forecast",
    responses(
        (status = 200, description = "Seven-month rolling cash forecast", body = ForecastResponse),
    )
)]
pub async fn forecast(State(state): State<AppState>) -> Result<Json<ForecastResponse>, ApiError> {
    let pool = &state.pool;
    let input = months::live_totals_input(pool).await?;
    let totals = calc::live_totals(&input);
    let salary = mpf::meta_f64(pool, months::SALARY_KEY).await?;
    let bill_amount = mpf::meta_f64(pool, months::BILL_AMOUNT_KEY)
        .await?
        .unwrap_or(months::DEFAULT_BILL_AMOUNT);
    let first = months::live_from();
    // The first column anchors on the current month's 月初(出糧後); while it is
    // unset the live 活期 sum stands in.
    let start = months::load_month(pool, &first.to_string())
        .await?
        .and_then(|stored| stored.start_cash)
        .or(Some(input.cash_sum));

    let stat_rows = months::load_stat_rows(pool, Some(&totals)).await?;
    let month_items = months::load_all_items(pool).await?;
    let trailing = calc::trailing_averages(&stat_rows, &month_items, first);
    let spend = trailing.living_budget.map(|budget| -budget);
    let events = months::load_interest_events(pool).await?;

    let items = load_all_items(pool).await?;
    let mut facts = Vec::with_capacity(items.len());
    for item in &items {
        facts.push(ForecastItemFacts {
            month: parse_stored_ym(&item.month, "month")?,
            kind: item.kind,
            amount: item.amount,
            return_month: item
                .return_month
                .as_deref()
                .map(|value| parse_stored_ym(value, "return_month"))
                .transpose()?,
        });
    }

    let derived = calc::forecast_months(&calc::ForecastInput {
        first_month: first,
        start,
        salary,
        spend,
        liquid_assets: totals.liquid_assets,
        locked_base: input.deposits_active_principal,
        bill_amount,
        deposits: &events.deposits,
        events: &events,
        items: &facts,
    });
    let months_out = derived
        .iter()
        .map(|column| {
            let month = column.month.to_string();
            ForecastMonth {
                interest_components: calc::interest_components(&month, &events),
                plan_items: items
                    .iter()
                    .filter(|item| item.month == month)
                    .cloned()
                    .collect(),
                month,
                start: column.start,
                salary,
                spend,
                deposit_finish: column.deposit_finish,
                interest: column.interest,
                bill: column.bill,
                deposit_return: column.deposit_return,
                cash: column.cash,
                locked: column.locked,
                semi_liquid: column.semi_liquid,
                ref_check: column.ref_check,
            }
        })
        .collect();
    Ok(Json(ForecastResponse {
        bill_amount,
        months: months_out,
    }))
}

#[utoipa::path(
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
    let month = months::parse_ym(&ym)?.to_string();

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
    let sort_order: i64 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(sort_order), 0) + 1 FROM forecast_items WHERE month = ?",
    )
    .bind(&month)
    .fetch_one(&state.pool)
    .await?;
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO forecast_items (month, kind, amount, return_month, note, sort_order, \
         created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(&month)
    .bind(body.kind.as_str())
    .bind(body.amount)
    .bind(&return_month)
    .bind(&note)
    .bind(sort_order)
    .bind(now_timestamp())
    .bind(now_timestamp())
    .fetch_one(&state.pool)
    .await?;
    Ok((StatusCode::CREATED, Json(load_item(&state.pool, id).await?)))
}

#[utoipa::path(
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

    sqlx::query(
        "UPDATE forecast_items SET kind = ?, amount = ?, return_month = ?, note = ?, \
         updated_at = ? WHERE id = ?",
    )
    .bind(kind.as_str())
    .bind(amount)
    .bind(&return_month)
    .bind(&note)
    .bind(now_timestamp())
    .bind(id)
    .execute(&state.pool)
    .await?;
    Ok(Json(load_item(&state.pool, id).await?))
}

#[utoipa::path(
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
    let result = sqlx::query("DELETE FROM forecast_items WHERE id = ?")
        .bind(id)
        .execute(&state.pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound(format!("forecast item {id} not found")));
    }
    Ok(StatusCode::NO_CONTENT)
}

/// 轉為定期: convert a deposit plan into a real `deposits` row and delete the
/// plan — in one transaction, so the plan's derived return/lock effects can
/// never double-count against the deposit.
#[utoipa::path(
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

    let mut tx = state.pool.begin().await?;
    let sort_order: i64 =
        sqlx::query_scalar("SELECT COALESCE(MAX(sort_order), 0) + 1 FROM deposits")
            .fetch_one(&mut *tx)
            .await?;
    let deposit_id = deposits::insert_deposit(&mut *tx, &validated, sort_order, &body).await?;
    sqlx::query("DELETE FROM forecast_items WHERE id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    Ok((
        StatusCode::CREATED,
        Json(deposits::load_one(&state.pool, deposit_id).await?),
    ))
}
