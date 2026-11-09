# Tasks

## 1. Baseline

- [x] 1.1 Before any code change, capture `GET /api/forecast`, `/api/months`, `/api/months/summary`, `/api/months/<current YYYY-MM>`, `/api/overview`, `/api/year-review` from the running backend (`http://127.0.0.1:8787`) into `/tmp/deepen-baseline/*.json`; verify every file is non-empty valid JSON (`python3 -m json.tool`).

## 2. Seal the Month book

- [x] 2.1 In `backend/src/service/months.rs`, make `MonthBook`'s fields private; have `assemble` compute `derived` (with tail) once; have `open`/`open_with_live` also load `store::months::pool_rates` and `store::yearly::hk_sold_pl`; add `months()`, `month(key)`, `year_summaries()`, `running_averages()`, `trailing_averages()`, `pool_balance()`, `pool_rate(year)` per design D5 (each keeping today's tail choice); make `present` private and delete `present_one`; keep `stat_rows_no_live` `pub(crate)`. Verify with `cargo check` (call-site errors only, fixed in 2.2).
- [x] 2.2 Migrate callers: `routes::months::{list,summary,show}` (list → `book.months()` + year filter; summary → `year_summaries`/`running_averages`/`pool_balance`/`pool_rate`, `calc::pool_totals` on the summaries; show → `book.month(key)` with `None` → 404 and `pool_input` from the presented month), `service::months::upsert` (`month(key)` → `NotFound`), `service::overview::build` (`trailing_averages`, `running_averages`), `service::year_review::build` (`year_summaries`; drop its own `pool_rates`/`hk_sold_pl` loads), `routes::forecast` (`trailing_averages`). Verify `cargo check` is clean and `grep -rn "book\.\(stat_rows\|items\|tail\|live\|stored\)" backend/src` matches only inside `service/months.rs`.
- [x] 2.3 Add book-interface tests in `service/months.rs` over `connect_memory()`: `months()` matches `month(key)` for every key; `month` of a missing key is `None`; `year_summaries`/`running_averages` see the current-month tail while `trailing_averages` does not; `pool_balance` is `0.0` with no rows and the latest year's closing otherwise; `pool_rate` falls back to an earlier year. Verify with `cargo test service::months`; existing gating tests still pass.

## 3. The 預測 module (backend)

- [x] 3.1 In `backend/src/models/forecast.rs`, add `ForecastItemKind::placement_row(bank: Option<&str>)` (`Some("HS")` → `HsDeposit`, else `ScDeposit`) and `ForecastCell { kind, figure, plan_items, placements, trade_lines, ibkr_lines }` (`ToSchema`, field docs). Replace `ForecastMonth`'s `plan_items`/`deposit_placements`/`trade_lines`/`ibkr_lines` with `cells: Vec<ForecastCell>`. Verify with `cargo check` (errors only in `routes/forecast.rs`, fixed in 3.3).
- [x] 3.2 In `backend/src/calc/forecast.rs`, add the pure `forecast_cells(..)` per design D2 (seven cells in kind order, placement routing via `placement_row`, trade/IBKR lines only on `stock` sorted by date then id, figure rule: bill → `column.bill`, interest → `Some(column.interest)`, others → Σ items + Σ −principal + Σ trades + Σ IBKR or `None` when empty). Add calc tests for the spec scenarios: seven ordered empty cells; HS vs other vs unset bank routing; bill and interest mirroring; empty → `None`; stock plan + trade + IBKR sum (`-50000 + -54063.17`). Verify with `cargo test calc::forecast`.
- [x] 3.3 Create `backend/src/service/forecast.rs` with `build(pool) -> Result<ForecastResponse, StoreError>`, moving the whole read body from `routes::forecast::forecast` (stored-month parse → `StoreError::Corrupt`), using `MonthBook::trailing_averages` and `forecast_cells`; register it in `service/mod.rs`; reduce the route to one call. Move `forecast_lists_the_windows_ibkr_transfers` into the service tests, asserting the `stock` cell's `ibkr_lines`, and update the convert tests to read placements from cells where they inspect the forecast. Verify with `cargo test` (all), `cargo clippy --all-targets -- -D warnings`, and `cargo fmt --check`; `routes::tests::openapi_documents_every_api_operation` is unchanged.

## 4. Frontend

- [x] 4.1 Run `cd frontend && pnpm gen:api` to regenerate `api-types.ts`; adjust only the hand-written parts of `api.ts` if an alias for `ForecastCell` is needed (following the existing `components['schemas']` alias pattern, never hand-editing generated aliases). Verify `api-types.ts` contains `ForecastCell` and no longer has `deposit_placements` on `ForecastMonth`.
- [x] 4.2 In `frontend/src/views/OverviewView.vue`, add `cellOf(month, kind)`; render every plan row and TBC as `fmtMoney(cell.figure) || '—'`; have the editor read `plan_items`/`placements`/`trade_lines`/`ibkr_lines` from the selected cell; delete `cellItems`, `cellPlacements`, `cellTradeLines`, `cellIbkrLines`, `cellSum`. Verify with `pnpm build && pnpm exec vue-tsc --noEmit`, and confirm `grep -n "deposit_placements\|cellSum\|bank === 'HS'" frontend/src/views/OverviewView.vue` finds nothing.

## 5. Docs and glossary

- [x] 5.1 Update `docs/overview.md` (the 預測 section: the `routes/forecast.rs` derive line → `service::forecast::build`; describe `cells` with per-row lines and `figure`, and the backend-owned placement routing) and `docs/DATA_FLOW.md` (forecast wiring; the Month book's query interface). Add *Forecast cell* to `GLOSSARY.md` under Overview & planning, and refresh the *Month book* entry to name its queries. Verify that `grep -n "deposit_placements\|trade_lines\|ibkr_lines" docs/` only finds mentions in their per-cell sense. (Done: `docs/DATA_FLOW.md` turned out to hold no forecast or Month book wiring — its topic-table row stays accurate — so only `docs/overview.md` and `GLOSSARY.md` changed.)

## 6. Integration check

- [x] 6.1 Restart the backend per AGENTS.md (kill `:8787`, `nohup cargo run -p wealth-backend --bin wealth-backend > /tmp/wealth-backend.log 2>&1 < /dev/null & disown`), re-capture the six endpoints from 1.1, and diff against `/tmp/deepen-baseline`. `/api/months*`, `/api/overview` and `/api/year-review` must be identical. In `/api/forecast`, every month figure (`start`…`ref_check`, `interest_components`) must be identical, and for each month and kind the cell's lines must equal the old flat lists filtered the old frontend way. Verify the 總覽 預測 grid renders the same cell values as before in the browser.

## Workflow follow-up

- Run `trail check` before committing.
- Archive the change after review (`openspec archive deepen-forecast-and-month-book`), syncing the `overview` delta.
