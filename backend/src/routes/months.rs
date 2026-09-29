use std::collections::{BTreeMap, HashMap, HashSet};

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use chrono::Datelike;
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqliteRow;
use sqlx::{Row, SqlitePool};

use super::{AppState, aia, bonds, mpf, now_timestamp, overview, summary, today};
use crate::calc::{
    DepositFacts, FieldError, LiveTotals, LiveTotalsInput, MonthDerived, MonthItemFacts,
    MonthItemSums, MonthRunningAverages, MonthStatRow, MonthYearSummary, SuggestionAiaPayment,
    SuggestionDeposit, SuggestionEvents, SuggestionReceipt, SuggestionTrade, active_totals,
    aia_totals, auto_interest, build_suggestions, interest_components, live_totals,
    month_derived_with_tail, month_item_sums, month_running_averages, month_year_summaries,
    mpf_totals, pool_balances, pool_rate_for_year, suggestions_enabled,
};
use crate::error::{ApiError, ErrorBody};
use crate::models::{
    InterestComponent, ManualAsset, ManualAssetKind, ManualAssetLiquidity, ManualAssetPatch,
    MonthItem, MonthItemCategory, MonthItemPatch, MonthSettings, MonthSettingsPatch, MonthStat,
    MonthStatPatch, MonthSuggestion, NewManualAsset, NewMonthItem, TradeType,
};

/// `app_meta` keys holding the month-stat settings.
pub const SALARY_KEY: &str = "overview.salary";
pub const POOL_RATE_PREFIX: &str = "overview.pool_rate.";
/// `app_meta` key for the forecast's quarterly 差餉 amount.
pub const BILL_AMOUNT_KEY: &str = "forecast.bill_amount";
/// The 差餉 figure the forecast falls back to while unset.
pub const DEFAULT_BILL_AMOUNT: f64 = 2158.0;
/// `app_meta` key for the 半流動資金 buffer's target share of 流動資產.
pub const SEMI_LIQUID_TARGET_KEY: &str = "overview.semi_liquid_target";
/// The buffer ratio the sheet hardcodes (`C14`/`N7`), used while unset.
pub const DEFAULT_SEMI_LIQUID_TARGET: f64 = 0.25;

pub fn pool_rate_key(year: i32) -> String {
    format!("{POOL_RATE_PREFIX}{year}")
}

/// The configured 半流動資金 target share of 流動資產 — `0.25` while unset.
pub async fn semi_liquid_target(pool: &SqlitePool) -> Result<f64, ApiError> {
    Ok(mpf::meta_f64(pool, SEMI_LIQUID_TARGET_KEY)
        .await?
        .unwrap_or(DEFAULT_SEMI_LIQUID_TARGET))
}

/// `:ym` accepts `YYYY-MM` or `YYYY-MM-01` and normalises to `YYYY-MM-01`.
pub(crate) fn parse_ym(ym: &str) -> Result<chrono::NaiveDate, ApiError> {
    let candidate = if ym.len() == 7 {
        format!("{ym}-01")
    } else {
        ym.to_string()
    };
    let date = chrono::NaiveDate::parse_from_str(&candidate, "%Y-%m-%d")
        .map_err(|_| ApiError::field("ym", "month must be in YYYY-MM form"))?;
    if date.day() != 1 {
        return Err(ApiError::field("ym", "month must be in YYYY-MM form"));
    }
    Ok(date)
}

/// The stored `month_stats` row, before derived columns are attached.
#[derive(Debug, Clone)]
pub(crate) struct StoredMonth {
    month: String,
    pub(crate) start_cash: Option<f64>,
    salary: Option<f64>,
    total_assets: Option<f64>,
    liquid_assets: Option<f64>,
    pool_input: f64,
    end_cash_override: Option<f64>,
    note: Option<String>,
    created_at: String,
    updated_at: String,
}

const MONTH_COLUMNS: &str = "month, start_cash, salary, total_assets, liquid_assets, \
     pool_input, end_cash_override, note, created_at, updated_at";

fn row_to_month(row: &SqliteRow) -> Result<StoredMonth, ApiError> {
    Ok(StoredMonth {
        month: row.try_get("month")?,
        start_cash: row.try_get("start_cash")?,
        salary: row.try_get("salary")?,
        total_assets: row.try_get("total_assets")?,
        liquid_assets: row.try_get("liquid_assets")?,
        pool_input: row.try_get("pool_input")?,
        end_cash_override: row.try_get("end_cash_override")?,
        note: row.try_get("note")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn parse_stored_month(month: &str) -> Result<chrono::NaiveDate, ApiError> {
    chrono::NaiveDate::parse_from_str(month, "%Y-%m-%d")
        .map_err(|_| ApiError::Conflict(format!("stored month {month} is not valid")))
}

fn row_to_item(row: &SqliteRow) -> Result<MonthItem, ApiError> {
    let category: String = row.try_get("category")?;
    Ok(MonthItem {
        id: row.try_get("id")?,
        month: row.try_get("month")?,
        category: MonthItemCategory::parse(&category).ok_or_else(|| {
            ApiError::Conflict(format!("stored category {category} is not valid"))
        })?,
        label: row.try_get("label")?,
        amount: row.try_get("amount")?,
        exclude_from_living: row.try_get::<i64, _>("exclude_from_living")? != 0,
        auto_key: row.try_get("auto_key")?,
        note: row.try_get("note")?,
        created_at: row.try_get("created_at")?,
    })
}

const ITEM_COLUMNS: &str =
    "id, month, category, label, amount, exclude_from_living, auto_key, note, created_at";

async fn load_all_months(pool: &SqlitePool) -> Result<Vec<StoredMonth>, ApiError> {
    let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
        "SELECT {MONTH_COLUMNS} FROM month_stats ORDER BY month"
    )))
    .fetch_all(pool)
    .await?;
    rows.iter().map(row_to_month).collect()
}

pub(crate) async fn load_month(
    pool: &SqlitePool,
    month: &str,
) -> Result<Option<StoredMonth>, ApiError> {
    let row = sqlx::query(sqlx::AssertSqlSafe(format!(
        "SELECT {MONTH_COLUMNS} FROM month_stats WHERE month = ?"
    )))
    .bind(month)
    .fetch_optional(pool)
    .await?;
    row.as_ref().map(row_to_month).transpose()
}

pub async fn load_all_items(
    pool: &SqlitePool,
) -> Result<HashMap<chrono::NaiveDate, Vec<MonthItemFacts>>, ApiError> {
    let rows = sqlx::query(
        "SELECT month, category, amount, exclude_from_living FROM month_items \
         ORDER BY month, id",
    )
    .fetch_all(pool)
    .await?;
    let mut items: HashMap<chrono::NaiveDate, Vec<MonthItemFacts>> = HashMap::new();
    for row in &rows {
        let month: String = row.try_get("month")?;
        let category: String = row.try_get("category")?;
        let category = MonthItemCategory::parse(&category).ok_or_else(|| {
            ApiError::Conflict(format!("stored category {category} is not valid"))
        })?;
        items
            .entry(parse_stored_month(&month)?)
            .or_default()
            .push(MonthItemFacts {
                category,
                amount: row.try_get("amount")?,
                exclude_from_living: row.try_get::<i64, _>("exclude_from_living")? != 0,
            });
    }
    Ok(items)
}

/// The per-year pool rates stored as `overview.pool_rate.<year>` meta keys.
pub(crate) async fn pool_rates(pool: &SqlitePool) -> Result<BTreeMap<i32, f64>, ApiError> {
    let rows = sqlx::query("SELECT key, value FROM app_meta WHERE key LIKE 'overview.pool_rate.%'")
        .fetch_all(pool)
        .await?;
    let mut rates = BTreeMap::new();
    for row in &rows {
        let key: String = row.try_get("key")?;
        let Some(year) = key
            .strip_prefix(POOL_RATE_PREFIX)
            .and_then(|year| year.parse::<i32>().ok())
        else {
            continue;
        };
        let value: String = row.try_get("value")?;
        if let Ok(rate) = value.parse::<f64>() {
            rates.insert(year, rate);
        }
    }
    Ok(rates)
}

/// The first day of the current month: live totals only fill NULL cells at or
/// after it — an empty historical B/D is a gap, not a live cell.
pub fn live_from() -> chrono::NaiveDate {
    let today = today();
    chrono::NaiveDate::from_ymd_opt(today.year(), today.month(), 1).expect("valid first of month")
}

/// All stored months as stat rows, with NULL totals on current/future months
/// filled from `live` when given.
pub async fn load_stat_rows(
    pool: &SqlitePool,
    live: Option<&LiveTotals>,
) -> Result<Vec<MonthStatRow>, ApiError> {
    let stored = load_all_months(pool).await?;
    let items = load_all_items(pool).await?;
    let events = load_interest_events(pool).await?;
    to_stat_rows(&stored, live, &events, &items)
}

/// Stat rows for aggregate paths (yearly blocks, 回顧): live-fills NULL
/// totals on current/future months and returns the gated live tail for
/// `month_derived_with_tail`/`month_year_summaries`.
pub async fn load_stat_rows_live(
    pool: &SqlitePool,
) -> Result<(Vec<MonthStatRow>, Option<LiveTotals>), ApiError> {
    let stored = load_all_months(pool).await?;
    let items = load_all_items(pool).await?;
    let events = load_interest_events(pool).await?;
    let live = if wants_live(&stored) {
        Some(live_if_needed(pool).await?)
    } else {
        None
    };
    let stat_rows = to_stat_rows(&stored, live.as_ref(), &events, &items)?;
    let tail = live_tail(&stat_rows, live);
    Ok((stat_rows, tail))
}

fn to_stat_rows(
    stored: &[StoredMonth],
    live: Option<&LiveTotals>,
    events: &SuggestionEvents,
    items: &HashMap<chrono::NaiveDate, Vec<MonthItemFacts>>,
) -> Result<Vec<MonthStatRow>, ApiError> {
    let live_from = live_from();
    stored
        .iter()
        .map(|row| {
            let month = parse_stored_month(&row.month)?;
            let live = live.filter(|_| month >= live_from);
            let manual_interest = items
                .get(&month)
                .map(|items| month_item_sums(items).interest)
                .unwrap_or_default();
            Ok(MonthStatRow {
                month,
                start_cash: row.start_cash,
                salary: row.salary,
                total_assets: row.total_assets.or(live.map(|live| live.total_assets)),
                liquid_assets: row.liquid_assets.or(live.map(|live| live.liquid_assets)),
                end_cash_override: row.end_cash_override,
                interest: auto_interest(month, events) + manual_interest,
                pool_input: row.pool_input,
            })
        })
        .collect()
}

fn present(
    stored: &StoredMonth,
    derived: &MonthDerived,
    live: Option<&LiveTotals>,
    sums: &MonthItemSums,
    interest: f64,
) -> MonthStat {
    let live = live.filter(|_| {
        parse_stored_month(&stored.month)
            .map(|month| month >= live_from())
            .unwrap_or(false)
    });
    MonthStat {
        month: stored.month.clone(),
        start_cash: stored.start_cash,
        salary: stored.salary,
        total_assets: stored.total_assets.or(live.map(|live| live.total_assets)),
        liquid_assets: stored.liquid_assets.or(live.map(|live| live.liquid_assets)),
        total_assets_live: stored.total_assets.is_none() && live.is_some(),
        liquid_assets_live: stored.liquid_assets.is_none() && live.is_some(),
        interest,
        pool_input: stored.pool_input,
        end_cash_override: stored.end_cash_override,
        note: stored.note.clone(),
        created_at: stored.created_at.clone(),
        updated_at: stored.updated_at.clone(),
        end_cash: derived.end_cash,
        month_spend: derived.month_spend,
        living_spend: derived.living_spend,
        saved: derived.saved,
        total_change: derived.total_change,
        liquid_change: derived.liquid_change,
        living_yoy: derived.living_yoy,
        adjustment_sum: sums.adjustment,
        extra_spend_sum: sums.extra_spend,
        income_sum: sums.income,
        entertainment_sum: sums.entertainment,
    }
}

/// The components of `Overview!B1`/`H1`, resolved from the current DB — the
/// same aggregation the later Overview endpoint reuses.
pub async fn live_totals_input(pool: &SqlitePool) -> Result<LiveTotalsInput, ApiError> {
    let today = today();
    let hk = summary::market_value_only(pool, crate::models::Market::Hk)
        .await?
        .unwrap_or(0.0);
    let us = summary::market_value_only(pool, crate::models::Market::Us)
        .await?
        .unwrap_or(0.0);
    let rate = mpf::meta_f64(pool, aia::RATE_KEY).await?;

    let deposit_rows =
        sqlx::query("SELECT principal, interest, end_date FROM deposits ORDER BY sort_order, id")
            .fetch_all(pool)
            .await?;
    let mut facts = Vec::with_capacity(deposit_rows.len());
    for row in &deposit_rows {
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
    // 定期!B1 counts principal only; the interest lands in 活期 at maturity.
    let deposits_active_principal = active_totals(&facts, today).principal;

    let bonds_active_principal = bonds::active_principal(pool).await?;

    let policies = aia::load_all_policies(pool).await?;
    let aia_facts: Vec<crate::calc::AiaPolicyFacts> = policies.iter().map(aia::facts_of).collect();
    let aia_value_usd = aia_totals(&aia_facts, rate).value;

    let accounts = mpf::load_all_stored(pool).await?;
    let history = mpf::load_history(pool, None).await?;
    let mpf_facts: Vec<crate::calc::MpfAccountFacts> = accounts
        .iter()
        .map(|account| mpf::facts_of(account, &history))
        .collect();
    let mpf_balance = mpf_totals(&mpf_facts, today, None, None).now;

    let asset_rows = sqlx::query(
        "SELECT kind, COALESCE(SUM(amount), 0) AS total FROM manual_assets GROUP BY kind",
    )
    .fetch_all(pool)
    .await?;
    let mut manual_assets_sum = 0.0;
    let mut cash_sum = 0.0;
    for row in &asset_rows {
        let kind: String = row.try_get("kind")?;
        let total: f64 = row.try_get("total")?;
        match ManualAssetKind::parse(&kind) {
            Some(ManualAssetKind::Asset) => manual_assets_sum += total,
            Some(ManualAssetKind::Cash) => cash_sum += total,
            None => {
                return Err(ApiError::Conflict(format!(
                    "stored manual asset kind {kind} is not valid"
                )));
            }
        }
    }

    let months = load_all_months(pool).await?;
    let items = load_all_items(pool).await?;
    let interest_events = load_interest_events(pool).await?;
    let stat_rows = to_stat_rows(&months, None, &interest_events, &items)?;
    let rates = pool_rates(pool).await?;
    let pool_balance = match stat_rows.last() {
        Some(latest) => pool_balances(&stat_rows, &items, &rates)
            .get(&latest.month.year())
            .copied()
            .unwrap_or(0.0),
        None => 0.0,
    };

    Ok(LiveTotalsInput {
        hk_market_value: hk,
        us_market_value: us,
        usd_hkd_rate: rate,
        deposits_active_principal,
        bonds_active_principal,
        aia_value_usd,
        mpf_balance,
        manual_assets_sum,
        cash_sum,
        ibkr_hkd_cash: mpf::meta_f64(pool, overview::IBKR_HKD_CASH_KEY)
            .await?
            .unwrap_or(0.0),
        ibkr_usd_cash: mpf::meta_f64(pool, overview::IBKR_USD_CASH_KEY)
            .await?
            .unwrap_or(0.0),
        pool_balance,
    })
}

/// Compute the live totals once; callers check `wants_live` first.
async fn live_if_needed(pool: &SqlitePool) -> Result<LiveTotals, ApiError> {
    let input = live_totals_input(pool).await?;
    Ok(live_totals(&input))
}

fn needs_live(stored: &[StoredMonth]) -> bool {
    let live_from = live_from();
    stored.iter().any(|row| {
        (row.total_assets.is_none() || row.liquid_assets.is_none())
            && parse_stored_month(&row.month)
                .map(|month| month >= live_from)
                .unwrap_or(false)
    })
}

/// Whether live totals are worth computing: a NULL B/D at or after the
/// current month fills live, or the last stored month IS the current month
/// and needs the live tail for its Changed cell.
fn wants_live(stored: &[StoredMonth]) -> bool {
    needs_live(stored)
        || stored
            .last()
            .is_some_and(|row| parse_stored_month(&row.month).is_ok_and(|m| m == live_from()))
}

/// The live totals the last row's Changed cells diff against — the sheet's
/// last-row C/E cells read the live B1/H1, so the current month reports its
/// in-progress change. Gated to the current month: a future placeholder has
/// no knowable change and a stale last row must not pull a live diff into
/// an old year.
fn live_tail(rows: &[MonthStatRow], live: Option<LiveTotals>) -> Option<LiveTotals> {
    live.filter(|_| rows.last().is_some_and(|row| row.month == live_from()))
}

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
    let stored = load_all_months(&state.pool).await?;
    let items = load_all_items(&state.pool).await?;
    let interest_events = load_interest_events(&state.pool).await?;
    let live = if wants_live(&stored) {
        Some(live_if_needed(&state.pool).await?)
    } else {
        None
    };
    let stat_rows = to_stat_rows(&stored, live.as_ref(), &interest_events, &items)?;
    let derived = month_derived_with_tail(&stat_rows, &items, live_tail(&stat_rows, live));
    let months: Vec<MonthStat> = stored
        .iter()
        .zip(derived.iter())
        .enumerate()
        .map(|(index, (row, derived))| {
            let sums = items
                .get(&stat_rows[index].month)
                .map(|items| month_item_sums(items))
                .unwrap_or_default();
            present(
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
    let stored = load_all_months(&state.pool).await?;
    let items = load_all_items(&state.pool).await?;
    let interest_events = load_interest_events(&state.pool).await?;
    let rates = pool_rates(&state.pool).await?;
    // 投資純利 = Σ interest + the year's stored HK sold P/L (absent while a
    // year has none stored).
    let hk_sold_pl = super::yearly::hk_sold_pl(&state.pool).await?;
    let live = if wants_live(&stored) {
        Some(live_if_needed(&state.pool).await?)
    } else {
        None
    };
    let stat_rows = to_stat_rows(&stored, live.as_ref(), &interest_events, &items)?;
    let tail = live_tail(&stat_rows, live);
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

#[utoipa::path(
    get,
    path = "/api/months/settings",
    tag = "months",
    responses(
        (status = 200, description = "Salary and current-year pool rate", body = MonthSettings),
    )
)]
pub async fn settings(State(state): State<AppState>) -> Result<Json<MonthSettings>, ApiError> {
    let pool_rate_year = today().year();
    Ok(Json(MonthSettings {
        salary: mpf::meta_f64(&state.pool, SALARY_KEY).await?,
        pool_rate: mpf::meta_f64(&state.pool, &pool_rate_key(pool_rate_year)).await?,
        pool_rate_year,
        bill_amount: mpf::meta_f64(&state.pool, BILL_AMOUNT_KEY)
            .await?
            .unwrap_or(DEFAULT_BILL_AMOUNT),
        semi_liquid_target: semi_liquid_target(&state.pool).await?,
    }))
}

#[utoipa::path(
    patch,
    path = "/api/months/settings",
    tag = "months",
    request_body = MonthSettingsPatch,
    responses(
        (status = 200, description = "Updated settings", body = MonthSettings),
        (status = 400, description = "Missing or invalid fields", body = ErrorBody),
    )
)]
pub async fn update_settings(
    State(state): State<AppState>,
    Json(patch): Json<MonthSettingsPatch>,
) -> Result<Json<MonthSettings>, ApiError> {
    let mut errors = Vec::new();
    if let Some(Some(salary)) = patch.salary
        && (!salary.is_finite() || salary < 0.0)
    {
        errors.push(FieldError::new("salary", "salary must not be negative"));
    }
    if let Some(Some(rate)) = patch.pool_rate
        && (!rate.is_finite() || !(0.0..1.0).contains(&rate))
    {
        errors.push(FieldError::new(
            "pool_rate",
            "pool rate is stored as a fraction (0.337 = 33.7%) and must be less than 1",
        ));
    }
    if let Some(Some(amount)) = patch.bill_amount
        && (!amount.is_finite() || amount < 0.0)
    {
        errors.push(FieldError::new(
            "bill_amount",
            "bill amount must not be negative",
        ));
    }
    if let Some(Some(ratio)) = patch.semi_liquid_target
        && (!ratio.is_finite() || !(0.0..1.0).contains(&ratio))
    {
        errors.push(FieldError::new(
            "semi_liquid_target",
            "semi-liquid target is stored as a fraction (0.25 = 25%) and must be less than 1",
        ));
    }
    if !errors.is_empty() {
        return Err(ApiError::Validation(errors));
    }

    if let Some(salary) = patch.salary {
        crate::mpf::meta_put(
            &state.pool,
            SALARY_KEY,
            salary.map(|salary| salary.to_string()).as_deref(),
        )
        .await?;
    }
    if let Some(rate) = patch.pool_rate {
        let year = patch.pool_rate_year.unwrap_or_else(|| today().year());
        crate::mpf::meta_put(
            &state.pool,
            &pool_rate_key(year),
            rate.map(|rate| rate.to_string()).as_deref(),
        )
        .await?;
    }
    if let Some(amount) = patch.bill_amount {
        crate::mpf::meta_put(
            &state.pool,
            BILL_AMOUNT_KEY,
            amount.map(|amount| amount.to_string()).as_deref(),
        )
        .await?;
    }
    if let Some(ratio) = patch.semi_liquid_target {
        crate::mpf::meta_put(
            &state.pool,
            SEMI_LIQUID_TARGET_KEY,
            ratio.map(|ratio| ratio.to_string()).as_deref(),
        )
        .await?;
    }
    settings(State(state)).await
}

fn parse_event_day(date: &str) -> Result<chrono::NaiveDate, ApiError> {
    chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|_| ApiError::Conflict(format!("stored date {date} is not valid")))
}

/// Deposits whose start or end date falls in `range` (`[first, next)`); `None`
/// loads them all — the derived 利息 needs every month at once.
async fn load_deposit_events(
    pool: &SqlitePool,
    range: Option<(String, String)>,
) -> Result<Vec<SuggestionDeposit>, ApiError> {
    let rows = match &range {
        Some((first, next)) => {
            sqlx::query(
                "SELECT id, label, start_date, end_date, principal, interest, received_at \
                 FROM deposits \
             WHERE (start_date IS NOT NULL AND start_date >= ? AND start_date < ?) \
             OR (end_date >= ? AND end_date < ?)",
            )
            .bind(first)
            .bind(next)
            .bind(first)
            .bind(next)
            .fetch_all(pool)
            .await?
        }
        None => {
            sqlx::query(
                "SELECT id, label, start_date, end_date, principal, interest, received_at \
                 FROM deposits",
            )
            .fetch_all(pool)
            .await?
        }
    };
    rows.iter()
        .map(|row| {
            let start_date: Option<String> = row.try_get("start_date")?;
            let received_at: Option<String> = row.try_get("received_at")?;
            Ok(SuggestionDeposit {
                id: row.try_get("id")?,
                start_date: start_date.map(|date| parse_event_day(&date)).transpose()?,
                end_date: parse_event_day(&row.try_get::<String, _>("end_date")?)?,
                principal: row.try_get::<Option<f64>, _>("principal")?.unwrap_or(0.0),
                interest: row.try_get::<Option<f64>, _>("interest")?.unwrap_or(0.0),
                label: row.try_get("label")?,
                received: received_at.is_some(),
            })
        })
        .collect()
}

/// Received HK dividends paid in `range`, or all of them when `None`.
async fn load_dividend_events(
    pool: &SqlitePool,
    range: Option<(String, String)>,
) -> Result<Vec<SuggestionReceipt>, ApiError> {
    let mut sql = "SELECT d.id, d.pay_date, d.received_amount, d.estimated_amount, s.code \
         FROM dividends d JOIN stocks s ON s.id = d.stock_id WHERE s.market = 'HK'"
        .to_string();
    if range.is_some() {
        sql.push_str(" AND d.pay_date >= ? AND d.pay_date < ?");
    }
    let mut query = sqlx::query(sqlx::AssertSqlSafe(sql));
    if let Some((first, next)) = &range {
        query = query.bind(first).bind(next);
    }
    let rows = query.fetch_all(pool).await?;
    rows.iter()
        .map(|row| {
            let received_amount: Option<f64> = row.try_get("received_amount")?;
            let estimated_amount: Option<f64> = row.try_get("estimated_amount")?;
            Ok(SuggestionReceipt {
                id: row.try_get("id")?,
                pay_date: parse_event_day(&row.try_get::<String, _>("pay_date")?)?,
                amount: received_amount.or(estimated_amount),
                label: row.try_get("code")?,
                received: received_amount.is_some(),
            })
        })
        .collect()
}

/// Bond coupons paid in `range`, or all of them when `None` — received ones
/// carry their amount, pending ones the expected figure (None while 待定).
async fn load_coupon_events(
    pool: &SqlitePool,
    range: Option<(String, String)>,
) -> Result<Vec<SuggestionReceipt>, ApiError> {
    let mut sql = "SELECT c.id, c.pay_date, c.received_amount, c.per_10k, b.principal, b.label \
         FROM bond_coupons c JOIN bonds b ON b.id = c.bond_id"
        .to_string();
    if range.is_some() {
        sql.push_str(" WHERE c.pay_date >= ? AND c.pay_date < ?");
    }
    let mut query = sqlx::query(sqlx::AssertSqlSafe(sql));
    if let Some((first, next)) = &range {
        query = query.bind(first).bind(next);
    }
    let rows = query.fetch_all(pool).await?;
    rows.iter()
        .map(|row| {
            let received_amount: Option<f64> = row.try_get("received_amount")?;
            let per_10k: Option<f64> = row.try_get("per_10k")?;
            let principal: f64 = row.try_get("principal")?;
            Ok(SuggestionReceipt {
                id: row.try_get("id")?,
                pay_date: parse_event_day(&row.try_get::<String, _>("pay_date")?)?,
                amount: received_amount.or(crate::calc::coupon_expected(per_10k, principal)),
                label: row.try_get("label")?,
                received: received_amount.is_some(),
            })
        })
        .collect()
}

/// The interest-relevant events across every month: all deposits (interest by
/// `end_date`), all received coupons, all received HK dividends. The other
/// `SuggestionEvents` fields stay empty — `interest_components` reads only
/// these three.
pub(crate) async fn load_interest_events(pool: &SqlitePool) -> Result<SuggestionEvents, ApiError> {
    Ok(SuggestionEvents {
        deposits: load_deposit_events(pool, None).await?,
        trades: Vec::new(),
        dividends: load_dividend_events(pool, None).await?,
        coupons: load_coupon_events(pool, None).await?,
        aia_payments: Vec::new(),
        pool_input: 0.0,
        usd_hkd_rate: None,
    })
}

/// The suggestion sources for one month, loaded fresh on each read.
async fn load_events(
    pool: &SqlitePool,
    month: chrono::NaiveDate,
    pool_input: f64,
) -> Result<SuggestionEvents, ApiError> {
    let range = Some((
        month.to_string(),
        (month + chrono::Months::new(1)).to_string(),
    ));
    let (first, next) = range.clone().expect("just built");

    let trade_rows = sqlx::query(
        "SELECT t.id, t.trade_date, t.trade_type, t.total, s.code FROM trades t \
         JOIN stocks s ON s.id = t.stock_id \
         WHERE s.market = 'HK' AND t.trade_date >= ? AND t.trade_date < ?",
    )
    .bind(&first)
    .bind(&next)
    .fetch_all(pool)
    .await?;
    let mut trades = Vec::with_capacity(trade_rows.len());
    for row in &trade_rows {
        let trade_type: String = row.try_get("trade_type")?;
        trades.push(SuggestionTrade {
            id: row.try_get("id")?,
            date: parse_event_day(&row.try_get::<String, _>("trade_date")?)?,
            kind: TradeType::parse(&trade_type).ok_or_else(|| {
                ApiError::Conflict(format!("stored trade type {trade_type} is not valid"))
            })?,
            total: row.try_get("total")?,
            code: row.try_get("code")?,
        });
    }

    let aia_rows = sqlx::query(
        "SELECT e.id, e.event_date, e.amount_usd, p.label FROM aia_events e \
         JOIN aia_policies p ON p.id = e.policy_id \
         WHERE e.kind = 'payment' AND e.event_date >= ? AND e.event_date < ?",
    )
    .bind(&first)
    .bind(&next)
    .fetch_all(pool)
    .await?;
    let mut aia_payments = Vec::with_capacity(aia_rows.len());
    for row in &aia_rows {
        aia_payments.push(SuggestionAiaPayment {
            id: row.try_get("id")?,
            date: parse_event_day(&row.try_get::<String, _>("event_date")?)?,
            amount_usd: row.try_get("amount_usd")?,
            policy: row.try_get("label")?,
        });
    }

    Ok(SuggestionEvents {
        deposits: load_deposit_events(pool, range.clone()).await?,
        trades,
        dividends: load_dividend_events(pool, range.clone()).await?,
        coupons: load_coupon_events(pool, range).await?,
        aia_payments,
        pool_input,
        usd_hkd_rate: mpf::meta_f64(pool, aia::RATE_KEY).await?,
    })
}

async fn load_items(pool: &SqlitePool, month: &str) -> Result<Vec<MonthItem>, ApiError> {
    let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
        "SELECT {ITEM_COLUMNS} FROM month_items WHERE month = ? ORDER BY id"
    )))
    .bind(month)
    .fetch_all(pool)
    .await?;
    rows.iter().map(row_to_item).collect()
}

async fn dismissed_keys(pool: &SqlitePool, month: &str) -> Result<HashSet<String>, ApiError> {
    let keys = sqlx::query_scalar::<_, String>(
        "SELECT auto_key FROM month_item_dismissals WHERE month = ?",
    )
    .bind(month)
    .fetch_all(pool)
    .await?;
    Ok(keys.into_iter().collect())
}

/// Load one row plus its derived columns and live-resolved totals.
async fn present_month(pool: &SqlitePool, stored: &StoredMonth) -> Result<MonthStat, ApiError> {
    let all = load_all_months(pool).await?;
    let items = load_all_items(pool).await?;
    let interest_events = load_interest_events(pool).await?;
    let live = if wants_live(&all) {
        Some(live_if_needed(pool).await?)
    } else {
        None
    };
    let stat_rows = to_stat_rows(&all, live.as_ref(), &interest_events, &items)?;
    let derived = month_derived_with_tail(&stat_rows, &items, live_tail(&stat_rows, live));
    let index = all
        .iter()
        .position(|row| row.month == stored.month)
        .ok_or_else(|| ApiError::NotFound(format!("month {} not found", stored.month)))?;
    let sums = items
        .get(&stat_rows[index].month)
        .map(|items| month_item_sums(items))
        .unwrap_or_default();
    Ok(present(
        &all[index],
        &derived[index],
        live.as_ref(),
        &sums,
        stat_rows[index].interest,
    ))
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
    let stored = load_month(&state.pool, &month_key)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("month {month_key} not found")))?;

    let items = load_items(&state.pool, &month_key).await?;
    let events = load_events(&state.pool, month, stored.pool_input).await?;
    let suggestions = if suggestions_enabled(&month_key, today()) {
        let stored_keys: HashSet<String> = items
            .iter()
            .filter_map(|item| item.auto_key.clone())
            .collect();
        let dismissed = dismissed_keys(&state.pool, &month_key).await?;
        build_suggestions(&month_key, &events, &stored_keys, &dismissed)
    } else {
        Vec::new()
    };

    Ok(Json(MonthDetailResponse {
        month: present_month(&state.pool, &stored).await?,
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

    let existing = load_month(&state.pool, &month_key).await?;
    let now = now_timestamp();

    match existing {
        None => {
            // Body-supplied totals win; otherwise snapshot the live values.
            let live = live_totals(&live_totals_input(&state.pool).await?);
            let total_assets = patch.total_assets.unwrap_or(Some(live.total_assets));
            let liquid_assets = patch.liquid_assets.unwrap_or(Some(live.liquid_assets));
            let salary = match patch.salary {
                Some(salary) => Some(salary),
                None => mpf::meta_f64(&state.pool, SALARY_KEY).await?,
            };
            sqlx::query(
                "INSERT INTO month_stats (month, start_cash, salary, total_assets, \
                 liquid_assets, pool_input, \
                 end_cash_override, note, created_at, updated_at) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&month_key)
            .bind(patch.start_cash)
            .bind(salary)
            .bind(total_assets)
            .bind(liquid_assets)
            .bind(patch.pool_input.unwrap_or(0.0))
            .bind(patch.end_cash_override.flatten())
            .bind(patch.note.flatten())
            .bind(&now)
            .bind(&now)
            .execute(&state.pool)
            .await?;
        }
        Some(existing) => {
            let merge = |patch: Option<f64>, existing: Option<f64>| patch.or(existing);
            let merge_num = |patch: Option<f64>, existing: f64| patch.unwrap_or(existing);
            let merge_text = |patch: Option<Option<String>>, existing: Option<String>| match patch {
                Some(value) => value
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty()),
                None => existing,
            };
            // A present field wins over recapture, recapture wins over the
            // stored value, and `null` stores NULL (live).
            let input = if patch.recapture == Some(true) {
                Some(live_totals_input(&state.pool).await?)
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
            sqlx::query(
                "UPDATE month_stats SET start_cash = ?, salary = ?, total_assets = ?, \
                 liquid_assets = ?, pool_input = ?, \
                 end_cash_override = ?, note = ?, updated_at = ? WHERE month = ?",
            )
            .bind(start_cash)
            .bind(merge(patch.salary, existing.salary))
            .bind(total_assets)
            .bind(liquid_assets)
            .bind(merge_num(patch.pool_input, existing.pool_input))
            .bind(
                patch
                    .end_cash_override
                    .unwrap_or(existing.end_cash_override),
            )
            .bind(merge_text(patch.note, existing.note))
            .bind(&now)
            .bind(&month_key)
            .execute(&state.pool)
            .await?;
        }
    }

    let stored = load_month(&state.pool, &month_key)
        .await?
        .expect("just written");
    Ok(Json(present_month(&state.pool, &stored).await?))
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
    let result = sqlx::query("DELETE FROM month_stats WHERE month = ?")
        .bind(&month_key)
        .execute(&state.pool)
        .await?;
    if result.rows_affected() == 0 {
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
    if load_month(&state.pool, &month_key).await?.is_none() {
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
        sqlx::query("DELETE FROM month_item_dismissals WHERE month = ? AND auto_key = ?")
            .bind(&month_key)
            .bind(auto_key)
            .execute(&mut *tx)
            .await?;
    }
    // A duplicate (month, auto_key) hits the partial unique index → 409.
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO month_items (month, category, label, amount, exclude_from_living, \
         auto_key, note, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(&month_key)
    .bind(body.category.as_str())
    .bind(&label)
    .bind(body.amount)
    .bind(body.exclude_from_living as i64)
    .bind(&auto_key)
    .bind(&note)
    .bind(now_timestamp())
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;

    let row = sqlx::query(sqlx::AssertSqlSafe(format!(
        "SELECT {ITEM_COLUMNS} FROM month_items WHERE id = ?"
    )))
    .bind(id)
    .fetch_one(&state.pool)
    .await?;
    Ok((StatusCode::CREATED, Json(row_to_item(&row)?)))
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
    let row = sqlx::query(sqlx::AssertSqlSafe(format!(
        "SELECT {ITEM_COLUMNS} FROM month_items WHERE id = ?"
    )))
    .bind(id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::NotFound(format!("month item {id} not found")))?;
    let existing = row_to_item(&row)?;

    let merge_text = |patch: Option<Option<String>>, existing: Option<String>| match patch {
        Some(value) => value
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        None => existing,
    };
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

    sqlx::query(
        "UPDATE month_items SET category = ?, label = ?, amount = ?, \
         exclude_from_living = ?, note = ? WHERE id = ?",
    )
    .bind(category.as_str())
    .bind(&label)
    .bind(amount)
    .bind(exclude_from_living as i64)
    .bind(&note)
    .bind(id)
    .execute(&state.pool)
    .await?;

    let row = sqlx::query(sqlx::AssertSqlSafe(format!(
        "SELECT {ITEM_COLUMNS} FROM month_items WHERE id = ?"
    )))
    .bind(id)
    .fetch_one(&state.pool)
    .await?;
    Ok(Json(row_to_item(&row)?))
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
    let result = sqlx::query("DELETE FROM month_items WHERE id = ?")
        .bind(id)
        .execute(&state.pool)
        .await?;
    if result.rows_affected() == 0 {
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
    sqlx::query(
        "INSERT OR IGNORE INTO month_item_dismissals (month, auto_key, created_at) \
         VALUES (?, ?, ?)",
    )
    .bind(&month_key)
    .bind(auto_key)
    .bind(now_timestamp())
    .execute(&state.pool)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

fn row_to_asset(row: &SqliteRow) -> Result<ManualAsset, ApiError> {
    let kind: String = row.try_get("kind")?;
    let liquidity: String = row.try_get("liquidity")?;
    Ok(ManualAsset {
        id: row.try_get("id")?,
        label: row.try_get("label")?,
        kind: ManualAssetKind::parse(&kind)
            .ok_or_else(|| ApiError::Conflict(format!("stored kind {kind} is not valid")))?,
        liquidity: ManualAssetLiquidity::parse(&liquidity).ok_or_else(|| {
            ApiError::Conflict(format!("stored liquidity {liquidity} is not valid"))
        })?,
        amount: row.try_get("amount")?,
        sort_order: row.try_get("sort_order")?,
        updated_at: row.try_get("updated_at")?,
    })
}

const ASSET_COLUMNS: &str = "id, label, kind, liquidity, amount, sort_order, updated_at";

async fn load_asset(pool: &SqlitePool, id: i64) -> Result<ManualAsset, ApiError> {
    let row = sqlx::query(sqlx::AssertSqlSafe(format!(
        "SELECT {ASSET_COLUMNS} FROM manual_assets WHERE id = ?"
    )))
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| ApiError::NotFound(format!("manual asset {id} not found")))?;
    row_to_asset(&row)
}

pub async fn load_assets(pool: &SqlitePool) -> Result<Vec<ManualAsset>, ApiError> {
    let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
        "SELECT {ASSET_COLUMNS} FROM manual_assets ORDER BY kind, sort_order, id"
    )))
    .fetch_all(pool)
    .await?;
    rows.iter().map(row_to_asset).collect::<Result<Vec<_>, _>>()
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
    load_assets(&state.pool).await.map(Json)
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
    let sort_order: i64 =
        sqlx::query_scalar("SELECT COALESCE(MAX(sort_order), 0) + 1 FROM manual_assets")
            .fetch_one(&state.pool)
            .await?;
    let liquidity = body.liquidity.unwrap_or_default();
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO manual_assets (label, kind, liquidity, amount, sort_order, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(&label)
    .bind(body.kind.as_str())
    .bind(liquidity.as_str())
    .bind(body.amount)
    .bind(sort_order)
    .bind(now_timestamp())
    .fetch_one(&state.pool)
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(load_asset(&state.pool, id).await?),
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
    let existing = load_asset(&state.pool, id).await?;
    let label = patch.label.unwrap_or(existing.label);
    let kind = patch.kind.unwrap_or(existing.kind);
    let liquidity = patch.liquidity.unwrap_or(existing.liquidity);
    let amount = patch.amount.unwrap_or(existing.amount);
    let label = validate_asset(&label, amount)?;
    sqlx::query(
        "UPDATE manual_assets SET label = ?, kind = ?, liquidity = ?, amount = ?, updated_at = ? \
         WHERE id = ?",
    )
    .bind(&label)
    .bind(kind.as_str())
    .bind(liquidity.as_str())
    .bind(amount)
    .bind(now_timestamp())
    .bind(id)
    .execute(&state.pool)
    .await?;
    Ok(Json(load_asset(&state.pool, id).await?))
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
    let result = sqlx::query("DELETE FROM manual_assets WHERE id = ?")
        .bind(id)
        .execute(&state.pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound(format!("manual asset {id} not found")));
    }
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connect_memory;

    #[tokio::test]
    async fn create_asset_defaults_liquidity_to_long() {
        let pool = connect_memory().await.expect("memory db");
        let created = create_asset(
            State(AppState { pool: pool.clone() }),
            Json(NewManualAsset {
                label: "HS人壽".to_string(),
                kind: ManualAssetKind::Asset,
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
                kind: ManualAssetKind::Asset,
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

        let stored = load_asset(&pool, created.id).await.expect("stored");
        assert_eq!(stored.liquidity, ManualAssetLiquidity::Short);
    }

    #[tokio::test]
    async fn update_settings_round_trips_the_semi_liquid_target() {
        let pool = connect_memory().await.expect("memory db");
        let state = AppState { pool: pool.clone() };

        // While unset the effective ratio is the sheet's 25% literal.
        let current = settings(State(state.clone())).await.expect("settings").0;
        assert_eq!(current.semi_liquid_target, 0.25);

        let updated = update_settings(
            State(state.clone()),
            Json(MonthSettingsPatch {
                semi_liquid_target: Some(Some(0.3)),
                ..Default::default()
            }),
        )
        .await
        .expect("patched")
        .0;
        assert_eq!(updated.semi_liquid_target, 0.3);
        assert_eq!(semi_liquid_target(&pool).await.unwrap(), 0.3);

        // `null` clears the stored value — back to the default.
        let reset = update_settings(
            State(state.clone()),
            Json(MonthSettingsPatch {
                semi_liquid_target: Some(None),
                ..Default::default()
            }),
        )
        .await
        .expect("reset")
        .0;
        assert_eq!(reset.semi_liquid_target, 0.25);

        // A share of 100%+ is meaningless for the buffer and rejected.
        let err = update_settings(
            State(state),
            Json(MonthSettingsPatch {
                semi_liquid_target: Some(Some(1.5)),
                ..Default::default()
            }),
        )
        .await
        .expect_err("rejected");
        assert!(matches!(err, ApiError::Validation(_)));
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
