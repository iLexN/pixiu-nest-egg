# Tasks

All work runs against scratch databases only (`WEALTH_DB=/tmp/...`, spare `WEALTH_ADDR`), never `data/wealth.db`, until group 6. "Stage gate" means: `cargo test`, `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` all pass, and the parity/API diff of task 1.3 shows no difference from the baseline.

## 1. Baseline

- [x] 1.1 Copy `data/wealth.db` to `/tmp/layered-baseline.db` (read-only source copy, never modified afterwards) and verify the file exists with the same size.
- [x] 1.2 Write a small throwaway script `/tmp/layered-snapshot.sh <out-dir>` that:
  - copies `/tmp/layered-baseline.db` to a fresh scratch DB;
  - runs `check_parity` against it into `<out-dir>/parity.txt`;
  - starts the backend on `127.0.0.1:8799` with that scratch DB;
  - saves `/api-docs/openapi.json` and the JSON of every parameter-free GET `/api/...` operation it lists, plus `summary`/`yearly` for both markets, into `<out-dir>/`;
  - then stops the backend.

  Verify by running it twice on unchanged code: `diff -r` of the two outputs is empty.
- [x] 1.3 Run the snapshot script on the unchanged code into `/tmp/layered-before/` and keep it as the baseline. Every stage gate below runs the script into `/tmp/layered-after-N/` and checks that `diff -r /tmp/layered-before /tmp/layered-after-N` is empty.

## 2. Stage 1 — split `calc.rs` into `calc/`

- [x] 2.1 Create `backend/src/calc/mod.rs` (shared `FieldError`, `approx_eq`, `has_note` and date helpers, plus `pub use <module>::*;` re-exports) and move the trades/stocks/rollup section into `calc/stocks.rs` with its tests. Verify `cargo test` passes and no caller import changed.
- [x] 2.2 Move the remaining sections verbatim, each with the tests that cover it:
  - deposits → `calc/deposits.rs`
  - dividends → `calc/dividends.rs`
  - yearly → `calc/yearly.rs`
  - MPF → `calc/mpf.rs`
  - bonds → `calc/bonds.rs`
  - AIA → `calc/aia.rs`
  - month stat → `calc/months.rs`
  - year review → `calc/year_review.rs`
  - live totals/liquidity/money master/goals → `calc/overview.rs`
  - suggestions/interest → `calc/suggestions.rs`
  - forecast → `calc/forecast.rs`

  Verify that `calc.rs` is gone, `cargo test` reports the same test count as before (142 calc tests plus the rest), and `src/calc/` contains no `sqlx`, `axum` or `SqlitePool`.
- [x] 2.3 Update the `calc.rs::...` references in `docs/overview.md`, `docs/stocks.md` and `docs/DATA_FLOW.md` to the new module paths, and verify that searching docs for `calc.rs` finds no stale paths.
- [x] 2.4 Run the stage gate into `/tmp/layered-after-1/` and verify the diff is empty.

## 3. Stage 2 — `StoreError` and `store/`

- [x] 3.1 Add `backend/src/store/mod.rs` with `StoreError { Database, NotFound, Corrupt }` and `From<sqlx::Error>`, and add `impl From<StoreError> for ApiError` in `error.rs` (Database→Database, NotFound→NotFound, Corrupt→Conflict, messages unchanged). Add a unit test asserting each mapping's HTTP status and that it passes.
- [x] 3.2 Create `store/stocks.rs` and `store/trades.rs`:
  - Move `STOCK_COLUMNS`, `row_to_stock`, `TRADE_SELECT`, `row_to_trade`, `stocks::{load_stocks, load_one, find_by_code}` and the trades list/insert/update/delete SQL.
  - Make the stocks/trades routes call them.

  Verify that `src/routes/{stocks,trades}.rs` contain no `sqlx::query` outside `#[cfg(test)]`, and that the route tests for 404/409/400 pass unchanged.
- [x] 3.3 Do the same for dividends (`DIVIDEND_SELECT`, `row_to_dividend`, `insert_dividend`), deposits (`DEPOSIT_COLUMNS`, `row_to_deposit`, `load_all`) and family deposits (`FAMILY_DEPOSIT_COLUMNS`, `row_to_family_deposit`). Verify there is no SQL in those route files and that `cargo test` passes.
- [x] 3.4 Do the same for bonds and coupons (`BOND_SELECT`, `COUPON_SELECT`, `row_to_bond`, `row_to_coupon`, `load_all_bonds`, `load_bond_coupons`, `insert_bond`, `insert_coupon`) and AIA (`load_all_policies`, `facts_of`, `insert_policy`, events). Verify there is no SQL in those route files and that `cargo test` passes.
- [x] 3.5 Move `mpf.rs` into `store/mpf.rs` plus `store/meta.rs` (`meta_*` helpers, `app_meta` keys) and `market_history.rs` into `store/market_history.rs`, and move the MPF route SQL there. Update the module doc comments that point at `calc.rs`/`routes/mpf.rs`. Verify `cargo test` passes, including the moved `adjacent_months_need_no_backfill` tests.
- [x] 3.6 Do the same for year snapshots (yearly/year_review), `ibkr_transfers` (overview), forecast items and `prices.rs` (retarget to `store::stocks`). Verify there is no SQL in `routes/{yearly,year_review,overview,forecast}.rs` outside tests, and that `cargo test` passes.
- [x] 3.7 Do the same for months, last: month stats, month items, dismissals, manual assets, `pool_rates`, settings keys, the event loaders and `record_receipt_item`/`hs_cash_asset_id`. Verify that `routes/months.rs` and `routes/mod.rs` contain no `sqlx::query` outside tests, and that `cargo test` passes.
- [x] 3.8 Update `docs/database.md` (or the topic docs) wherever they say which module reads or writes a table. Verify by searching docs for `routes/` file references to SQL.
- [x] 3.9 Run the stage gate into `/tmp/layered-after-2/` and verify the diff is empty.

## 4. Stage 3 — `service/` and thin routes

- [x] 4.1 Create `service/summary.rs` with `build` (keeps the `market_history` side effect) and `market_value_only` (no side effect), and move the `routes::summary` tests that cover them. Verify `build_records_todays_totals` and `unpriced_market_records_nothing` pass.
- [x] 4.2 Create `service/yearly.rs` (`build`, `hk_sold_pl`) and `service/year_review.rs` (`build`). Verify `cargo test` passes and that `routes/{yearly,year_review}.rs` handlers only extract input, call the service and map errors.
- [x] 4.3 Create `service/months.rs` (`live_totals_input`, `load_stat_rows_live`, `load_interest_events`, `goal_constants`, `semi_liquid_target`, suggestion assembly) and move the remaining multi-store logic out of `routes/months.rs`. Verify the `routes::months` tests pass.
- [x] 4.4 Create `service/overview.rs` with `build`, the body of today's `overview::overview` handler, and `ibkr_block`. Make the handler a thin wrapper. Verify the `routes::overview` tests pass.
- [x] 4.5 Move `trades::resolve_stock` and the receipt-recording composition into `service/`. Verify that `routes/dividends.rs` no longer imports `routes::trades` and that `cargo test` passes.
- [x] 4.6 Retarget `import.rs` and `parity.rs` to `service::*`/`store::*`; parity calls `service::overview::build`, not the handler. Verify that searching `src/import.rs src/parity.rs src/prices.rs src/service src/store src/calc` for `routes::` / `crate::routes` finds nothing.
- [x] 4.7 Remove helpers left orphaned in `routes/mod.rs`, keeping `AppState`, `api_router`, `parse_market` and the route tests. Verify that `cargo clippy --all-targets -- -D warnings` reports no dead code and that `routes::tests::openapi_documents_every_api_operation` passes unchanged.
- [x] 4.8 Update `docs/DATA_FLOW.md` and the topic docs so that each flow names its route → service → store → calc modules, and update the `Layout` section of `AGENTS.md` to describe `routes/`, `service/`, `store/` and `calc/` and the layer rules. Verify by reading the updated sections against the final tree.
- [x] 4.9 Run the stage gate into `/tmp/layered-after-3/` and verify the diff is empty.

## 5. Layer-rule integration check

- [x] 5.1 Run the design.md Decision 5 greps and verify every one comes back empty:
  - no `sqlx::query*` in `src/routes/` outside `#[cfg(test)]`;
  - no `routes::` outside `main.rs`/`lib.rs`/`routes/`;
  - `ApiError` only in `src/routes/` and `src/error.rs`;
  - no `sqlx`/`axum`/`SqlitePool` in `src/calc/`;
  - no `crate::service` in `src/store/`.
- [x] 5.2 Run the full verification from AGENTS.md: `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, and `cd frontend && pnpm build && pnpm exec vue-tsc --noEmit`, the last confirming that the API types the frontend relies on are unchanged. Verify that all pass.

## 6. Live restart

- [x] 6.1 Restart the real backend per AGENTS.md (`lsof -ti :8787 | xargs kill`, then `cargo run -p wealth-backend --bin wealth-backend` in the background). Verify that `GET http://127.0.0.1:8787/api/summary?market=HK` answers 200 and `/scalar` loads.

## Workflow follow-up

- Archive the change with `/openspec-archive-change` after review; no spec deltas will merge (`skip_specs: true`).
- Delete the `/tmp/layered-*` scratch files once archived.
