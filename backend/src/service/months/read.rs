use std::collections::HashSet;

use chrono::Datelike;

use serde::Serialize;
use sqlx::SqlitePool;

use crate::calc::{
    MonthRunningAverages, MonthYearSummary, PoolTotals, build_suggestions, interest_components,
    pool_totals, suggestion_actions_enabled,
};
use crate::models::{InterestComponent, MonthItem, MonthStat, MonthSuggestion};
use crate::store::StoreError;
use crate::store::today;

use super::book::{MonthBook, load_events};

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

/// `GET /api/months`: the book's rows, optionally restricted to one year.
pub async fn list(pool: &SqlitePool, year: Option<i32>) -> Result<Vec<MonthStat>, StoreError> {
    Ok(MonthBook::open(pool)
        .await?
        .months()
        .into_iter()
        .filter(|month| year.is_none_or(|year| month.month.starts_with(&format!("{year:04}-"))))
        .collect())
}

/// `GET /api/months/summary`: year summaries, running averages, and the
/// 開心Pool figures for the current year.
pub async fn summary(pool: &SqlitePool) -> Result<MonthSummaryResponse, StoreError> {
    let book = MonthBook::open(pool).await?;
    // 投資純利 = Σ interest + the year's stored HK sold P/L (absent while a
    // year has none stored).
    let years = book.year_summaries();
    let pool_rate_year = today().year();
    let pool_totals = pool_totals(&years);
    Ok(MonthSummaryResponse {
        years,
        running: book.running_averages(),
        pool_balance: book.pool_balance(),
        pool_totals,
        pool_rate: book.pool_rate(pool_rate_year),
        pool_rate_year,
    })
}

/// `GET /api/months/{ym}`: the month's derived columns, items, candidates and
/// 利息 breakdown. `Err(NotFound)` while `month` names no stored row.
pub async fn detail(
    pool: &SqlitePool,
    month: chrono::NaiveDate,
) -> Result<MonthDetailResponse, StoreError> {
    let month_key = month.to_string();
    let stat = MonthBook::open(pool)
        .await?
        .month(&month_key)
        .ok_or_else(|| StoreError::NotFound(format!("month {month_key} not found")))?;

    let items = crate::store::months::load_items(pool, &month_key).await?;
    let events = load_events(pool, month).await?;
    let stored_keys: HashSet<String> = items
        .iter()
        .filter_map(|item| item.auto_key.clone())
        .collect();
    let dismissed = crate::store::months::dismissed_keys(pool, &month_key).await?;
    let suggestions = build_suggestions(&month_key, &events, &stored_keys, &dismissed);

    Ok(MonthDetailResponse {
        month: stat,
        items,
        suggestions,
        suggestions_actionable: suggestion_actions_enabled(&month_key, today()),
        interest_auto: interest_components(&month_key, &events),
    })
}
