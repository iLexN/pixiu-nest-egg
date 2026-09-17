# Tasks

## 1. Database

- [x] 1.1 Add `backend/migrations/0008_year_snapshots.sql` creating `year_snapshots (market, year, invested, cost, market_value, updated_at, PRIMARY KEY (market, year))` per design D1, and verify `cargo test` migration re-run test still passes with the new table present

## 2. Backend rollup

- [x] 2.1 Add a `YearRow` model and a pure rollup function in `backend/src/calc.rs` that groups a market's trades by `trade_date` year and dividends by `pay_date` year, producing per-year `invested` (Σ BUY − Σ SELL), cumulative `cost` (Σ BUY ≤ Dec 31), and `dividends` (Σ received_amount), then applies snapshot overrides and derives `yield_on_cost`, `yield_on_value`, `monthly_dividend`, `dividend_yoy`, `invested_yoy`, and `sold_pl: None`; verify with unit tests covering: cumulative cost across years, received-only dividends, empty YoY in the first year, snapshot override precedence, and empty market_value for past years without snapshots
- [x] 2.2 Create `backend/src/routes/yearly.rs` with `GET /api/summary/yearly?market=` returning the rows, `PATCH /api/summary/yearly/{market}/{year}` upserting/clearing `invested`/`cost`/`market_value` (reject non-finite or negative values), and `POST /api/summary/yearly/{market}/{year}/freeze` storing the currently computed cost and live market value; register routes in `routes/mod.rs` and verify with `cargo test` route-level tests (or manual curl) that a freeze then a price change leaves the frozen row unchanged

## 3. Workbook seeding

- [x] 3.1 Extend `backend/src/xlsx.rs` to parse year-end figures attributable to a single market+year (港股/美股 year blocks, YearInReview 股票 cost and "now value" cells), skipping ambiguous combined cells, and extend `backend/src/import.rs` to insert them via `INSERT OR IGNORE`; verify `cargo run -p wealth-backend --bin import_xlsx -- "財富分析報告.xlsx"` seeds rows on a fresh DB and a second import creates no duplicates and preserves user-edited snapshots

## 4. Frontend

- [x] 4.1 Add `YearRow`/`YearlySummaryResponse` types and `listYearly`, `patchYearly`, `freezeYearly` methods to `frontend/src/api.ts`; verify `pnpm exec vue-tsc --noEmit` passes
- [x] 4.2 Add a year table section to `frontend/src/views/SummaryView.vue` rendered under each market tab: columns year, invested, sold P/L (empty), 成本, 總市值, 報酬率1, 報酬率2, 派息, 月均派息, 派息 YoY, invested YoY; frozen values visually marked with snapshot `updated_at`; money 2dp, prices/yields per existing format helpers; verify the table renders for both markets via `pnpm build` and a manual check against `GET /api/summary/yearly`
- [x] 4.3 Add inline editing for `invested`/`cost`/`market_value` cells (PATCH) and a 凍結 button on the current-year row (POST freeze) that reloads the table; verify editing a past year's 市值 persists after reload and freezing the current year survives a price change

## 5. Docs and verification

- [x] 5.1 Add a "Yearly summary" section to `docs/DATA_FLOW.md` covering the GET/PATCH/freeze calls, the `year_snapshots` table, and the freeze workflow; verify the doc matches the implemented endpoints
- [x] 5.2 Run the full verification suite — `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, `cd frontend && pnpm build && pnpm exec vue-tsc --noEmit` — and confirm all pass
