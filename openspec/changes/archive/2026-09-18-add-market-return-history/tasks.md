# Tasks

## 1. Schema and module

- [x] 1.1 Add `backend/migrations/0011_market_history.sql` creating `market_history` (`id`, `market`, `recorded_on`, `buy_cost_priced`, `market_value`, `synthetic`, `UNIQUE(market, recorded_on)`); verify `cargo test` migration tests pass and the table exists after running the backend once
- [x] 1.2 Add `backend/src/market_history.rs` (registered in `lib.rs`) with `record(pool, market, current: (f64, f64), today)` — anchor = `MAX(recorded_on)`, `INSERT OR IGNORE` synthetic month-end rows from `mpf_gap_month_ends(anchor, today)` carrying the previous row's values, upsert today's row with `synthetic = 0` — and `load(pool, market) -> Vec<MpfPoint>` mapping columns to `contributions`/`balance`; verify unit tests cover: first record writes no backfill, Aug→Oct record backfills September with August values, same-day replace

## 2. Summary derivation and API

- [x] 2.1 Add `MarketFigures { percent: Option<f64>, amount: f64 }` to `backend/src/models.rs` and `last_month`/`max: Option<MarketFigures>` to `SummaryResponse` in `backend/src/routes/summary.rs`; verify `cargo build` compiles
- [x] 2.2 In `summary::build`, when `totals.market_value` is `Some` call `market_history::record`, then `load` the market's rows and set `last_month` from `mpf_last_month(&points, today)` and `max` from `mpf_max(&points, current_point, None, None)` mapped to `MarketFigures`; verify unit/integration test: history row written on build, previous-month row yields 上月 figures, a new high raises 最高 with no manual step, unpriced market records nothing

## 3. Capture paths and seeding

- [x] 3.1 In `backend/src/prices.rs` `apply`, after the update transaction commits, trigger `summary::build` for each distinct market among updated stocks; verify `apply_resolves_reports_and_updates` still passes and a bulk update touching both markets writes a row for each
- [x] 3.2 In `backend/src/import.rs`, after `import_year_snapshots`, seed `INSERT OR IGNORE INTO market_history (market, recorded_on, buy_cost_priced, market_value, synthetic) SELECT market, printf('%04d-12-31', year), cost, market_value, 1 FROM year_snapshots WHERE cost IS NOT NULL AND market_value IS NOT NULL`; verify a fresh `import_xlsx` seeds HK `2023-12-31`/`2024-12-31`/`2025-12-31` rows and zero US rows, and a second import adds no duplicates

## 4. Frontend

- [x] 4.1 Add `MarketFigures` and the `last_month`/`max` fields to the summary response type in `frontend/src/api.ts`; verify `pnpm exec vue-tsc --noEmit`
- [x] 4.2 Lift `compareClass`/`fmtFigures` from `MpfView.vue` into `format.ts` and update `MpfView.vue` to import them; verify `pnpm build` and the MPF page renders unchanged
- [x] 4.3 In `SummaryView.vue` add 上月 and 最高 cells to the totals strip after 未實現報酬率 showing `percent / amount` pairs colored via `compareClass`, empty when the fields are null; verify `pnpm exec vue-tsc --noEmit` and a manual UI pass: HK shows seeded 最高, both cells color against current values

## 5. Docs and verification

- [x] 5.1 Add the market-history data flow to `docs/DATA_FLOW.md` and note the new derived figures in `AGENTS.md`; verify the docs match the implemented behavior
- [x] 5.2 Run `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, and `cd frontend && pnpm build && pnpm exec vue-tsc --noEmit`; verify all pass
- [x] 5.3 Manual check: run the backend, open 持倉總覽 for HK and US — HK shows 上月/最高 (最高 seeded from year-ends until October), US shows the same cells accruing from usage; update a price, reload, and confirm today's `market_history` row updated via `sqlite3 data/wealth.db "SELECT * FROM market_history"`

## 6. Workbook-cached figure seeding

- [x] 6.1 Add `MarketSheetCached { last_month_percent, max_percent, max_amount }` to `backend/src/xlsx.rs` as a `cached` field on `MarketSheets`, parsed by scanning the market's summary sheet for `last month` / `max Balance %` / `max net` labels and reading the next numeric cell (美股's HKD-converted K column is ignored); verify a parse test reads HK 0.33/0.3641/362869.79 and US 0.023/0.0289/322.58 from the real workbook
- [x] 6.2 In `backend/src/import.rs`, after `seed_market_history`: for each market with a cached last-month rate, `INSERT OR IGNORE` a synthetic previous-month-end row `(cost, cost × (1 + rate))` where `cost` = the market's Σ BUY total; upsert `app_meta` keys `market.<MKT>.seed_max_percent` / `.seed_max_amount` from the cached max cells (keys defined in `market_history.rs`, written via `mpf::meta_put`); verify a fresh `import_xlsx` writes the HK/US last-month rows and the four marks, and a second import stays clean
- [x] 6.3 In `summary::build`, load the market's seed marks via `routes::mpf::meta_f64` and pass them to `mpf_max`; verify a test where the seeded mark exceeds all history rows and current values still wins
- [x] 6.4 In `parity.rs` `check_market`, compare derived 上月 percent, 最高 percent and 最高 amount against the sheet's cached cells as loose named rows; verify `check_parity` reports matches after a fresh import
- [x] 6.5 Update `docs/DATA_FLOW.md` and `AGENTS.md` for the cached-cell seeding; re-run the full verification suite
