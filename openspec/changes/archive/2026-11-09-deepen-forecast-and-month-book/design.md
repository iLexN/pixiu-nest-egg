# Design

## Context

See proposal.md (Why) for the motivation. This design depends on the following current state.

- `routes::forecast::forecast` (≈160 lines) loads settings, live totals, the current month's `start_cash`, the Month book (`open_with_live`, used only for `trailing_averages`), interest events, window HK trades, window IBKR transfers, and forecast items. It parses item months, calls `calc::forecast_months`, and then builds each column's `plan_items`, `deposit_placements`, `trade_lines`, `ibkr_lines` (sorted by date then id) and `interest_components`.
- `OverviewView.vue` turns those flat lists into grid rows. `cellPlacements` maps bank `HS` to `hs_deposit` and every other bank (including null) to `sc_deposit`. `cellTradeLines`/`cellIbkrLines` send lines to `stock`. `cellSum` adds the figures, and the `bill`/`interest` rows read `month.bill`/`month.interest` instead.
- `MonthBook` exposes `stored`, `stat_rows`, `items`, `live` and `tail` as pub fields. Its outside callers are:
  - `routes::months::list`: hand-zips `month_derived_with_tail` with `present`, which copies `present_one`.
  - `routes::months::summary`: `month_year_summaries` with tail, `month_running_averages` with tail, and `pool_balances` without tail; it also loads `pool_rates` and `hk_sold_pl` itself.
  - `routes::months::show` and `service::months::upsert`: `present_one(&stored)`.
  - `service::overview::build`: `trailing_averages(.., live_from())` without tail, plus `month_running_averages` with tail.
  - `service::year_review::build`: `month_year_summaries` with tail; loads `pool_rates` and `hk_sold_pl` itself.
  - `routes::forecast`: `trailing_averages(.., first)`, where `first == live_from()`.
- ADR-0005 (routes thin, `calc/` pure, `service/` composes) and ADR-0006 (the book is the only month-read path, and its pipeline stays private) still hold. This change applies them.
- `StoreError::Corrupt` already maps to 409, the same status the route's `parse_stored_ym` returned. Moving the parse into the service therefore keeps the error status unchanged.

## Goals / Non-Goals

**Goals:**
- One 預測 read whose interface is `service::forecast::build(pool)`. Tests exercise it without axum `State`.
- The rules for which row a line belongs in, and for each cell's figure, live in Rust and are tested there.
- The Month book's interface is a set of questions. Callers never pass `stat_rows`/`items`/`tail` themselves.

**Non-Goals:**
- Moving the forecast item writes (`create_item`, `update_item`, `remove_item`, `convert`) and their validation out of `routes::forecast`. That is the separate candidate for 定期 placement and route validation.
- Moving Candidate building for `GET /api/months/{ym}` (`build_suggestions` and dismissals) into the book. That is the separate candidate for route-assembled summaries.
- Changing any forecast or month figure.
- Changing the 投資目標 footnote math in `OverviewView` (separate candidate).

## Decisions

### D1. `service::forecast::build(pool) -> Result<ForecastResponse, StoreError>`

The whole read body of the current route moves into a new `backend/src/service/forecast.rs`. Stored-month parse failures become `StoreError::Corrupt`, which maps to the same 409. The route becomes `Ok(Json(service::forecast::build(&state.pool).await?))`. The existing route test `forecast_lists_the_windows_ibkr_transfers` moves to the service and calls `build` directly, asserting through `cells`.

*Alternative: leave orchestration in the route and only add cells.* Rejected: the route would keep its 8 loaders, against ADR-0005.

### D2. Cell assembly is a pure calc function

`calc::forecast::forecast_cells(column: &ForecastMonthDerived, items, placements, trades, transfers) -> Vec<ForecastCell>` receives that month's already-filtered inputs and returns seven cells in `ForecastItemKind` declaration order (`hs_deposit, sc_deposit, interest, tax, stock, bill, other`, which matches the sheet). `service::forecast` filters each input by month and calls it. The figure rule (spec: *Forecast cell figure*) is:
- `bill` → `column.bill`
- `interest` → `Some(column.interest)`
- all other kinds → Σ items + Σ(−principal) + Σ trade amounts + Σ IBKR amounts, or `None` while all four lists are empty.

Trade and IBKR lines keep their current sort (date, then id) and are produced only on the `stock` cell.

*Alternative: build the cells inside `service::forecast`.* Rejected: the rule is pure, and calc tests are the cheapest place to pin it (one adapter, so no new seam).

### D3. Placement routing lives on `ForecastItemKind`

`ForecastItemKind::placement_row(bank: Option<&str>) -> ForecastItemKind` returns `HsDeposit` for `Some("HS")` and `ScDeposit` otherwise. It sits next to `default_bank`, the reverse mapping used by convert, so both directions are in one place. Matching stays exact and case-sensitive, as in today's frontend.

### D4. `cells` is an ordered array, not a map

`ForecastMonth.cells: Vec<ForecastCell>` where `ForecastCell { kind, figure: Option<f64>, plan_items: Vec<ForecastItem>, placements: Vec<ForecastPlacement>, trade_lines: Vec<ForecastTrade>, ibkr_lines: Vec<ForecastIbkrTransfer> }`. The month-level `plan_items`, `deposit_placements`, `trade_lines` and `ibkr_lines` fields are removed. `interest_components` stays on the month because the 利息 editor reads it there. An array keeps the generated TS type simple, utoipa renders a `HashMap<enum, _>` poorly, and the order is the sheet's row order.

*Alternative: keep the flat lists alongside `cells`.* Rejected: two interfaces for the same lines invite whichever one is closest at hand. ADR-0006 rejected exactly this kind of parallel interface.

### D5. The sealed Month book interface

```rust
impl MonthBook {
    pub async fn open(pool) -> Result<Self, StoreError>;
    pub async fn open_with_live(pool, live: LiveTotals) -> Result<Self, StoreError>;
    pub fn months(&self) -> Vec<MonthStat>;                 // every stored month, presented
    pub fn month(&self, key: &str) -> Option<MonthStat>;    // `YYYY-MM-01`
    pub fn year_summaries(&self) -> Vec<MonthYearSummary>;  // with tail, rates, hk_sold_pl
    pub fn running_averages(&self) -> MonthRunningAverages; // with tail
    pub fn trailing_averages(&self) -> TrailingAverages;    // window at live_from(), no tail
    pub fn pool_balance(&self) -> f64;                      // latest stored year's closing, 0 when none
    pub fn pool_rate(&self, year: i32) -> Option<f64>;      // with earlier-year fallback
}
```

- All fields become private. `assemble` also computes `derived = month_derived_with_tail(..)` once, so `months()`/`month()` don't re-derive on every call. `present_one` re-derives on each call today.
- `open`/`open_with_live` also load `store::months::pool_rates` and `store::yearly::hk_sold_pl` (one query each). Every caller of the year summaries loaded them already.
- Each method keeps the tail choice its caller makes today, so figures are byte-identical: summaries and running averages use the tail; trailing averages and pool balances don't.
- `month()` returns `Option`. `routes::months::show` and `service::months::upsert` map `None` to `StoreError::NotFound("month … not found")`. `show` reads `pool_input` from the presented `MonthStat` instead of a separate `store::load_month`.
- `present` and `present_one` become private or are deleted. `stat_rows_no_live` stays `pub(crate)` because it guards against recursion inside `service::wealth::totals_input` (ADR-0006).

*Alternative: lazy accessors that load on demand.* ADR-0006 already rejected this (async accessors add friction and save nothing).

### D6. Frontend reads cells

`OverviewView.vue` gains `cellOf(month, kind) = month.cells.find(c => c.kind === kind)`. The grid renders `fmtMoney(cell.figure) || '—'` for every plan row and TBC. `fmtMoney(0)` formats as `0.00`, so the interest row still never shows `—`. The editor reads `plan_items`/`placements`/`trade_lines`/`ibkr_lines` from the selected cell. `cellItems`, `cellPlacements`, `cellTradeLines`, `cellIbkrLines` and `cellSum` are deleted. Types come from `pnpm gen:api`; the aliases in `api.ts` are never hand-edited.

## Risks / Trade-offs

- [Breaking response shape] → The only consumer is the in-repo frontend, and it ships in the same change. `vue-tsc` after `pnpm gen:api` catches every stale field access.
- [The book's `open` does two more queries for callers that don't need year data (`list`, `show`, forecast, overview)] → Two small indexed reads. That costs less than leaking `rates`/`hk_sold_pl` loading back to callers.
- [Figure drift during the move] → Before the refactor, capture `/api/forecast`, `/api/months`, `/api/months/summary`, `/api/overview` and `/api/year-review` from the running backend. Afterwards, diff every figure. Only forecast's line placement may differ.
- [A cell figure differs from the old frontend sum by float order] → Use the same summation order as `cellSum` (items, placements, trades, IBKR).

## Migration Plan

There is no data migration. Backend and frontend ship together. Rollback is a git revert.
