use std::collections::HashMap;

use chrono::Datelike;
use sqlx::SqlitePool;

use crate::calc::{
    LiveTotals, LiveTotalsInput, MonthDerived, MonthItemFacts, MonthItemSums, SuggestionAiaPayment,
    SuggestionDeposit, SuggestionEvents, SuggestionReceipt, SuggestionTrade, active_totals,
    aia_totals, auto_interest, live_totals, month_item_sums, mpf_totals, pool_balances,
};
use crate::models::MonthStat;
use crate::store::months::StoredMonth;
use crate::store::{StoreError, today};

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
) -> Result<Vec<crate::calc::MonthStatRow>, StoreError> {
    let stored = crate::store::months::load_all_months(pool).await?;
    let items = crate::store::months::load_all_items(pool).await?;
    let events = load_interest_events(pool).await?;
    to_stat_rows(&stored, live, &events, &items)
}

/// Stat rows for aggregate paths (yearly blocks, 回顧): live-fills NULL
/// totals on current/future months and returns the gated live tail for
/// `month_derived_with_tail`/`month_year_summaries`.
pub async fn load_stat_rows_live(
    pool: &SqlitePool,
) -> Result<(Vec<crate::calc::MonthStatRow>, Option<LiveTotals>), StoreError> {
    let stored = crate::store::months::load_all_months(pool).await?;
    let items = crate::store::months::load_all_items(pool).await?;
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

pub fn to_stat_rows(
    stored: &[StoredMonth],
    live: Option<&LiveTotals>,
    events: &SuggestionEvents,
    items: &HashMap<chrono::NaiveDate, Vec<MonthItemFacts>>,
) -> Result<Vec<crate::calc::MonthStatRow>, StoreError> {
    let live_from = live_from();
    stored
        .iter()
        .map(|row| {
            let month = crate::store::months::parse_stored_month(&row.month)?;
            let live = live.filter(|_| month >= live_from);
            let manual_interest = items
                .get(&month)
                .map(|items| month_item_sums(items).interest)
                .unwrap_or_default();
            Ok(crate::calc::MonthStatRow {
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

pub fn present(
    stored: &StoredMonth,
    derived: &MonthDerived,
    live: Option<&LiveTotals>,
    sums: &MonthItemSums,
    interest: f64,
) -> MonthStat {
    let live = live.filter(|_| {
        crate::store::months::parse_stored_month(&stored.month)
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
pub async fn live_totals_input(pool: &SqlitePool) -> Result<LiveTotalsInput, StoreError> {
    let today = today();
    let hk = super::summary::market_value_only(pool, crate::models::Market::Hk)
        .await?
        .unwrap_or(0.0);
    let us = super::summary::market_value_only(pool, crate::models::Market::Us)
        .await?
        .unwrap_or(0.0);
    let rate = crate::store::fx::usd_rate(pool).await?;

    let facts = crate::store::deposits::active_facts(pool).await?;
    // 定期!B1 counts principal only; the interest lands in 活期 at maturity.
    let deposits_active_principal = active_totals(&facts).principal;

    let bonds_active_principal = super::bonds::active_principal(pool).await?;

    let policies = crate::store::aia::load_all_policies(pool).await?;
    let aia_facts: Vec<crate::calc::AiaPolicyFacts> =
        policies.iter().map(crate::store::aia::facts_of).collect();
    let aia_value_usd = aia_totals(&aia_facts, rate).value;

    let accounts = crate::store::mpf::load_all_stored(pool).await?;
    let history = crate::store::mpf::load_history(pool, None).await?;
    let mpf_facts: Vec<crate::calc::MpfAccountFacts> = accounts
        .iter()
        .map(|account| crate::store::mpf::facts_of(account, &history))
        .collect();
    let mpf_balance = mpf_totals(&mpf_facts, today, None, None).now;

    let manual_assets_sum = crate::store::months::manual_asset_sum(pool).await?;
    let cash_sum = crate::store::cash_accounts::bank_sum(pool).await?;

    let months = crate::store::months::load_all_months(pool).await?;
    let items = crate::store::months::load_all_items(pool).await?;
    let interest_events = load_interest_events(pool).await?;
    let stat_rows = to_stat_rows(&months, None, &interest_events, &items)?;
    let rates = crate::store::months::pool_rates(pool).await?;
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
        ibkr_hkd_cash: crate::store::cash_accounts::broker(pool, crate::models::Currency::Hkd)
            .await?
            .map(|account| account.amount)
            .unwrap_or(0.0),
        ibkr_usd_cash: crate::store::cash_accounts::broker(pool, crate::models::Currency::Usd)
            .await?
            .map(|account| account.amount)
            .unwrap_or(0.0),
        pool_balance,
    })
}

/// Compute the live totals once; callers check `wants_live` first.
pub async fn live_if_needed(pool: &SqlitePool) -> Result<LiveTotals, StoreError> {
    let input = live_totals_input(pool).await?;
    Ok(live_totals(&input))
}

/// `live_totals_input` computes live totals unconditionally — the
/// `if needs_live { ... }` gate lives in `load_stat_rows_live` and the
/// handlers that want them.
pub fn needs_live(stored: &[StoredMonth]) -> bool {
    let live_from = live_from();
    stored.iter().any(|row| {
        (row.total_assets.is_none() || row.liquid_assets.is_none())
            && crate::store::months::parse_stored_month(&row.month)
                .map(|month| month >= live_from)
                .unwrap_or(false)
    })
}

/// Whether live totals are worth computing: a NULL B/D at or after the
/// current month fills live, or the last stored month IS the current month
/// and needs the live tail for its Changed cell.
pub fn wants_live(stored: &[StoredMonth]) -> bool {
    needs_live(stored)
        || stored.last().is_some_and(|row| {
            crate::store::months::parse_stored_month(&row.month).is_ok_and(|m| m == live_from())
        })
}

/// The live totals the last row's Changed cells diff against — the sheet's
/// last-row C/E cells read the live B1/H1, so the current month reports its
/// in-progress change. Gated to the current month: a future placeholder has
/// no knowable change and a stale last row must not pull a live diff into
/// an old year.
pub fn live_tail(
    rows: &[crate::calc::MonthStatRow],
    live: Option<LiveTotals>,
) -> Option<LiveTotals> {
    live.filter(|_| rows.last().is_some_and(|row| row.month == live_from()))
}

/// Deposits whose start or end date falls in `range` (`[first, next)`); `None`
/// loads them all — the derived 利息 needs every month at once.
pub async fn load_deposit_events(
    pool: &SqlitePool,
    range: Option<(String, String)>,
) -> Result<Vec<SuggestionDeposit>, StoreError> {
    crate::store::deposits::suggestion_events(pool, range).await
}

/// Received HK dividends paid in `range`, or all of them when `None`.
pub async fn load_dividend_events(
    pool: &SqlitePool,
    range: Option<(String, String)>,
) -> Result<Vec<SuggestionReceipt>, StoreError> {
    crate::store::dividends::suggestion_receipts(pool, range).await
}

/// Bond coupons paid in `range`, or all of them when `None` — received ones
/// carry their amount, pending ones the expected figure (None while 待定).
pub async fn load_coupon_events(
    pool: &SqlitePool,
    range: Option<(String, String)>,
) -> Result<Vec<SuggestionReceipt>, StoreError> {
    crate::store::bonds::suggestion_receipts(pool, range).await
}

/// The interest-relevant events across every month: all deposits (interest by
/// `end_date`), all received coupons, all received HK dividends. The other
/// `SuggestionEvents` fields stay empty — `interest_components` reads only
/// these three.
pub async fn load_interest_events(pool: &SqlitePool) -> Result<SuggestionEvents, StoreError> {
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
pub async fn load_events(
    pool: &SqlitePool,
    month: chrono::NaiveDate,
    pool_input: f64,
) -> Result<SuggestionEvents, StoreError> {
    let range = Some((
        month.to_string(),
        (month + chrono::Months::new(1)).to_string(),
    ));
    let (first, next) = range.clone().expect("just built");

    let trades: Vec<SuggestionTrade> =
        crate::store::trades::hk_suggestion_trades(pool, &first, &next).await?;
    let aia_payments: Vec<SuggestionAiaPayment> =
        crate::store::aia::suggestion_payments(pool, &first, &next).await?;

    Ok(SuggestionEvents {
        deposits: load_deposit_events(pool, range.clone()).await?,
        trades,
        dividends: load_dividend_events(pool, range.clone()).await?,
        coupons: load_coupon_events(pool, range).await?,
        aia_payments,
        pool_input,
        usd_hkd_rate: crate::store::fx::usd_rate(pool).await?,
    })
}

/// Load one row plus its derived columns and live-resolved totals.
pub async fn present_month(
    pool: &SqlitePool,
    stored: &StoredMonth,
) -> Result<MonthStat, StoreError> {
    let all = crate::store::months::load_all_months(pool).await?;
    let items = crate::store::months::load_all_items(pool).await?;
    let interest_events = load_interest_events(pool).await?;
    let live = if wants_live(&all) {
        Some(live_if_needed(pool).await?)
    } else {
        None
    };
    let stat_rows = to_stat_rows(&all, live.as_ref(), &interest_events, &items)?;
    let derived =
        crate::calc::month_derived_with_tail(&stat_rows, &items, live_tail(&stat_rows, live));
    let index = all
        .iter()
        .position(|row| row.month == stored.month)
        .ok_or_else(|| StoreError::NotFound(format!("month {} not found", stored.month)))?;
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

/// `goal3.inflation` / `goal3.years` / `final.replacement` — the sheet
/// literals while no `app_meta` override exists.
pub const DEFAULT_GOAL3_INFLATION: f64 = 1.07;
pub const DEFAULT_GOAL3_YEARS: f64 = 30.0;
pub const DEFAULT_FINAL_REPLACEMENT: f64 = 0.7;
/// The semi-liquid target ratio's sheet literal while no `app_meta` override
/// exists.
pub const DEFAULT_SEMI_LIQUID_TARGET: f64 = 0.25;

/// The effective 3rd-target/Final-Target constants — the sheet literals
/// (`1.07`, `30`, `0.7`) while unset.
pub async fn goal_constants(pool: &SqlitePool) -> Result<(f64, f64, f64), StoreError> {
    let stored = crate::store::settings::Settings::load(pool).await?;
    Ok((
        stored.goal3_inflation.unwrap_or(DEFAULT_GOAL3_INFLATION),
        stored.goal3_years.unwrap_or(DEFAULT_GOAL3_YEARS),
        stored
            .final_replacement
            .unwrap_or(DEFAULT_FINAL_REPLACEMENT),
    ))
}

/// The configured 半流動資金 target share of 流動資產 — `0.25` while unset.
pub async fn semi_liquid_target(pool: &SqlitePool) -> Result<f64, StoreError> {
    Ok(crate::store::settings::Settings::load(pool)
        .await?
        .semi_liquid_target
        .unwrap_or(DEFAULT_SEMI_LIQUID_TARGET))
}
