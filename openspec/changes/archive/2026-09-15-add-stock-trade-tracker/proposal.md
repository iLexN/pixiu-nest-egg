## Why

Stock trade history currently lives in `財富分析報告.xlsx` (a Google Sheets export), where every new trade must be copy/pasted across several sheets (`港股Trade`/`美股Trade` → `港股`/`美股` → `Overview`, `Month Stat`) and the HK and US sheets use different input conventions. This is repetitive and error-prone. A local app that owns the trade data removes the duplication and gives one place to enter a trade and immediately see the resulting average cost.

This change is the first migration step: it covers only the stock trade section (HK + US) so the numbers can be verified against the spreadsheet before later sections (派息, 定期, Month Stat, Overview) move over.

## What Changes

- New local web app in this repo: Rust HTTP backend (axum) with SQLite storage, Vue 3 + Vite frontend, run on localhost. No new hosted services, no external market-data calls.
- Record HK (`港股`) trades from: 股票代碼, 股數, buy total (fee included), 單價, 日期 (`YYYY-MM-DD`), 類別 (BUY/SELL) — the app derives `fee = buy total − 股數 × 單價`.
- Record US (`美股`) trades from: 股票代碼, 股數, 單價, fee, 日期, 類別 — the app derives `buy total = 股數 × 單價 + fee`.
- Every trade row shows its own 平均單價 (buy total ÷ 股數, i.e. unit price including fee), matching the sheet's `buy unit price` column.
- Per-stock summary per market reproducing the current sheet formulas: 股數 (ΣBUY − ΣSELL), 總買入成本 (Σ BUY total), 加權平均買入單價 (總買入成本 ÷ Σ BUY 股數), and — from a manually entered 現價 — 當前總市值, 未實現金額, 未實現報酬率, plus sector rollups and market totals.
- 現價 and optional PE/EPS/high52/low52 are entered manually (replacing `GOOGLEFINANCE`); no price fetching in this change.
- Edit and delete trades and stocks; summaries always recompute from stored trades (no denormalized totals).
- Manually reorder the stock list in 持倉總覽; the order is persisted per market and initially follows the workbook's summary-sheet row order.
- One-off importer that loads the existing `港股Trade` (24 rows) and `美股Trade` (10 rows) plus stock metadata (中文名, ticker, exchange, sector) from `財富分析報告.xlsx`, idempotently.
- A parity check that compares the app's computed summary against the spreadsheet's cached summary values, so the section can be signed off before the workflow switches.
- Not in this change: 派息/dividends, 定期, MPF, 債券, AIA, Overview/Month Stat aggregates, price auto-fetch, multi-user access, auth, currency conversion beyond what the summary needs. The spreadsheet remains the source of truth until all sections are migrated.

## Capabilities

### New Capabilities
- `stock-trades`: recording, listing, editing and deleting HK/US stock trades, including the market-specific input derivation (fee vs. buy total) and per-trade 平均單價.
- `stock-portfolio-summary`: per-stock and per-market aggregation — holdings, total cost, weighted average buy price, manually entered current price, market value, unrealized P/L, sector rollups.
- `spreadsheet-trade-import`: one-off idempotent import of trades and stock metadata from `財富分析報告.xlsx`, plus the parity report against the spreadsheet's cached summary values.

### Modified Capabilities
<!-- None: this is the first capability in the project. -->

## Impact

- New code: Cargo workspace with a `backend` crate (axum, sqlx/SQLite, calamine for xlsx) and a `frontend` Vue 3 + Vite app; SQLite database file under `data/` (gitignored).
- New dependencies: Rust — axum, tokio, serde, sqlx (sqlite), tower-http, chrono, calamine, thiserror/anyhow; JS — vue, vite, typescript, vue-tsc. Toolchains already installed locally (rustc 1.94.1, node 26.8.2, pnpm).
- `財富分析報告.xlsx` is read-only input for the importer; it is never written to, and its own formulas keep working during the transition.
- Deliberately replicates a spreadsheet quirk: 加權平均買入單價 divides total buy cost by total *bought* shares (not shares held), so after a SELL it is not the cost per held share. Kept for parity; revisiting it is a separate change.
- Follow-up changes will migrate the remaining sheet sections and only then retire the spreadsheet.
