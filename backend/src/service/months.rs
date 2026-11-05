use std::collections::HashMap;

use chrono::Datelike;
use sqlx::SqlitePool;

use crate::calc::{
    LiveTotals, MonthDerived, MonthItemFacts, MonthItemSums, SuggestionAiaPayment,
    SuggestionDeposit, SuggestionEvents, SuggestionReceipt, SuggestionTrade, auto_interest,
    month_item_sums,
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

/// The assembled 月結 ledger: every month-derived read opens the book rather
/// than re-sequencing the load/live-gate/derive pipeline itself.
pub struct MonthBook {
    /// The stored `month_stats` rows, in `month` order.
    pub stored: Vec<StoredMonth>,
    /// One [`crate::calc::MonthStatRow`] per stored row, NULL totals on
    /// current/future months filled from `live` when held.
    pub stat_rows: Vec<crate::calc::MonthStatRow>,
    /// All `month_items`, grouped by month.
    pub items: HashMap<chrono::NaiveDate, Vec<MonthItemFacts>>,
    /// The gated live totals — `Some` only when a stored NULL or the
    /// current-month tail needs them.
    pub live: Option<LiveTotals>,
    /// The live totals the last row's Changed cells diff against — `Some`
    /// only when the last stored month IS the current month.
    pub tail: Option<LiveTotals>,
}

impl MonthBook {
    /// Open the book, computing live totals only when `wants_live` — a
    /// current/future NULL B/D to fill, or a current last row needing the tail.
    pub async fn open(pool: &SqlitePool) -> Result<Self, StoreError> {
        let stored = crate::store::months::load_all_months(pool).await?;
        let items = crate::store::months::load_all_items(pool).await?;
        let events = load_interest_events(pool).await?;
        let live = if wants_live(&stored) {
            Some(crate::service::wealth::live_totals(pool).await?)
        } else {
            None
        };
        Self::assemble(&stored, items, live, &events)
    }

    /// Open the book with live totals the caller already holds (overview,
    /// forecast compute them for their own responses). `live` is applied
    /// per-row at [`live_from`] and gated into `tail` as usual.
    pub async fn open_with_live(pool: &SqlitePool, live: LiveTotals) -> Result<Self, StoreError> {
        let stored = crate::store::months::load_all_months(pool).await?;
        let items = crate::store::months::load_all_items(pool).await?;
        let events = load_interest_events(pool).await?;
        Self::assemble(&stored, items, Some(live), &events)
    }

    fn assemble(
        stored: &[StoredMonth],
        items: HashMap<chrono::NaiveDate, Vec<MonthItemFacts>>,
        live: Option<LiveTotals>,
        events: &SuggestionEvents,
    ) -> Result<Self, StoreError> {
        let stat_rows = to_stat_rows(stored, live.as_ref(), events, &items)?;
        let tail = live_tail(&stat_rows, live);
        Ok(Self {
            stored: stored.to_vec(),
            stat_rows,
            items,
            live,
            tail,
        })
    }

    /// One stored row with its derived columns and live-resolved totals.
    /// `Err(NotFound)` while `stored.month` names no row in the book.
    pub fn present_one(&self, stored: &StoredMonth) -> Result<MonthStat, StoreError> {
        let derived = crate::calc::month_derived_with_tail(&self.stat_rows, &self.items, self.tail);
        let index = self
            .stored
            .iter()
            .position(|row| row.month == stored.month)
            .ok_or_else(|| StoreError::NotFound(format!("month {} not found", stored.month)))?;
        let sums = self
            .items
            .get(&self.stat_rows[index].month)
            .map(|items| month_item_sums(items))
            .unwrap_or_default();
        Ok(present(
            &self.stored[index],
            &derived[index],
            self.live.as_ref(),
            &sums,
            self.stat_rows[index].interest,
        ))
    }
}

/// Stat rows built from stored figures only — for the 開心Pool chain inside
/// the live-totals aggregate, which must not pull in live (it is computing
/// the live figures).
pub(crate) async fn stat_rows_no_live(
    pool: &SqlitePool,
) -> Result<
    (
        Vec<crate::calc::MonthStatRow>,
        HashMap<chrono::NaiveDate, Vec<MonthItemFacts>>,
    ),
    StoreError,
> {
    let stored = crate::store::months::load_all_months(pool).await?;
    let items = crate::store::months::load_all_items(pool).await?;
    let events = load_interest_events(pool).await?;
    let stat_rows = to_stat_rows(&stored, None, &events, &items)?;
    Ok((stat_rows, items))
}

fn to_stat_rows(
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

/// The `if needs_live { ... }` half of the gate: a NULL B/D at or after the
/// current month fills live.
fn needs_live(stored: &[StoredMonth]) -> bool {
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
fn wants_live(stored: &[StoredMonth]) -> bool {
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
fn live_tail(rows: &[crate::calc::MonthStatRow], live: Option<LiveTotals>) -> Option<LiveTotals> {
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
        ibkr_transfers: Vec::new(),
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
    // Cash-moving transfers only: log-only legacy rows already carry their
    // manual items and must never suggest.
    let ibkr_transfers = crate::store::ibkr::transfers_in_range(pool, &first, &next, true).await?;

    Ok(SuggestionEvents {
        deposits: load_deposit_events(pool, range.clone()).await?,
        trades,
        dividends: load_dividend_events(pool, range.clone()).await?,
        coupons: load_coupon_events(pool, range).await?,
        aia_payments,
        ibkr_transfers,
        pool_input,
        usd_hkd_rate: crate::store::fx::usd_rate(pool).await?,
    })
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

#[cfg(test)]
mod tests {
    use super::*;

    fn month_key(month: chrono::NaiveDate) -> String {
        month.format("%Y-%m-01").to_string()
    }

    async fn insert(
        pool: &SqlitePool,
        month: chrono::NaiveDate,
        total_assets: Option<f64>,
        liquid_assets: Option<f64>,
    ) {
        let now = crate::store::now_timestamp();
        crate::store::months::insert_month(
            pool,
            &month_key(month),
            Some(1000.0),
            Some(30000.0),
            total_assets,
            liquid_assets,
            0.0,
            None,
            None,
            &now,
        )
        .await
        .expect("insert month");
    }

    fn past(months_ago: u32) -> chrono::NaiveDate {
        live_from()
            .checked_sub_months(chrono::Months::new(months_ago))
            .expect("past month")
    }

    fn future(months_ahead: u32) -> chrono::NaiveDate {
        live_from()
            .checked_add_months(chrono::Months::new(months_ahead))
            .expect("future month")
    }

    fn row(book: &MonthBook, month: chrono::NaiveDate) -> &crate::calc::MonthStatRow {
        book.stat_rows
            .iter()
            .find(|row| row.month == month)
            .expect("stat row")
    }

    #[tokio::test]
    async fn open_fills_null_totals_live_only_at_and_after_the_current_month() {
        let pool = crate::db::connect_memory().await.expect("memory db");
        let first = live_from();
        insert(&pool, past(2), None, None).await;
        insert(&pool, first, None, None).await;
        insert(&pool, future(2), None, None).await;

        let book = MonthBook::open(&pool).await.expect("book");

        assert!(row(&book, past(2)).total_assets.is_none());
        assert!(row(&book, past(2)).liquid_assets.is_none());
        assert!(row(&book, first).total_assets.is_some());
        assert!(row(&book, first).liquid_assets.is_some());
        assert!(row(&book, future(2)).total_assets.is_some());
    }

    #[tokio::test]
    async fn open_gates_the_tail_to_a_current_last_row() {
        let pool = crate::db::connect_memory().await.expect("memory db");
        insert(&pool, past(2), None, None).await;
        insert(&pool, live_from(), None, None).await;

        let book = MonthBook::open(&pool).await.expect("book");

        assert_eq!(book.tail, book.live);
    }

    #[tokio::test]
    async fn open_leaves_the_tail_off_when_the_last_row_is_not_current() {
        let pool = crate::db::connect_memory().await.expect("memory db");
        insert(&pool, past(3), None, None).await;
        insert(&pool, live_from(), None, None).await;
        insert(&pool, future(2), None, None).await;

        let book = MonthBook::open(&pool).await.expect("book");

        assert!(book.tail.is_none());
    }

    #[tokio::test]
    async fn open_pulls_live_for_the_tail_even_without_null_totals() {
        let pool = crate::db::connect_memory().await.expect("memory db");
        insert(&pool, past(2), Some(500.0), Some(200.0)).await;
        insert(&pool, live_from(), Some(900.0), Some(300.0)).await;

        let book = MonthBook::open(&pool).await.expect("book");

        assert!(book.live.is_some());
        assert_eq!(book.tail, book.live);
        // Stored totals still win over the live fill.
        assert_eq!(row(&book, live_from()).total_assets, Some(900.0));
        assert_eq!(row(&book, live_from()).liquid_assets, Some(300.0));
    }

    #[tokio::test]
    async fn open_with_live_applies_the_caller_supplied_totals() {
        let pool = crate::db::connect_memory().await.expect("memory db");
        insert(&pool, past(2), None, None).await;
        insert(&pool, live_from(), None, None).await;

        let totals = LiveTotals {
            total_assets: 42.0,
            liquid_assets: 7.0,
        };
        let book = MonthBook::open_with_live(&pool, totals)
            .await
            .expect("book");

        assert_eq!(row(&book, live_from()).total_assets, Some(42.0));
        assert_eq!(row(&book, live_from()).liquid_assets, Some(7.0));
        assert!(row(&book, past(2)).total_assets.is_none());
        assert_eq!(book.tail, Some(totals));
    }
}
