# Wealth report stock tracker

Local Rust + SQLite + Vue app for migrating the stock-trade sections of `財富分析報告.xlsx` out of the spreadsheet. The workbook remains the source of truth for sections that have not been migrated yet.

## Layout

- `backend/` — axum HTTP API, SQLite persistence via sqlx, workbook importer, parity checker.
- `frontend/` — Vue 3 + TypeScript + Vite UI.
- `data/wealth.db` — default local SQLite database. `data/` is gitignored because it contains personal financial data.
- `財富分析報告.xlsx` — read-only import/parity input. Do not write to it from this app.
- `docs/DATA_FLOW.md` — user-facing guide to each UI action, API call, database table, and calculation path.
- `openspec/specs/` — current behavior specifications.
- `openspec/changes/archive/2026-09-15-add-stock-trade-tracker/` — archived design, delta specs, and implementation checklist for this migration step.

## Run locally

Backend, serving the built frontend and API on `http://127.0.0.1:8787`:

```sh
cd frontend && pnpm install && pnpm build
cd ..
cargo run -p wealth-backend --bin wealth-backend
```

Development mode with Vite on `http://127.0.0.1:5173` and `/api` proxied to the backend:

```sh
cargo run -p wealth-backend --bin wealth-backend
cd frontend && pnpm dev
```

Useful environment variables:

- `WEALTH_DB=/path/to/wealth.db` — override the database path.
- `WEALTH_ADDR=127.0.0.1:8787` — override the backend bind address; keep it loopback-only.
- `WEALTH_FRONTEND_DIST=/path/to/dist` — override the static frontend directory.

## Import and parity check

Import the workbook into the default `data/wealth.db`:

```sh
cargo run -p wealth-backend --bin import_xlsx -- "財富分析報告.xlsx"
```

Bulk-update 現價 from a price file (`{"stocks": [{"symbol", "price"}]}`; `NNNN.HK` → HK ticker, others → US code/ticker with `-` ≡ `.`):

```sh
cargo run -p wealth-backend --bin import_prices -- current-price.json
```

The same update is available in the UI via 持倉總覽 → 匯入現價 JSON (`POST /api/stocks/prices`).

Compare the database summary against the workbook's cached values:

```sh
cargo run -p wealth-backend --bin check_parity -- "財富分析報告.xlsx"
```

Both commands accept a workbook path and honor `WEALTH_DB`. They open the workbook read-only. A second import is expected to skip all existing rows and create no duplicates.

## Verification

```sh
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cd frontend && pnpm build && pnpm exec vue-tsc --noEmit
```

## Data flow

The backend is the source of truth for validation, persistence, and all financial calculations. The Vue frontend collects input, renders API responses, formats values for display, and shows temporary previews before submission; previews are not treated as authoritative results.

### Updating 現價

1. `SummaryView.vue` sends `PATCH /api/stocks/:id` with `manual_price`.
2. `backend/src/routes/stocks.rs` stores the price and updates `price_updated_at` in SQLite.
3. The frontend reloads `GET /api/summary?market=HK|US`.
4. `backend/src/routes/summary.rs` and `backend/src/calc.rs` recompute market value, unrealized amount/return, sector rollups, and totals from the stored stock/trade rows.
5. The frontend only formats and displays the returned numbers.

### Bulk-updating 現價 from a file

1. `SummaryView.vue` sends the picked file's text to `POST /api/stocks/prices` (or `import_prices` reads a local path).
2. `backend/src/prices.rs` parses `{"stocks": [{"symbol", "price"}]}`, resolves `NNNN.HK` to HK tickers and other symbols to US codes/tickers (`-` ≡ `.`), then updates `manual_price`/`price_updated_at` in one transaction.
3. The report lists updated stocks, unmatched symbols, invalid entries, and stocks with no entry; the frontend reloads the summary afterward.

### Creating or editing a trade

1. `TradeForm.vue` or `TradeTable.vue` sends the user-entered fields to `POST /api/trades` or `PATCH /api/trades/:id`.
2. `backend/src/calc.rs` validates the trade and derives the missing value:
   - HK: `fee = buy total − 股數 × 單價`
   - US: `buy total = 股數 × 單價 + fee`
3. SQLite stores shares, unit price, fee, total, and input mode.
4. The response includes the stored row plus per-trade `平均單價`; summaries recompute from trades on the next summary request.

The live formula preview in `TradeForm.vue` mirrors those rules only so the user can check the numbers before submitting.

### Reordering stocks

1. Dragging a summary row in `SummaryView.vue` sends `POST /api/stocks/order` with every stock id in the selected market.
2. The backend validates the complete list and persists `sort_order` in SQLite.
3. Stock lists and summaries return rows ordered by that saved market-specific order.

### Workbook import and parity

1. `backend/src/xlsx.rs` reads `財富分析報告.xlsx` read-only, using cached formula values.
2. `backend/src/import.rs` inserts stocks, trades, and 定期 deposits into SQLite, preserving the workbook's initial order (`sort_order`).
3. `backend/src/parity.rs` compares recomputed summaries and deposit rollups (定期!B1, month/bank rows, 定期Info year tables) against the workbook's cached figures.
4. The workbook is never modified.

## Calculation conventions

- HK trade input uses `股數`, `單價`, and `buy total` including fee; the server derives `fee = buy total − 股數 × 單價`.
- US trade input uses `股數`, `單價`, and `fee`; the server derives `buy total = 股數 × 單價 + fee`.
- Per-trade `平均單價 = total ÷ 股數`; it is empty for zero-share adjustment rows.
- Per-stock holdings are `Σ BUY 股數 − Σ SELL 股數`.
- `總買入成本 = Σ BUY total`, so SELL rows do not reduce historical buy cost.
- `加權平均買入單價 = 總買入成本 ÷ Σ BUY 股數`. This intentionally divides by shares bought, not shares held, to match the spreadsheet; after a SELL it is not cost per currently held share.
- Money and prices are stored and calculated as SQLite `REAL`/`f64`, matching the spreadsheet. Do not round stored values. Display formatting only: money 2 decimal places, prices/averages up to 4, percentages 2.
- Stocks with no current price, zero holdings, or no derived market value are not included in market-value totals; the summary response lists them in `totals.excluded_codes`.
- Each stock has a persisted `sort_order` within its market. Imported stocks initially follow the workbook's `港股`/`美股` row order; dragging rows in 持倉總覽 calls `POST /api/stocks/order` with every stock id in that market.

## Migration roadmap

Completed so far: HK/US trade registry, trade history, per-stock summaries, manual prices/metadata, 定期 deposits (registry, upcoming/history views, month/bank/year rollups), workbook import, and parity check.

Remaining spreadsheet sections, in intended order:

1. 派息
2. Month Stat / Overview
3. MPF / 債券 / AIA

Until those are migrated, continue maintaining the workbook's non-trade sheets by hand. The app should become the source of truth only after all sections are covered and verified.
