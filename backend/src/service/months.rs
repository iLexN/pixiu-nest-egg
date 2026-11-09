use std::collections::{BTreeMap, HashMap};

use chrono::Datelike;
use sqlx::SqlitePool;

use crate::calc::FieldError;
use crate::calc::{
    LiveTotals, MonthDerived, MonthItemFacts, MonthItemSums, MonthRunningAverages,
    MonthYearSummary, SuggestionAiaPayment, SuggestionDeposit, SuggestionEvents, SuggestionReceipt,
    SuggestionTrade, TrailingAverages, auto_interest, month_item_sums,
};
use crate::models::{
    MonthItem, MonthItemCategory, MonthItemPatch, MonthStat, MonthStatPatch, NewMonthItem,
};
use crate::service::ServiceError;
use crate::store::months::StoredMonth;
use crate::store::{StoreError, now_timestamp, today};

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
    stored: Vec<StoredMonth>,
    /// One [`crate::calc::MonthStatRow`] per stored row, NULL totals on
    /// current/future months filled from `live` when held.
    stat_rows: Vec<crate::calc::MonthStatRow>,
    /// All `month_items`, grouped by month.
    items: HashMap<chrono::NaiveDate, Vec<MonthItemFacts>>,
    /// The derived columns per stored row, the last row diffing against
    /// `tail`.
    derived: Vec<MonthDerived>,
    /// The gated live totals — `Some` only when a stored NULL or the
    /// current-month tail needs them.
    live: Option<LiveTotals>,
    /// The live totals the last row's Changed cells diff against — `Some`
    /// only when the last stored month IS the current month.
    tail: Option<LiveTotals>,
    /// The per-year 開心Pool rates.
    rates: BTreeMap<i32, f64>,
    /// The stored HK sold P/L per year (投資純利's second term).
    hk_sold_pl: BTreeMap<i32, f64>,
}

impl MonthBook {
    /// Open the book, computing live totals only when `wants_live` — a
    /// current/future NULL B/D to fill, or a current last row needing the tail.
    pub async fn open(pool: &SqlitePool) -> Result<Self, StoreError> {
        let stored = crate::store::months::load_all_months(pool).await?;
        let live = if wants_live(&stored) {
            Some(crate::service::wealth::live_totals(pool).await?)
        } else {
            None
        };
        Self::assemble(pool, stored, live).await
    }

    /// Open the book with live totals the caller already holds (overview,
    /// forecast compute them for their own responses). `live` is applied
    /// per-row at [`live_from`] and gated into `tail` as usual.
    pub async fn open_with_live(pool: &SqlitePool, live: LiveTotals) -> Result<Self, StoreError> {
        let stored = crate::store::months::load_all_months(pool).await?;
        Self::assemble(pool, stored, Some(live)).await
    }

    async fn assemble(
        pool: &SqlitePool,
        stored: Vec<StoredMonth>,
        live: Option<LiveTotals>,
    ) -> Result<Self, StoreError> {
        let items = crate::store::months::load_all_items(pool).await?;
        let events = load_interest_events(pool).await?;
        let rates = crate::store::months::pool_rates(pool).await?;
        let hk_sold_pl = crate::store::yearly::hk_sold_pl(pool).await?;
        let stat_rows = to_stat_rows(&stored, live.as_ref(), &events, &items)?;
        let tail = live_tail(&stat_rows, live);
        let derived = crate::calc::month_derived_with_tail(&stat_rows, &items, tail);
        Ok(Self {
            stored,
            stat_rows,
            items,
            derived,
            live,
            tail,
            rates,
            hk_sold_pl,
        })
    }

    fn present_at(&self, index: usize) -> MonthStat {
        let sums = self
            .items
            .get(&self.stat_rows[index].month)
            .map(|items| month_item_sums(items))
            .unwrap_or_default();
        present(
            &self.stored[index],
            &self.derived[index],
            self.live.as_ref(),
            &sums,
            self.stat_rows[index].interest,
        )
    }

    /// Every stored month with its derived columns and live-resolved totals,
    /// in `month` order.
    pub fn months(&self) -> Vec<MonthStat> {
        (0..self.stored.len())
            .map(|index| self.present_at(index))
            .collect()
    }

    /// One presented month by its `YYYY-MM-01` key; `None` while the book
    /// holds no such row.
    pub fn month(&self, key: &str) -> Option<MonthStat> {
        self.stored
            .iter()
            .position(|row| row.month == key)
            .map(|index| self.present_at(index))
    }

    /// The sheet's per-year rows 2–4 (with the current-month tail).
    pub fn year_summaries(&self) -> Vec<MonthYearSummary> {
        crate::calc::month_year_summaries(
            &self.stat_rows,
            &self.items,
            &self.rates,
            &self.hk_sold_pl,
            self.tail,
        )
    }

    /// The sheet's row-8 running averages (with the current-month tail).
    pub fn running_averages(&self) -> MonthRunningAverages {
        crate::calc::month_running_averages(&self.stat_rows, &self.items, self.tail)
    }

    /// The 過去 12 個月 window ending before the current month (no tail).
    pub fn trailing_averages(&self) -> TrailingAverages {
        crate::calc::trailing_averages(&self.stat_rows, &self.items, live_from())
    }

    /// The 開心Pool closing balance of the latest stored month's year; `0`
    /// while no month is stored.
    pub fn pool_balance(&self) -> f64 {
        self.stat_rows
            .last()
            .and_then(|latest| {
                crate::calc::pool_balances(&self.stat_rows, &self.items, &self.rates)
                    .get(&latest.month.year())
                    .copied()
            })
            .unwrap_or(0.0)
    }

    /// The effective pool rate for `year`, falling back to the latest
    /// earlier year's rate.
    pub fn pool_rate(&self, year: i32) -> Option<f64> {
        crate::calc::pool_rate_for_year(&self.rates, year)
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

fn present(
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

/// `None` keeps `existing`; a present value is trimmed and `null`/blank
/// clears it — the shared PATCH-merge rule for nullable text.
fn merge_text(patch: Option<Option<String>>, existing: Option<String>) -> Option<String> {
    match patch {
        Some(value) => value
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        None => existing,
    }
}

/// Upsert a month row. Creating snapshots the live totals and the current
/// salary; explicit `null` on a total keeps it live, `recapture` re-snapshots
/// the totals plus 月初 (the live 活期 cash sum). Returns the row with its
/// derived columns.
pub async fn upsert(
    pool: &SqlitePool,
    month: chrono::NaiveDate,
    patch: &MonthStatPatch,
) -> Result<MonthStat, ServiceError> {
    let month_key = month.to_string();
    let existing = crate::store::months::load_month(pool, &month_key).await?;
    let now = now_timestamp();

    match existing {
        None => {
            // Body-supplied totals win; otherwise snapshot the live values.
            let live = crate::calc::live_totals(&crate::service::wealth::totals_input(pool).await?);
            let total_assets = patch.total_assets.unwrap_or(Some(live.total_assets));
            let liquid_assets = patch.liquid_assets.unwrap_or(Some(live.liquid_assets));
            let salary = match patch.salary {
                Some(salary) => Some(salary),
                None => crate::store::settings::Settings::load(pool).await?.salary,
            };
            crate::store::months::insert_month(
                pool,
                &month_key,
                patch.start_cash,
                salary,
                total_assets,
                liquid_assets,
                patch.pool_input.unwrap_or(0.0),
                patch.end_cash_override.flatten(),
                patch.note.clone().flatten(),
                &now,
            )
            .await?;
        }
        Some(existing) => {
            // A present field wins over recapture, recapture wins over the
            // stored value, and `null` stores NULL (live).
            let input = if patch.recapture == Some(true) {
                Some(crate::service::wealth::totals_input(pool).await?)
            } else {
                None
            };
            let live = input.map(|input| crate::calc::live_totals(&input));
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
            crate::store::months::update_month(
                pool,
                &month_key,
                start_cash,
                patch.salary.or(existing.salary),
                total_assets,
                liquid_assets,
                patch.pool_input.unwrap_or(existing.pool_input),
                patch
                    .end_cash_override
                    .unwrap_or(existing.end_cash_override),
                merge_text(patch.note.clone(), existing.note),
                &now,
            )
            .await?;
        }
    }

    MonthBook::open(pool)
        .await?
        .month(&month_key)
        .ok_or_else(|| ServiceError::NotFound(format!("month {month_key} not found")))
}

/// Insert a month item: accepting a suggestion (any `auto_key`) clears its
/// dismissal tombstone in the same transaction, so it can't resurface.
/// `Err(NotFound)` while `month_key` names no month; `Err(Validation)` when
/// `exclude_from_living` rides a non-娛樂支出 item. A duplicate
/// (month, auto_key) hits the partial unique index → `Err(Conflict)`.
pub async fn create_item(
    pool: &SqlitePool,
    month_key: &str,
    body: &NewMonthItem,
) -> Result<MonthItem, ServiceError> {
    if crate::store::months::load_month(pool, month_key)
        .await?
        .is_none()
    {
        return Err(ServiceError::NotFound(format!(
            "month {month_key} not found"
        )));
    }
    if body.exclude_from_living && body.category != MonthItemCategory::Entertainment {
        return Err(ServiceError::Validation(vec![FieldError::new(
            "exclude_from_living",
            "exclude_from_living only applies to 娛樂支出 items",
        )]));
    }
    let label = body
        .label
        .as_deref()
        .map(str::trim)
        .filter(|label| !label.is_empty())
        .map(str::to_string);
    let auto_key = body
        .auto_key
        .as_deref()
        .map(str::trim)
        .filter(|key| !key.is_empty())
        .map(str::to_string);
    let note = body
        .note
        .as_deref()
        .map(str::trim)
        .filter(|note| !note.is_empty())
        .map(str::to_string);

    let mut tx = pool.begin().await.map_err(StoreError::from)?;
    if let Some(auto_key) = &auto_key {
        crate::store::months::clear_dismissal(&mut tx, month_key, auto_key).await?;
    }
    let id = crate::store::months::insert_item(
        &mut tx,
        month_key,
        body.category.as_str(),
        &label,
        body.amount,
        body.exclude_from_living,
        &auto_key,
        &note,
    )
    .await?;
    tx.commit().await.map_err(StoreError::from)?;

    Ok(crate::store::months::item_row(pool, id).await?)
}

/// Update a month item. The `exclude_from_living` flag is entertainment-only:
/// moving an item to another category clears it, while setting it on a
/// non-entertainment item is `Err(Validation)`.
pub async fn update_item(
    pool: &SqlitePool,
    id: i64,
    patch: &MonthItemPatch,
) -> Result<MonthItem, ServiceError> {
    let existing = crate::store::months::load_item(pool, id).await?;

    let category = patch.category.unwrap_or(existing.category);
    let label = merge_text(patch.label.clone(), existing.label);
    let amount = patch.amount.unwrap_or(existing.amount);
    let note = merge_text(patch.note.clone(), existing.note);
    if !amount.is_finite() {
        return Err(ServiceError::Validation(vec![FieldError::new(
            "amount",
            "amount must be a number",
        )]));
    }
    let exclude_from_living = match (patch.exclude_from_living, category) {
        (Some(true), MonthItemCategory::Entertainment) => true,
        (Some(true), _) => {
            return Err(ServiceError::Validation(vec![FieldError::new(
                "exclude_from_living",
                "exclude_from_living only applies to 娛樂支出 items",
            )]));
        }
        (Some(false), _) => false,
        (None, MonthItemCategory::Entertainment) => existing.exclude_from_living,
        (None, _) => false,
    };

    crate::store::months::update_item(
        pool,
        id,
        category.as_str(),
        &label,
        amount,
        exclude_from_living,
        &note,
    )
    .await?;

    Ok(crate::store::months::item_row(pool, id).await?)
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

    fn empty_patch() -> MonthStatPatch {
        MonthStatPatch {
            start_cash: None,
            salary: None,
            total_assets: None,
            liquid_assets: None,
            pool_input: None,
            end_cash_override: None,
            note: None,
            recapture: None,
        }
    }

    fn item(category: MonthItemCategory, exclude: bool, auto_key: Option<&str>) -> NewMonthItem {
        NewMonthItem {
            category,
            label: None,
            amount: 5.0,
            exclude_from_living: exclude,
            auto_key: auto_key.map(str::to_string),
            note: None,
        }
    }

    #[tokio::test]
    async fn upsert_create_snapshots_the_live_totals() {
        let pool = crate::db::connect_memory().await.expect("memory db");
        let stat = upsert(&pool, live_from(), &empty_patch())
            .await
            .expect("create");
        // Empty DB → live totals are 0.0, snapshotted as stored values.
        assert_eq!(stat.total_assets, Some(0.0));
        assert_eq!(stat.liquid_assets, Some(0.0));
    }

    #[tokio::test]
    async fn create_item_clears_the_dismissal_and_rejects_flag_on_non_entertainment() {
        let pool = crate::db::connect_memory().await.expect("memory db");
        let first = live_from();
        insert(&pool, first, Some(1.0), Some(1.0)).await;
        let month_key = month_key(first);
        crate::store::months::dismiss_item(&pool, &month_key, "dep-start:1")
            .await
            .expect("dismiss");

        let reject = create_item(
            &pool,
            &month_key,
            &item(MonthItemCategory::Income, true, None),
        )
        .await;
        assert!(matches!(reject, Err(ServiceError::Validation(_))));

        create_item(
            &pool,
            &month_key,
            &item(MonthItemCategory::ExtraSpend, false, Some("dep-start:1")),
        )
        .await
        .expect("accept");
        let dismissed = crate::store::months::dismissed_keys(&pool, &month_key)
            .await
            .expect("keys");
        assert!(!dismissed.contains("dep-start:1"));
    }

    #[tokio::test]
    async fn update_item_category_change_clears_the_entertainment_flag() {
        let pool = crate::db::connect_memory().await.expect("memory db");
        let first = live_from();
        insert(&pool, first, Some(1.0), Some(1.0)).await;
        let month_key = month_key(first);
        let item = create_item(
            &pool,
            &month_key,
            &item(MonthItemCategory::Entertainment, true, None),
        )
        .await
        .expect("item");
        assert!(item.exclude_from_living);

        let moved = update_item(
            &pool,
            item.id,
            &MonthItemPatch {
                category: Some(MonthItemCategory::Income),
                ..Default::default()
            },
        )
        .await
        .expect("move");
        assert!(!moved.exclude_from_living);

        let flag = update_item(
            &pool,
            item.id,
            &MonthItemPatch {
                exclude_from_living: Some(true),
                ..Default::default()
            },
        )
        .await;
        assert!(matches!(flag, Err(ServiceError::Validation(_))));
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

    #[tokio::test]
    async fn months_and_month_present_the_same_rows() {
        let pool = crate::db::connect_memory().await.expect("memory db");
        insert(&pool, past(2), Some(500.0), Some(200.0)).await;
        insert(&pool, past(1), Some(600.0), Some(250.0)).await;
        insert(&pool, live_from(), None, None).await;

        let book = MonthBook::open(&pool).await.expect("book");
        let months = book.months();

        assert_eq!(months.len(), 3);
        for stat in &months {
            let one = book.month(&stat.month).expect("present");
            assert_eq!(
                serde_json::to_value(&one).unwrap(),
                serde_json::to_value(stat).unwrap()
            );
        }
        assert_eq!(months[0].total_change, Some(100.0));
        assert!(book.month(&month_key(future(5))).is_none());
    }

    /// The current month's Changed diffs against the live tail in the row,
    /// the year summaries and the running averages; the trailing window ends
    /// before the current month and never sees it.
    #[tokio::test]
    async fn tail_reaches_summaries_and_running_but_not_trailing() {
        let pool = crate::db::connect_memory().await.expect("memory db");
        insert(&pool, live_from(), Some(900.0), Some(300.0)).await;
        let book = MonthBook::open_with_live(
            &pool,
            LiveTotals {
                total_assets: 1000.0,
                liquid_assets: 350.0,
            },
        )
        .await
        .expect("book");

        assert_eq!(book.months()[0].total_change, Some(100.0));
        let year = book
            .year_summaries()
            .into_iter()
            .find(|summary| summary.year == live_from().year())
            .expect("current year");
        assert_eq!(year.total_change_sum, Some(100.0));
        assert_eq!(book.running_averages().total_change_avg, Some(100.0));
        assert_eq!(book.trailing_averages().total_change, None);
    }

    #[tokio::test]
    async fn pool_balance_is_zero_without_months_and_pool_rate_falls_back() {
        let pool = crate::db::connect_memory().await.expect("memory db");
        sqlx::query("INSERT INTO pool_rates (year, rate) VALUES (3000, 0.25)")
            .execute(&pool)
            .await
            .unwrap();

        let book = MonthBook::open(&pool).await.expect("book");

        assert_eq!(book.pool_balance(), 0.0);
        assert_eq!(book.pool_rate(3000), Some(0.25));
        assert_eq!(book.pool_rate(3005), Some(0.25));
    }

    #[tokio::test]
    async fn pool_balance_reports_the_latest_years_closing() {
        let pool = crate::db::connect_memory().await.expect("memory db");
        insert(&pool, past(1), Some(500.0), Some(200.0)).await;
        crate::store::months::insert_item(
            &mut pool.acquire().await.unwrap(),
            &month_key(past(1)),
            "entertainment",
            &None,
            40.0,
            false,
            &None,
            &None,
        )
        .await
        .unwrap();

        let book = MonthBook::open(&pool).await.expect("book");
        let rows = crate::store::months::load_all_items(&pool).await.unwrap();
        let (stat_rows, _) = stat_rows_no_live(&pool).await.unwrap();
        let rates = crate::store::months::pool_rates(&pool).await.unwrap();
        let expected = crate::calc::pool_balances(&stat_rows, &rows, &rates)[&past(1).year()];

        assert_eq!(book.pool_balance(), expected);
        assert_ne!(expected, 0.0);
    }
}
