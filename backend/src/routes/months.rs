use std::collections::HashSet;

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use chrono::Datelike;
use serde::{Deserialize, Serialize};

use super::patch::{merge, merge_opt, merge_text};
use super::{AppState, parse_ym};
use crate::calc::{
    FieldError, MonthRunningAverages, MonthYearSummary, build_suggestions, interest_components,
    live_totals, month_derived_with_tail, month_item_sums, month_running_averages,
    month_year_summaries, pool_balances, pool_rate_for_year, suggestions_enabled,
};
use crate::error::{ApiError, ErrorBody};
use crate::models::{
    InterestComponent, ManualAsset, ManualAssetPatch, MonthItem, MonthItemCategory, MonthItemPatch,
    MonthStat, MonthStatPatch, MonthSuggestion, NewManualAsset, NewMonthItem,
};
use crate::store::months as store;
use crate::store::{now_timestamp, today};

#[derive(Debug, Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
pub struct ListQuery {
    /// Restrict to rows of this year.
    pub year: Option<i32>,
}

#[utoipa::path(
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
    let stored = store::load_all_months(&state.pool).await?;
    let items = store::load_all_items(&state.pool).await?;
    let interest_events = crate::service::months::load_interest_events(&state.pool).await?;
    let live = if crate::service::months::wants_live(&stored) {
        Some(crate::service::months::live_if_needed(&state.pool).await?)
    } else {
        None
    };
    let stat_rows =
        crate::service::months::to_stat_rows(&stored, live.as_ref(), &interest_events, &items)?;
    let derived = month_derived_with_tail(
        &stat_rows,
        &items,
        crate::service::months::live_tail(&stat_rows, live),
    );
    let months: Vec<MonthStat> = stored
        .iter()
        .zip(derived.iter())
        .enumerate()
        .map(|(index, (row, derived))| {
            let sums = items
                .get(&stat_rows[index].month)
                .map(|items| month_item_sums(items))
                .unwrap_or_default();
            crate::service::months::present(
                row,
                derived,
                live.as_ref(),
                &sums,
                stat_rows[index].interest,
            )
        })
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
    /// The current year's effective pool rate (incl. earlier-year fallback).
    pub pool_rate: Option<f64>,
    pub pool_rate_year: i32,
}

#[utoipa::path(
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
    let stored = store::load_all_months(&state.pool).await?;
    let items = store::load_all_items(&state.pool).await?;
    let interest_events = crate::service::months::load_interest_events(&state.pool).await?;
    let rates = store::pool_rates(&state.pool).await?;
    // 投資純利 = Σ interest + the year's stored HK sold P/L (absent while a
    // year has none stored).
    let hk_sold_pl = crate::store::yearly::hk_sold_pl(&state.pool).await?;
    let live = if crate::service::months::wants_live(&stored) {
        Some(crate::service::months::live_if_needed(&state.pool).await?)
    } else {
        None
    };
    let stat_rows =
        crate::service::months::to_stat_rows(&stored, live.as_ref(), &interest_events, &items)?;
    let tail = crate::service::months::live_tail(&stat_rows, live);
    let years = month_year_summaries(&stat_rows, &items, &rates, &hk_sold_pl, tail);
    let running = month_running_averages(&stat_rows, &items, tail);
    let pool_balance = stat_rows
        .last()
        .and_then(|latest| {
            pool_balances(&stat_rows, &items, &rates)
                .get(&latest.month.year())
                .copied()
        })
        .unwrap_or(0.0);
    let pool_rate_year = today().year();
    Ok(Json(MonthSummaryResponse {
        years,
        running,
        pool_balance,
        pool_rate: pool_rate_for_year(&rates, pool_rate_year),
        pool_rate_year,
    }))
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct MonthDetailResponse {
    pub month: MonthStat,
    pub items: Vec<MonthItem>,
    /// Computed candidates for the current/future month; never stored until
    /// accepted. Empty for past months.
    pub suggestions: Vec<MonthSuggestion>,
    /// The auto 利息 components: deposits ending + received coupons + received
    /// HK dividends dated in the month. `month.interest` = Σ these + Σ
    /// `interest` items.
    pub interest_auto: Vec<InterestComponent>,
}

#[utoipa::path(
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
    let stored = store::load_month(&state.pool, &month_key)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("month {month_key} not found")))?;

    let items = store::load_items(&state.pool, &month_key).await?;
    let events = crate::service::months::load_events(&state.pool, month, stored.pool_input).await?;
    let suggestions = if suggestions_enabled(&month_key, today()) {
        let stored_keys: HashSet<String> = items
            .iter()
            .filter_map(|item| item.auto_key.clone())
            .collect();
        let dismissed = store::dismissed_keys(&state.pool, &month_key).await?;
        build_suggestions(&month_key, &events, &stored_keys, &dismissed)
    } else {
        Vec::new()
    };

    Ok(Json(MonthDetailResponse {
        month: crate::service::months::present_month(&state.pool, &stored).await?,
        items,
        suggestions,
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
    let month_key = month.to_string();
    validate_figures(&patch)?;

    let existing = store::load_month(&state.pool, &month_key).await?;
    let now = now_timestamp();

    match existing {
        None => {
            // Body-supplied totals win; otherwise snapshot the live values.
            let live = live_totals(&crate::service::months::live_totals_input(&state.pool).await?);
            let total_assets = patch.total_assets.unwrap_or(Some(live.total_assets));
            let liquid_assets = patch.liquid_assets.unwrap_or(Some(live.liquid_assets));
            let salary = match patch.salary {
                Some(salary) => Some(salary),
                None => {
                    crate::store::settings::Settings::load(&state.pool)
                        .await?
                        .salary
                }
            };
            store::insert_month(
                &state.pool,
                &month_key,
                patch.start_cash,
                salary,
                total_assets,
                liquid_assets,
                patch.pool_input.unwrap_or(0.0),
                patch.end_cash_override.flatten(),
                patch.note.flatten(),
                &now,
            )
            .await?;
        }
        Some(existing) => {
            // A present field wins over recapture, recapture wins over the
            // stored value, and `null` stores NULL (live).
            let input = if patch.recapture == Some(true) {
                Some(crate::service::months::live_totals_input(&state.pool).await?)
            } else {
                None
            };
            let live = input.map(|input| live_totals(&input));
            let start_cash = patch
                .start_cash
                .or_else(|| input.map(|input| input.cash_sum))
                .or(existing.start_cash);
            let total_assets = match (patch.total_assets, live) {
                (Some(value), _) => value,
                (None, Some(live)) => Some(live.total_assets),
                (None, None) => existing.total_assets,
            };
            let liquid_assets = match (patch.liquid_assets, live) {
                (Some(value), _) => value,
                (None, Some(live)) => Some(live.liquid_assets),
                (None, None) => existing.liquid_assets,
            };
            store::update_month(
                &state.pool,
                &month_key,
                start_cash,
                merge_opt(patch.salary, existing.salary),
                total_assets,
                liquid_assets,
                merge(patch.pool_input, existing.pool_input),
                patch
                    .end_cash_override
                    .unwrap_or(existing.end_cash_override),
                merge_text(patch.note, existing.note),
                &now,
            )
            .await?;
        }
    }

    let stored = store::load_month(&state.pool, &month_key)
        .await?
        .expect("just written");
    Ok(Json(
        crate::service::months::present_month(&state.pool, &stored).await?,
    ))
}

/// Deleting a month removes its items via the FK cascade; dismissal tombstones
/// stay (they are per-month keys, harmless without the row).
#[utoipa::path(
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
    if store::load_month(&state.pool, &month_key).await?.is_none() {
        return Err(ApiError::NotFound(format!("month {month_key} not found")));
    }
    if !body.amount.is_finite() {
        return Err(ApiError::field("amount", "amount must be a number"));
    }
    if body.exclude_from_living && body.category != MonthItemCategory::Entertainment {
        return Err(ApiError::field(
            "exclude_from_living",
            "exclude_from_living only applies to 娛樂支出 items",
        ));
    }
    let label = body
        .label
        .map(|label| label.trim().to_string())
        .filter(|label| !label.is_empty());
    let auto_key = body
        .auto_key
        .map(|key| key.trim().to_string())
        .filter(|key| !key.is_empty());
    let note = body
        .note
        .map(|note| note.trim().to_string())
        .filter(|note| !note.is_empty());

    let mut tx = state.pool.begin().await?;
    if let Some(auto_key) = &auto_key {
        // Accepting a suggestion clears any dismissal of the same key.
        store::clear_dismissal(&mut tx, &month_key, auto_key).await?;
    }
    // A duplicate (month, auto_key) hits the partial unique index → 409.
    let id = store::insert_item(
        &mut tx,
        &month_key,
        body.category.as_str(),
        &label,
        body.amount,
        body.exclude_from_living,
        &auto_key,
        &note,
    )
    .await?;
    tx.commit().await?;

    Ok((
        StatusCode::CREATED,
        Json(store::item_row(&state.pool, id).await?),
    ))
}

#[utoipa::path(
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
    let existing = store::load_item(&state.pool, id).await?;

    let category = patch.category.unwrap_or(existing.category);
    let label = merge_text(patch.label, existing.label);
    let amount = patch.amount.unwrap_or(existing.amount);
    let note = merge_text(patch.note, existing.note);
    if !amount.is_finite() {
        return Err(ApiError::field("amount", "amount must be a number"));
    }
    // The flag is entertainment-only: moving an item to another category
    // clears it, while setting it on a non-entertainment item is rejected.
    let exclude_from_living = match (patch.exclude_from_living, category) {
        (Some(true), MonthItemCategory::Entertainment) => true,
        (Some(true), _) => {
            return Err(ApiError::field(
                "exclude_from_living",
                "exclude_from_living only applies to 娛樂支出 items",
            ));
        }
        (Some(false), _) => false,
        (None, MonthItemCategory::Entertainment) => existing.exclude_from_living,
        (None, _) => false,
    };

    store::update_item(
        &state.pool,
        id,
        category.as_str(),
        &label,
        amount,
        exclude_from_living,
        &note,
    )
    .await?;

    Ok(Json(store::item_row(&state.pool, id).await?))
}

#[utoipa::path(
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
pub struct DismissBody {
    pub auto_key: String,
}

/// Tombstone a suggestion key so it never reappears for the month.
#[utoipa::path(
    post,
    path = "/api/months/{ym}/items/dismiss",
    tag = "months",
    params(("ym" = String, Path, description = "Month key YYYY-MM")),
    request_body = DismissBody,
    responses(
        (status = 204, description = "Suggestion dismissed (tombstone written)"),
        (status = 400, description = "Invalid month key or empty auto_key", body = ErrorBody),
    )
)]
pub async fn dismiss_item(
    State(state): State<AppState>,
    Path(ym): Path<String>,
    Json(body): Json<DismissBody>,
) -> Result<StatusCode, ApiError> {
    let month_key = parse_ym(&ym)?.to_string();
    let auto_key = body.auto_key.trim();
    if auto_key.is_empty() {
        return Err(ApiError::field("auto_key", "auto_key is required"));
    }
    store::dismiss_item(&state.pool, &month_key, auto_key).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/api/manual-assets",
    tag = "months",
    responses(
        (status = 200, description = "List manual asset rows", body = [ManualAsset]),
    )
)]
pub async fn list_assets(
    State(state): State<AppState>,
) -> Result<Json<Vec<ManualAsset>>, ApiError> {
    store::load_assets(&state.pool)
        .await
        .map(Json)
        .map_err(ApiError::from)
}

fn validate_asset(label: &str, amount: f64) -> Result<String, ApiError> {
    let label = label.trim();
    if label.is_empty() {
        return Err(ApiError::field("label", "label is required"));
    }
    if !amount.is_finite() {
        return Err(ApiError::field("amount", "amount must be a number"));
    }
    Ok(label.to_string())
}

#[utoipa::path(
    post,
    path = "/api/manual-assets",
    tag = "months",
    request_body = NewManualAsset,
    responses(
        (status = 201, description = "Manual asset created", body = ManualAsset),
        (status = 400, description = "Missing or invalid fields", body = ErrorBody),
    )
)]
pub async fn create_asset(
    State(state): State<AppState>,
    Json(body): Json<NewManualAsset>,
) -> Result<(StatusCode, Json<ManualAsset>), ApiError> {
    let label = validate_asset(&body.label, body.amount)?;
    let sort_order = store::next_asset_sort_order(&state.pool).await?;
    let liquidity = body.liquidity.unwrap_or_default();
    let id = store::insert_asset(
        &state.pool,
        &label,
        liquidity.as_str(),
        body.amount,
        sort_order,
    )
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(store::load_asset(&state.pool, id).await?),
    ))
}

#[utoipa::path(
    patch,
    path = "/api/manual-assets/{id}",
    tag = "months",
    params(("id" = i64, Path, description = "Manual asset id")),
    request_body = ManualAssetPatch,
    responses(
        (status = 200, description = "Updated manual asset", body = ManualAsset),
        (status = 400, description = "Missing or invalid fields", body = ErrorBody),
        (status = 404, description = "Manual asset not found", body = ErrorBody),
    )
)]
pub async fn update_asset(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(patch): Json<ManualAssetPatch>,
) -> Result<Json<ManualAsset>, ApiError> {
    let existing = store::load_asset(&state.pool, id).await?;
    let label = patch.label.unwrap_or(existing.label);
    let liquidity = patch.liquidity.unwrap_or(existing.liquidity);
    let amount = patch.amount.unwrap_or(existing.amount);
    let label = validate_asset(&label, amount)?;
    store::update_asset(&state.pool, id, &label, liquidity.as_str(), amount).await?;
    Ok(Json(store::load_asset(&state.pool, id).await?))
}

#[utoipa::path(
    delete,
    path = "/api/manual-assets/{id}",
    tag = "months",
    params(("id" = i64, Path, description = "Manual asset id")),
    responses(
        (status = 204, description = "Manual asset deleted"),
        (status = 404, description = "Manual asset not found", body = ErrorBody),
    )
)]
pub async fn remove_asset(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    if store::delete_asset(&state.pool, id).await? == 0 {
        return Err(ApiError::NotFound(format!("manual asset {id} not found")));
    }
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connect_memory;
    use crate::models::{AppSettingsPatch, ManualAssetLiquidity};
    use crate::routes::settings;

    #[tokio::test]
    async fn create_asset_defaults_liquidity_to_long() {
        let pool = connect_memory().await.expect("memory db");
        let created = create_asset(
            State(AppState { pool: pool.clone() }),
            Json(NewManualAsset {
                label: "HS人壽".to_string(),
                liquidity: None,
                amount: 69440.47,
            }),
        )
        .await
        .expect("created")
        .1;
        assert_eq!(created.liquidity, ManualAssetLiquidity::Long);
    }

    #[tokio::test]
    async fn patch_asset_reclassifies_liquidity() {
        let pool = connect_memory().await.expect("memory db");
        let state = AppState { pool: pool.clone() };
        let created = create_asset(
            State(state.clone()),
            Json(NewManualAsset {
                label: "Irene".to_string(),
                liquidity: None,
                amount: 20000.0,
            }),
        )
        .await
        .expect("created")
        .1;

        let updated = update_asset(
            State(state),
            Path(created.id),
            Json(ManualAssetPatch {
                liquidity: Some(ManualAssetLiquidity::Short),
                ..Default::default()
            }),
        )
        .await
        .expect("patched")
        .0;
        assert_eq!(updated.liquidity, ManualAssetLiquidity::Short);

        let stored = store::load_asset(&pool, created.id).await.expect("stored");
        assert_eq!(stored.liquidity, ManualAssetLiquidity::Short);
    }

    #[tokio::test]
    async fn update_settings_round_trips_the_semi_liquid_target() {
        let pool = connect_memory().await.expect("memory db");
        let state = AppState { pool: pool.clone() };

        // While unset the effective ratio is the sheet's 25% literal.
        let current = settings::get(State(state.clone()))
            .await
            .expect("settings")
            .0;
        assert_eq!(current.semi_liquid_target, 0.25);

        let updated = settings::update(
            State(state.clone()),
            Json(AppSettingsPatch {
                semi_liquid_target: Some(Some(0.3)),
                ..Default::default()
            }),
        )
        .await
        .expect("patched")
        .0;
        assert_eq!(updated.semi_liquid_target, 0.3);
        assert_eq!(
            crate::service::months::semi_liquid_target(&pool)
                .await
                .unwrap(),
            0.3
        );

        // `null` clears the stored value — back to the default.
        let reset = settings::update(
            State(state.clone()),
            Json(AppSettingsPatch {
                semi_liquid_target: Some(None),
                ..Default::default()
            }),
        )
        .await
        .expect("reset")
        .0;
        assert_eq!(reset.semi_liquid_target, 0.25);

        // A share of 100%+ is meaningless for the buffer and rejected.
        let err = settings::update(
            State(state),
            Json(AppSettingsPatch {
                semi_liquid_target: Some(Some(1.5)),
                ..Default::default()
            }),
        )
        .await
        .expect_err("rejected");
        assert!(matches!(err, ApiError::Validation(_)));
    }

    #[tokio::test]
    async fn update_settings_round_trips_the_goal_settings() {
        let pool = connect_memory().await.expect("memory db");
        let state = AppState { pool };

        // The constants fall back to the sheet literals while unset.
        let current = settings::get(State(state.clone()))
            .await
            .expect("settings")
            .0;
        assert_eq!(current.goal1_amount, None);
        assert_eq!(current.goal2_amount, None);
        assert_eq!(
            current.goal3_inflation,
            crate::service::months::DEFAULT_GOAL3_INFLATION
        );
        assert_eq!(
            current.goal3_years,
            crate::service::months::DEFAULT_GOAL3_YEARS
        );
        assert_eq!(
            current.final_replacement,
            crate::service::months::DEFAULT_FINAL_REPLACEMENT
        );

        let updated = settings::update(
            State(state.clone()),
            Json(AppSettingsPatch {
                goal1_amount: Some(Some(1_550_000.0)),
                goal3_inflation: Some(Some(1.05)),
                ..Default::default()
            }),
        )
        .await
        .expect("patched")
        .0;
        assert_eq!(updated.goal1_amount, Some(1_550_000.0));
        assert_eq!(updated.goal3_inflation, 1.05);
        assert_eq!(
            updated.goal3_years,
            crate::service::months::DEFAULT_GOAL3_YEARS
        );

        // `null` clears an amount and resets a constant to its default.
        let reset = settings::update(
            State(state.clone()),
            Json(AppSettingsPatch {
                goal1_amount: Some(None),
                goal3_inflation: Some(None),
                ..Default::default()
            }),
        )
        .await
        .expect("reset")
        .0;
        assert_eq!(reset.goal1_amount, None);
        assert_eq!(
            reset.goal3_inflation,
            crate::service::months::DEFAULT_GOAL3_INFLATION
        );

        // Negative amounts and non-positive constants are rejected.
        for patch in [
            AppSettingsPatch {
                goal1_amount: Some(Some(-1.0)),
                ..Default::default()
            },
            AppSettingsPatch {
                goal3_years: Some(Some(0.0)),
                ..Default::default()
            },
            AppSettingsPatch {
                final_replacement: Some(Some(-0.5)),
                ..Default::default()
            },
        ] {
            let err = settings::update(State(state.clone()), Json(patch))
                .await
                .expect_err("rejected");
            assert!(matches!(err, ApiError::Validation(_)));
        }
    }

    #[tokio::test]
    async fn unknown_liquidity_is_rejected_with_a_client_error() {
        let pool = connect_memory().await.expect("memory db");
        let (router, _api) = crate::routes::api_router(AppState { pool });
        let response = tower::ServiceExt::oneshot(
            router,
            axum::http::Request::builder()
                .method("POST")
                .uri("/api/manual-assets")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(
                    r#"{"label":"X","kind":"asset","amount":1,"liquidity":"medium"}"#,
                ))
                .unwrap(),
        )
        .await
        .expect("response");
        assert!(
            response.status().is_client_error(),
            "expected 4xx, got {}",
            response.status()
        );
    }
}
