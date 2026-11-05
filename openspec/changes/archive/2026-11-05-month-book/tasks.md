# Tasks

## 1. service::wealth

- [x] 1.1 Create `backend/src/service/wealth.rs` holding `totals_input` (moved from `service::months::live_totals_input`) and `live_totals` (moved from `live_if_needed`); register `pub mod wealth;` in `service/mod.rs`; verify `cargo check` passes with `service::months` callers updated to the new paths.

## 2. MonthBook in service::months

- [x] 2.1 Add `MonthBook { stored, stat_rows, items, live, tail }` with `open(pool)` (live computed only when `wants_live`) and `open_with_live(pool, totals)`, plus `present_one(&stored)`; demote `wants_live`/`needs_live`/`to_stat_rows`/`live_tail` to private; delete `load_stat_rows`, `load_stat_rows_live`, `live_if_needed`, `present_month`; verify `cargo check` shows only the known call-site errors to be fixed in group 3.
- [x] 2.2 Add `#[cfg(test)]` tests over `db::connect_memory()` pinning: NULL totals fill live only at/after the current month, the tail applies only when the last row is the current month, `wants_live` fires for the tail case; verify `cargo test service::months` passes.

## 3. Migrate consumers

- [x] 3.1 `routes/months.rs`: `list`/`summary` open the book (summary keeps `pool_rates`/`hk_sold_pl` loads); `show`/`upsert` use `book.present_one`; upsert's snapshot/recapture uses `service::wealth::totals_input` + `calc::live_totals`; verify `cargo test` and route inline tests pass.
- [x] 3.2 `routes/forecast.rs::forecast`: `service::wealth::totals_input` → `calc::live_totals` → `MonthBook::open_with_live`; keep window trades/ibkr/event loads; verify `cargo test` passes.
- [x] 3.3 `service/overview.rs::build` and `service/year_review.rs::build`: `MonthBook::open_with_live` / `MonthBook::open` replace `load_stat_rows`/`load_stat_rows_live`/`live_tail`; verify `cargo test` passes.
- [x] 3.4 `routes/deposits.rs` tests: `service::months::live_totals_input` → `service::wealth::totals_input`; verify `cargo test` passes.

## 4. Docs and glossary

- [x] 4.1 Create `GLOSSARY.md` with the *Month book* term (assembled 月結 ledger: stored rows, stat rows, items, gated live totals, live tail) and update `docs/DATA_FLOW.md` wiring for the new module; verify docs render the new path.

## 5. Verification

- [x] 5.1 Run `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`; all pass.
- [x] 5.2 Restart the backend per AGENTS.md and confirm `GET /api/months`, `/api/months/summary`, `/api/forecast` return identical figures to before the refactor.

## Workflow follow-up

- Archive the change after review (`openspec archive month-book`), syncing nothing since `skip_specs: true`.
