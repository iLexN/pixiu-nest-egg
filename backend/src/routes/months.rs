use std::collections::HashSet;

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use chrono::Datelike;
use serde::{Deserialize, Serialize};

use super::{AppState, parse_ym};
use crate::calc::{
    FieldError, MonthRunningAverages, MonthYearSummary, PoolTotals, build_suggestions,
    interest_components, pool_totals, suggestion_actions_enabled,
};
use crate::error::{ApiError, ErrorBody};
use crate::models::{
    InterestComponent, MonthItem, MonthItemPatch, MonthStat, MonthStatPatch, MonthSuggestion,
    NewMonthItem,
};
use crate::store::months as store;
use crate::store::today;

#[derive(Debug, Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
pub struct ListQuery {
    /// Restrict to rows of this year.
    pub year: Option<i32>,
}

#[utoipa::path(
    operation_id = "months_list",
    get,
    path = "/api/months",
    tag = "months",
    params(ListQuery),
    responses(
        (status = 200, description = "List month-stat rows with derived columns", body = [MonthStat]),
    )
)]
pub async fn list(
    State(state): State<AppState>,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<MonthStat>>, ApiError> {
    let book = crate::service::months::MonthBook::open(&state.pool).await?;
    let months: Vec<MonthStat> = book
        .months()
        .into_iter()
        .filter(|month| {
            query
                .year
                .is_none_or(|year| month.month.starts_with(&format!("{year:04}-")))
        })
        .collect();
    Ok(Json(months))
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct MonthSummaryResponse {
    /// The sheet's rows 2–4, one per year present.
    pub years: Vec<MonthYearSummary>,
    /// The sheet's row-8 running averages.
    pub running: MonthRunningAverages,
    /// The running 開心Pool balance through the latest stored month's year.
    pub pool_balance: f64,
    /// The all-years 開心Pool contribution totals (iLex input, Irene input,
    /// Irene payable) and per-active-year averages.
    pub pool_totals: PoolTotals,
    /// The current year's effective pool rate (incl. earlier-year fallback).
    pub pool_rate: Option<f64>,
    pub pool_rate_year: i32,
}

#[utoipa::path(
    operation_id = "months_summary",
    get,
    path = "/api/months/summary",
    tag = "months",
    responses(
        (status = 200, description = "Month rows, pool balance chain, and running averages", body = MonthSummaryResponse),
    )
)]
pub async fn summary(
    State(state): State<AppState>,
) -> Result<Json<MonthSummaryResponse>, ApiError> {
    let book = crate::service::months::MonthBook::open(&state.pool).await?;
    // 投資純利 = Σ interest + the year's stored HK sold P/L (absent while a
    // year has none stored).
    let years = book.year_summaries();
    let pool_rate_year = today().year();
    let pool_totals = pool_totals(&years);
    Ok(Json(MonthSummaryResponse {
        years,
        running: book.running_averages(),
        pool_balance: book.pool_balance(),
        pool_totals,
        pool_rate: book.pool_rate(pool_rate_year),
        pool_rate_year,
    }))
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct MonthDetailResponse {
    pub month: MonthStat,
    pub items: Vec<MonthItem>,
    /// Computed candidates for the month; never stored until accepted.
    /// Dismissed ones stay listed, flagged `dismissed`.
    pub suggestions: Vec<MonthSuggestion>,
    /// Whether the month offers candidate actions (accept/dismiss/undismiss) —
    /// the previous month and later. Older months' suggestions are
    /// informational only.
    pub suggestions_actionable: bool,
    /// The auto 利息 components: deposits ending + received coupons + received
    /// HK dividends dated in the month. `month.interest` = Σ these + Σ
    /// `interest` items.
    pub interest_auto: Vec<InterestComponent>,
}

#[utoipa::path(
    operation_id = "months_show",
    get,
    path = "/api/months/{ym}",
    tag = "months",
    params(("ym" = String, Path, description = "Month key YYYY-MM")),
    responses(
        (status = 200, description = "Month detail: derived columns, items, interest breakdown, suggestions", body = MonthDetailResponse),
        (status = 400, description = "Invalid month key", body = ErrorBody),
    )
)]
pub async fn show(
    State(state): State<AppState>,
    Path(ym): Path<String>,
) -> Result<Json<MonthDetailResponse>, ApiError> {
    let month = parse_ym(&ym)?;
    let month_key = month.to_string();
    let stat = crate::service::months::MonthBook::open(&state.pool)
        .await?
        .month(&month_key)
        .ok_or_else(|| ApiError::NotFound(format!("month {month_key} not found")))?;

    let items = store::load_items(&state.pool, &month_key).await?;
    let events = crate::service::months::load_events(&state.pool, month).await?;
    let stored_keys: HashSet<String> = items
        .iter()
        .filter_map(|item| item.auto_key.clone())
        .collect();
    let dismissed = store::dismissed_keys(&state.pool, &month_key).await?;
    let suggestions = build_suggestions(&month_key, &events, &stored_keys, &dismissed);

    Ok(Json(MonthDetailResponse {
        month: stat,
        items,
        suggestions,
        suggestions_actionable: suggestion_actions_enabled(&month_key, today()),
        interest_auto: interest_components(&month_key, &events),
    }))
}

/// Validate the figures a patch may carry; absent fields skip the check.
fn validate_figures(patch: &MonthStatPatch) -> Result<(), ApiError> {
    let mut errors = Vec::new();
    for (field, value, name) in [
        ("start_cash", patch.start_cash, "月初"),
        ("salary", patch.salary, "salary"),
        (
            "end_cash_override",
            patch.end_cash_override.flatten(),
            "月尾",
        ),
        ("pool_input", patch.pool_input, "Irene + 開心 Pool"),
    ] {
        if let Some(value) = value
            && !value.is_finite()
        {
            errors.push(FieldError::new(field, format!("{name} must be a number")));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(ApiError::Validation(errors))
    }
}

/// Upsert a month row. Creating snapshots the live totals and the current
/// salary; explicit `null` on a total keeps it live, `recapture` re-snapshots
/// the totals plus 月初 (the live 活期 cash sum).
#[utoipa::path(
    operation_id = "months_upsert",
    patch,
    path = "/api/months/{ym}",
    tag = "months",
    params(("ym" = String, Path, description = "Month key YYYY-MM")),
    request_body = MonthStatPatch,
    responses(
        (status = 200, description = "Upserted month row", body = MonthStat),
        (status = 400, description = "Invalid month key or fields", body = ErrorBody),
    )
)]
pub async fn upsert(
    State(state): State<AppState>,
    Path(ym): Path<String>,
    Json(patch): Json<MonthStatPatch>,
) -> Result<Json<MonthStat>, ApiError> {
    let month = parse_ym(&ym)?;
    validate_figures(&patch)?;
    Ok(Json(
        crate::service::months::upsert(&state.pool, month, &patch).await?,
    ))
}

/// Deleting a month removes its items via the FK cascade; dismissal tombstones
/// stay (they are per-month keys, harmless without the row).
#[utoipa::path(
    operation_id = "months_remove",
    delete,
    path = "/api/months/{ym}",
    tag = "months",
    params(("ym" = String, Path, description = "Month key YYYY-MM")),
    responses(
        (status = 204, description = "Month row deleted"),
        (status = 400, description = "Invalid month key", body = ErrorBody),
        (status = 404, description = "Month not found", body = ErrorBody),
    )
)]
pub async fn remove(
    State(state): State<AppState>,
    Path(ym): Path<String>,
) -> Result<StatusCode, ApiError> {
    let month_key = parse_ym(&ym)?.to_string();
    if store::delete_month(&state.pool, &month_key).await? == 0 {
        return Err(ApiError::NotFound(format!("month {month_key} not found")));
    }
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    operation_id = "months_create_item",
    post,
    path = "/api/months/{ym}/items",
    tag = "months",
    params(("ym" = String, Path, description = "Month key YYYY-MM")),
    request_body = NewMonthItem,
    responses(
        (status = 201, description = "Month item created", body = MonthItem),
        (status = 400, description = "Invalid month key or fields", body = ErrorBody),
        (status = 404, description = "Month not found", body = ErrorBody),
        (status = 409, description = "Auto item with this key already exists", body = ErrorBody),
    )
)]
pub async fn create_item(
    State(state): State<AppState>,
    Path(ym): Path<String>,
    Json(body): Json<NewMonthItem>,
) -> Result<(StatusCode, Json<MonthItem>), ApiError> {
    let month_key = parse_ym(&ym)?.to_string();
    if !body.amount.is_finite() {
        return Err(ApiError::field("amount", "amount must be a number"));
    }
    let item = crate::service::months::create_item(&state.pool, &month_key, &body).await?;
    Ok((StatusCode::CREATED, Json(item)))
}

#[utoipa::path(
    operation_id = "months_update_item",
    patch,
    path = "/api/month-items/{id}",
    tag = "months",
    params(("id" = i64, Path, description = "Month item id")),
    request_body = MonthItemPatch,
    responses(
        (status = 200, description = "Updated month item", body = MonthItem),
        (status = 400, description = "Missing or invalid fields", body = ErrorBody),
        (status = 404, description = "Month item not found", body = ErrorBody),
    )
)]
pub async fn update_item(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(patch): Json<MonthItemPatch>,
) -> Result<Json<MonthItem>, ApiError> {
    Ok(Json(
        crate::service::months::update_item(&state.pool, id, &patch).await?,
    ))
}

#[utoipa::path(
    operation_id = "months_remove_item",
    delete,
    path = "/api/month-items/{id}",
    tag = "months",
    params(("id" = i64, Path, description = "Month item id")),
    responses(
        (status = 204, description = "Month item deleted"),
        (status = 404, description = "Month item not found", body = ErrorBody),
    )
)]
pub async fn remove_item(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    if store::delete_item(&state.pool, id).await? == 0 {
        return Err(ApiError::NotFound(format!("month item {id} not found")));
    }
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
pub struct AutoKeyBody {
    pub auto_key: String,
}

/// Tombstone a suggestion key so it stays listed flagged `dismissed`.
#[utoipa::path(
    operation_id = "months_dismiss_item",
    post,
    path = "/api/months/{ym}/items/dismiss",
    tag = "months",
    params(("ym" = String, Path, description = "Month key YYYY-MM")),
    request_body = AutoKeyBody,
    responses(
        (status = 204, description = "Suggestion dismissed (tombstone written)"),
        (status = 400, description = "Invalid month key or empty auto_key", body = ErrorBody),
    )
)]
pub async fn dismiss_item(
    State(state): State<AppState>,
    Path(ym): Path<String>,
    Json(body): Json<AutoKeyBody>,
) -> Result<StatusCode, ApiError> {
    let month_key = parse_ym(&ym)?.to_string();
    let auto_key = body.auto_key.trim();
    if auto_key.is_empty() {
        return Err(ApiError::field("auto_key", "auto_key is required"));
    }
    store::dismiss_item(&state.pool, &month_key, auto_key).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Remove a dismissal tombstone so the candidate is pending again.
#[utoipa::path(
    operation_id = "months_undismiss_item",
    post,
    path = "/api/months/{ym}/items/undismiss",
    tag = "months",
    params(("ym" = String, Path, description = "Month key YYYY-MM")),
    request_body = AutoKeyBody,
    responses(
        (status = 204, description = "Dismissal removed (candidate pending again)"),
        (status = 400, description = "Invalid month key or empty auto_key", body = ErrorBody),
    )
)]
pub async fn undismiss_item(
    State(state): State<AppState>,
    Path(ym): Path<String>,
    Json(body): Json<AutoKeyBody>,
) -> Result<StatusCode, ApiError> {
    let month_key = parse_ym(&ym)?.to_string();
    let auto_key = body.auto_key.trim();
    if auto_key.is_empty() {
        return Err(ApiError::field("auto_key", "auto_key is required"));
    }
    store::undismiss_item(&state.pool, &month_key, auto_key).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connect_memory;
    use crate::models::MonthItemCategory;

    /// A bond maturing in the current month suggests its `bond-end:<id>`
    /// adjustment through the month read.
    #[tokio::test]
    async fn a_maturing_bond_suggests_bond_end() {
        let pool = connect_memory().await.expect("memory db");
        let this_month = today().format("%Y-%m").to_string();
        sqlx::query("INSERT INTO month_stats (month, created_at, updated_at) VALUES (?, ?, ?)")
            .bind(format!("{this_month}-01"))
            .bind("2026-01-01T00:00:00")
            .bind("2026-01-01T00:00:00")
            .execute(&pool)
            .await
            .unwrap();
        let bond: i64 = sqlx::query_scalar(
            "INSERT INTO bonds (label, issue_no, principal, maturity_date, sort_order, \
             created_at, updated_at) VALUES ('iBond', 'x', 50000, ?, 1, \
             '2026-01-01T00:00:00', '2026-01-01T00:00:00') RETURNING id",
        )
        .bind(format!("{this_month}-15"))
        .fetch_one(&pool)
        .await
        .unwrap();

        let response = show(State(AppState { pool }), Path(this_month))
            .await
            .expect("show")
            .0;
        let suggestion = response
            .suggestions
            .iter()
            .find(|suggestion| suggestion.auto_key == format!("bond-end:{bond}"))
            .expect("bond-end suggestion");
        assert_eq!(suggestion.source, "bond");
        assert_eq!(suggestion.category, MonthItemCategory::Adjustment);
        assert_eq!(suggestion.label.as_deref(), Some("iBond"));
        assert_eq!(suggestion.amount, 50000.0);
        assert!(!suggestion.dismissed);
        assert!(response.suggestions_actionable);
    }

    /// A history month lists its candidates read-only — pending or dismissed,
    /// they stay in the response with `suggestions_actionable` false — and
    /// undismiss flips a dismissed one back to pending.
    #[tokio::test]
    async fn history_month_lists_dismissed_candidates_and_undismiss_restores() {
        let pool = connect_memory().await.expect("memory db");
        let month = "2020-01".to_string();
        sqlx::query("INSERT INTO month_stats (month, created_at, updated_at) VALUES (?, ?, ?)")
            .bind(format!("{month}-01"))
            .bind("2020-01-01T00:00:00")
            .bind("2020-01-01T00:00:00")
            .execute(&pool)
            .await
            .unwrap();
        let bond: i64 = sqlx::query_scalar(
            "INSERT INTO bonds (label, issue_no, principal, maturity_date, sort_order, \
             created_at, updated_at) VALUES ('iBond', 'x', 50000, ?, 1, \
             '2020-01-01T00:00:00', '2020-01-01T00:00:00') RETURNING id",
        )
        .bind(format!("{month}-15"))
        .fetch_one(&pool)
        .await
        .unwrap();
        let key = format!("bond-end:{bond}");

        let suggestion_of = async |pool: &sqlx::SqlitePool| {
            show(State(AppState { pool: pool.clone() }), Path(month.clone()))
                .await
                .expect("show")
                .0
        };
        let response = suggestion_of(&pool).await;
        assert!(!response.suggestions_actionable);
        let suggestion = response
            .suggestions
            .iter()
            .find(|s| s.auto_key == key)
            .expect("bond-end suggestion");
        assert!(!suggestion.dismissed);

        dismiss_item(
            State(AppState { pool: pool.clone() }),
            Path(month.clone()),
            Json(AutoKeyBody {
                auto_key: key.clone(),
            }),
        )
        .await
        .unwrap();
        let response = suggestion_of(&pool).await;
        let suggestion = response
            .suggestions
            .iter()
            .find(|s| s.auto_key == key)
            .expect("dismissed stays listed");
        assert!(suggestion.dismissed);

        undismiss_item(
            State(AppState { pool: pool.clone() }),
            Path(month.clone()),
            Json(AutoKeyBody {
                auto_key: key.clone(),
            }),
        )
        .await
        .unwrap();
        let response = suggestion_of(&pool).await;
        let suggestion = response
            .suggestions
            .iter()
            .find(|s| s.auto_key == key)
            .expect("undismissed pending again");
        assert!(!suggestion.dismissed);
    }
}
