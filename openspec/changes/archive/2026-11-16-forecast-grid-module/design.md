# Design

## Context

See proposal.md (Why).

Today the read works in three steps:

1. `service::forecast::build` loads the Live totals, settings, the trailing averages from the Month book, `load_interest_events` (every deposit, coupon and HK dividend), the window's HK trades and 轉入 rows, and every `forecast_items` row.
2. It calls `calc::forecast_months` for the chained figures. Then, per column, it filters the plan items (by string `item.month == month`), the placements (deposits whose `start_date` is in the month, using `same_month`), the trades and the 轉入 rows. It sorts the trade and 轉入 lines by (date, id) and calls `calc::forecast_cells`.
3. `forecast_cells` routes each line to its plan row and sums the figure.

`calc::forecast_months` receives `ForecastItemFacts { month, kind, amount, return_month }`, which drops the stored item's id, note and other fields. That's why the service has to filter the full rows again.

## Goals / Non-Goals

**Goals:**
- One place assigns each event to its column. A cell's lines and the column's chained amounts both come from that one grouping.
- A single pure call is enough to test the whole grid.

**Non-Goals:**
- No change to any figure, line, sort order or JSON field.
- Loading stays in `service::forecast`. The 總覽 read context (architecture review round 2, #3) is a separate change.
- Calendar, convert and plan-item write paths are untouched.

## Decisions

### D1: `calc::forecast_grid(&ForecastInput) -> Vec<ForecastMonth>` is the only public grid entry

- It builds the seven-column window, and for each column it collects that column's events once:
  - plan items whose parsed `month` equals the column;
  - deposits ending in the month (finish, interest);
  - deposits starting in the month (placements);
  - receipts (`events.dividends`, `events.coupons`) dated in the month;
  - trades dated in the month;
  - 轉入 rows dated in the month.
- From that one grouping it derives today's chained values (`deposit_finish`, `interest`, `bill`, `plans`, `deposit_plans`, `deposit_start`, `deposit_return`, `trade_flows`, `transfer_flows`, `cash`, `locked`, `semi_liquid`, `ref_check`). It also builds the display lines and passes them to the cell-routing step.
- The returned `models::ForecastMonth` values carry `month` (as `YYYY-MM-01`), `start`, `salary` and `spend` (from the input), `interest_components` (`calc::interest_components` on `input.events`, unchanged), `cells`, and the chained values.
- `forecast_months` and `forecast_cells` become private helpers, or are folded into the new function. Either way they leave the interface. `ForecastMonthDerived` becomes private too.
- Line order is kept exactly as today:
  - plan items: in input order (`load_all_items` order);
  - placements: in `events.deposits` order;
  - trade lines and IBKR lines: sorted by (date, id).
- The `+ 0.0` normalisation for −0.0 stays, but in one place.
- *Alternative considered:* keeping `forecast_months` public and having it also return the grouped events. Rejected because the service would still have to get the call sequence right.

### D2: `ForecastItemFacts<'a>` borrows its stored row

- Shape: `ForecastItemFacts<'a> { item: &'a ForecastItem, month: NaiveDate, return_month: Option<NaiveDate> }`.
- `kind` and `amount` are read through `item`, so there's a single source for both.
- `store::forecast::facts_of(&ForecastItem) -> Result<ForecastItemFacts<'_>, StoreError>` parses `month` and `return_month`. A bad stored date returns `StoreError::Corrupt("stored month {x} is not valid")` or `("stored return_month {x} is not valid")`, the same messages `parse_stored_ym` produces today. This follows the 2026-11-16 rule that stored-date parsing lives in store `facts_of` helpers.
- `ForecastInput.items: &'a [ForecastItemFacts<'a>]` replaces the owned facts.
- *Alternatives considered:*
  - Parallel slices of facts and rows, matched by index. Rejected because both sides would have to keep the pairing in sync.
  - Letting calc parse the strings. Rejected because it breaks the store `facts_of` convention.

### D3: `service::forecast::build` loads, then calls once

- `build` keeps all of today's loading.
- It maps the stored items through `facts_of`, builds `ForecastInput`, and returns `ForecastResponse { bill_amount, months: calc::forecast_grid(&input) }`.
- `same_month`, `parse_stored_ym` and the per-column `map` closure are deleted.

## Risks / Trade-offs

- **[Silent output drift during the port]** → The existing `forecast_months` and `forecast_cells` tests (26 calc tests) are ported onto `forecast_grid` with their fixed expected values unchanged. The two `service::forecast` tests and the convert tests in `routes/forecast.rs` stay as they are. A final check against the running backend compares `GET /api/forecast` JSON before and after: it must be byte-identical. Because `live_from()` is today's month, both captures must be taken on the same day.
- **[A lifetime on `ForecastItemFacts`]** → It's contained to calc/forecast plus one store helper. Each field borrows exactly one stored row.

## Migration Plan

No schema or data change. Rollback is a code revert.
