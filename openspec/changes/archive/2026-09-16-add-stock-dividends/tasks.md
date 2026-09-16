## 1. Storage layer

- [x] 1.1 Write `backend/migrations/0005_dividends.sql` creating `dividends` (stock_id FK, pay_date, per_share, shares_held, buy_cost, estimated_amount, received_amount, received_price, note, timestamps) with index on `(stock_id, pay_date)`; verify `sqlite3 data/wealth.db ".schema dividends"` after a backend run
- [x] 1.2 Add `Dividend`, `NewDividend`, `DividendPatch`, `DividendStatus` to `models.rs` reusing the `nullable` double-Option pattern; verify `cargo build` succeeds

## 2. Calculation core (pure, unit-tested)

- [x] 2.1 Implement snapshot derivation in `calc.rs` — `shares_held = ΣBUY−ΣSELL shares` and `buy_cost = ΣBUY total` over trades with `trade_date <= pay_date`; verify unit test reproduces 中國銀行 68000 shares / 208746.64 cost and an early-date snapshot of 30000 / 96523.15
- [x] 2.2 Implement `validate_dividend` (pay_date `YYYY-MM-DD`, non-negative finite amounts, at least one of per_share/estimated_amount) and the `per_share × shares_held` estimate derivation; verify unit tests cover each rejection and the per-share-only path
- [x] 2.3 Implement derived fields (`status`, effective amount, `yield_on_cost`, `yield_on_price`, `variance`) with empty-when-no-denominator semantics; verify unit tests cover both rates, the pending/received switch, and missing denominators

## 3. HTTP API

- [x] 3.1 Implement `routes/dividends.rs` — `GET /api/dividends` (market/status/year/order filters), `POST` (resolve stock like trades, auto-snapshot when fields absent), `PATCH` (merge + `refresh_snapshots` + receive via received_amount/received_price), `DELETE`; `GET /api/dividends/summary?market=` returning pending list + per-year received totals + per-stock breakdown; verify an integration test covers create → list/filter → receive → edit → delete
- [x] 3.2 Wire `routes/mod.rs` (`/dividends`, `/dividends/summary`, `/dividends/{id}`, `DIVIDEND_SELECT`, `row_to_dividend`) and extend `stocks::remove` to refuse stocks with dividend records; verify an integration test shows the delete rejection naming the dividend count

## 4. Workbook import and parity

- [x] 4.1 Extend `xlsx.rs` to parse each trade sheet's J–O block into `SheetDividend` (stock, pay date incl. Excel serials, amount, shares, cached L/N rates, M formula text via `worksheet_formula`); verify unit test reads the current 港股Trade block and recovers buy_cost = M÷L and price = M÷(N×O)
- [x] 4.2 Extend `import.rs` to insert dividends idempotently on `(stock_id, pay_date, amount)` with the status heuristic (N present → received; pay_date ≤ today → received; else pending) and per-row reporting; verify a second import run reports all skipped
- [x] 4.3 Extend `parity.rs` with a dividend section (row count + Σ amount per market); verify `check_parity` exits 0 after import and non-zero on a deliberately altered amount

## 5. Frontend

- [x] 5.1 Add dividend types and calls to `api.ts`; verify `vue-tsc --noEmit` is clean
- [x] 5.2 Build `DividendForm.vue` (stock dropdown for current market, pay date, 每股派息, 預期派息 with a `per_share × shares` hint from the market summary, note); verify the preview updates as per_share changes
- [x] 5.3 Build `DividendTable.vue` (快照 股數, per-share, 預期/實收, both rates, dates, note, edit/delete; pending rows get inline 收訖 editor with 同時更新現價 checkbox); verify receiving a row shows both rates and optionally bumps the stock's 現價
- [x] 5.4 Build `DividendsView.vue` (待收派息 + 已收記錄 with year filter + per-stock/year rollup) and register the 派息 tab in `App.vue`'s `STOCK_TABS`; verify the HK/US toggle scopes the view

## 6. Verification and handover

- [x] 6.1 Run `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, `pnpm build`, `vue-tsc --noEmit`; all clean
- [x] 6.2 End-to-end on the real database: import → ~40 HK dividend rows → parity 0 → create a pending dividend → confirm snapshot frozen after a new BUY → mark received with price → both rates render; update `docs/DATA_FLOW.md` and `AGENTS.md` (move 派息 to completed)
