## 1. Project scaffolding

- [x] 1.1 Create the Cargo workspace root `Cargo.toml` and the `backend` crate with axum, tokio, serde/serde_json, sqlx (sqlite + runtime-tokio), tower-http (fs, cors, trace), chrono, calamine, thiserror and anyhow (pinned versions, each published at least 7 days ago); verify `cargo build` succeeds
- [x] 1.2 Add `.gitignore` covering `target/`, `node_modules/`, `frontend/dist/`, `data/` and `.env`; verify `git status --short` shows no build output or `data/wealth.db` after a build
- [x] 1.3 Scaffold `frontend/` with Vite + Vue 3 + TypeScript and a `/api` dev proxy to `127.0.0.1:8787`; verify `pnpm install && pnpm build` succeeds and `vue-tsc --noEmit` is clean

## 2. Storage layer

- [x] 2.1 Write `backend/migrations/0001_init.sql` creating `stocks` (with `UNIQUE(market, code)`) and `trades` (FK to stocks, index on `(stock_id, trade_date)`) as specified in design.md; verify by running the migration against a scratch DB and inspecting `sqlite3 data/wealth.db ".schema"`
- [x] 2.2 Implement `db.rs` opening/creating `data/wealth.db` and running migrations at startup; verify a fresh run creates the file with both tables and a second run is a no-op
- [x] 2.3 Define `models.rs` types for `Stock`, `Trade`, `Market`, `TradeType`, `InputMode` and the request/response DTOs; verify `cargo build` succeeds

## 3. Calculation core (pure, unit-tested)

- [x] 3.1 Implement HK derivation (`fee = total − shares × unit_price`) and US derivation (`total = shares × unit_price + fee`) in `calc.rs`; verify unit tests reproduce 中國銀行 30000 @ 3.2 / total 96523.15 → fee 523.15 and VOO 3 @ 696.04 + fee 1.000009 → total 2089.120009
- [x] 3.2 Implement per-trade 平均單價 (`total ÷ shares`, empty for zero shares); verify unit tests cover 96523.15/30000 → 3.217438… and the zero-share adjustment row returning `None`
- [x] 3.3 Implement per-stock summary (holdings = ΣBUY − ΣSELL shares, 總買入成本 = ΣBUY total, 加權平均買入單價 = 總買入成本 ÷ ΣBUY shares with 0 when no holdings); verify unit tests reproduce 中國銀行 → 68000 shares, 208746.64 cost, 3.069803529 average
- [x] 3.4 Implement unrealized figures (當前總市值, 未實現金額, 未實現報酬率) with empty results for missing 現價, zero holdings or zero cost; verify unit tests reproduce 68000 @ 5.91 → 401880 / 193133.36 / 0.9252046404 and the empty cases
- [x] 3.5 Implement sector rollup and market totals (buy cost, market value, share of market value, % change, "未分類" bucket for stocks without a sector); verify unit tests cover a multi-stock sector, an unsectored stock, and totals excluding stocks with no 現價
- [x] 3.6 Implement trade input validation (date format `YYYY-MM-DD`, 類別 in BUY/SELL, non-negative 股數/單價, negative fee only with a note, zero-share row allowed with a note); verify unit tests cover each rejection and each accepted edge case

## 4. HTTP API

- [x] 4.1 Implement stocks routes (`GET/POST /api/stocks`, `PATCH/DELETE /api/stocks/:id`) including manual 現價/PE/EPS/high52/low52 updates with `price_updated_at`, duplicate `(market, code)` rejection, and refusal to delete a stock that has trades; verify with an integration test hitting each case
- [x] 4.2 Implement trades routes (`GET /api/trades` with market/stock/date-range filters and ordering, `POST`, `PATCH`, `DELETE`) returning derived fee, total and per-trade 平均單價; verify with an integration test covering create → filter → edit re-derivation → delete
- [x] 4.3 Implement `GET /api/summary?market=` returning per-stock rows, sector rollup and market totals computed from stored trades only; verify an integration test shows the summary changing after a trade edit with no extra recalculation step
- [x] 4.4 Implement a JSON error type mapping validation and not-found failures to 400/404/409 with field-level messages; verify an integration test asserts the status and body for a malformed date, an unknown stock and a duplicate stock code
- [x] 4.5 Wire `main.rs`: bind loopback `127.0.0.1:8787`, mount `/api`, serve `frontend/dist` as static files with SPA fallback; verify `curl 127.0.0.1:8787/api/stocks` returns `[]` on a fresh DB

## 5. Workbook import and parity

- [x] 5.1 Implement `import_xlsx` reading `港股Trade` A:I and `美股Trade` A:H cached values (ignoring the 派息 columns J–O, template cells and blank rows) and inserting trades; verify running it against `財富分析報告.xlsx` reports 24 HK and 10 US trades imported, including the formula cell `=32825.78+70` → 32895.78 and the 0-share −0.01 adjustment row
- [x] 5.2 Extend the importer to upsert stock metadata (中文名/ticker, exchange `HKG`/`NYSE`/`NASDAQ`/`NYSEARCA`, trimmed sector) from the `港股`/`美股` sheets, including listed stocks with no trades such as `港交所` and `ＦＧ恆生紅利`; verify the stock list after import matches the sheets
- [x] 5.3 Make the import idempotent on `(stock_id, trade_date, trade_type, shares, total)` while preserving genuine duplicate source rows on the first run; verify a second run reports every row skipped and creates no new stocks or trades
- [x] 5.4 Implement `check_parity` comparing computed shares, 總買入成本 and 加權平均買入單價 per stock against the `港股`/`美股` cached values with a 1e-6 relative tolerance, failing loudly on a cell with no cached value; verify it exits 0 with zero differences after import and exits non-zero when a trade is deliberately altered
- [x] 5.5 Confirm both commands open the workbook read-only and error clearly on a missing file or missing sheet; verify `shasum` of `財富分析報告.xlsx` is unchanged after running both, and that a bogus path fails with a named-file message

## 6. Frontend

- [x] 6.1 Implement `api.ts` typed client for the stocks, trades and summary endpoints with error surfacing; verify `vue-tsc --noEmit` is clean and a smoke run lists imported trades in the browser
- [x] 6.2 Build `TradeForm.vue` with market-specific fields (HK: buy total; US: fee) and a live preview of the derived fee/total and per-trade 平均單價 before submit; verify entering the 中國銀行 and VOO examples previews 523.15 / 3.217438 and 2089.120009 / 696.373336
- [x] 6.3 Build `TradeTable.vue` with market tab, stock and date-range filters, sortable date column, inline edit and delete; verify filtering by `中移動` and by 2025-01-01..2025-12-31 returns only matching rows in the UI
- [x] 6.4 Build `SummaryView.vue` mirroring the sheet layout: per-stock columns, inline 現價 editing, sector rollup, market totals, empty (not zero) cells for stocks lacking 現價, and a caption stating that 加權平均買入單價 divides by shares bought; verify the HK view matches the `港股` sheet after prices are entered
- [x] 6.5 Build `StocksView.vue` for the stock registry and metadata editing with duplicate-code error display; verify adding a duplicate `(market, code)` shows the server error and adding a same-code stock in the other market succeeds
- [x] 6.6 Add persisted per-market stock ordering: `sort_order` migration, initial order from the workbook summary sheets, `POST /api/stocks/order`, and drag-and-drop rows in `SummaryView.vue`; verify reorder persists in the API and summary without changing calculations

## 7. Verification and handover

- [x] 7.1 Run `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, `pnpm build` and `vue-tsc --noEmit`; verify all pass with no warnings
- [x] 7.2 End-to-end pass on a fresh database: import → parity zero diffs → enter 現價 for every stock → add one HK BUY, one US BUY and one SELL → edit and delete a trade; verify holdings and 加權平均買入單價 behave as specified (average unchanged by the SELL) and the summary stays consistent
- [x] 7.3 Write `AGENTS.md` with run/build/test commands, import and parity commands, the f64/rounding and 加權平均買入單價 conventions, and the roadmap for remaining sheet sections (派息 → 定期 → Month Stat/Overview → MPF/債券/AIA); verify a fresh reader can start both servers from it alone
