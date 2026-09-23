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

Frontend form inputs declared `type="number"` (or bound with `v-model.number`) store numbers in the model, not strings — coerce with `Number(...)`/`String(...)` instead of calling `.trim()` on them.

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

### Market 上月/最高 figures

1. `GET /api/summary?market=HK|US` (`backend/src/routes/summary.rs`) upserts today's `market_history` row with the totals' `buy_cost_priced`/`market_value` whenever at least one stock is priced, backfilling a synthetic month-end row per fully elapsed empty month (`backend/src/market_history.rs`). `prices::apply` triggers the same build per touched market so bulk/CLI price imports are captured too.
2. The response's `last_month`/`max` are derived on read by reusing the MPF math (`mpf_last_month`, `mpf_max`) over history rows mapped to `(contributions, balance)`; the two maxima are independent.
3. `import_xlsx` seeds a synthetic `YYYY-12-31` row per `year_snapshots` year that has both 成本 and 總市值 (HK only — the workbook attributes none to US), plus each market's cached `last month` rate as a synthetic previous-month-end row `(Σ BUY total, Σ BUY total × (1 + rate))` and the cached `max Balance %`/`max net` cells as `app_meta` marks (`market.<MKT>.seed_max_percent`/`.seed_max_amount`) flooring the derived 最高.
4. `SummaryView.vue` shows the `percent / amount` pairs in the totals strip, colored against the current figures.

### Reordering stocks

1. Dragging a summary row in `SummaryView.vue` sends `POST /api/stocks/order` with every stock id in the selected market.
2. The backend validates the complete list and persists `sort_order` in SQLite.
3. Stock lists and summaries return rows ordered by that saved market-specific order.

### Workbook import and parity

1. `backend/src/xlsx.rs` reads `財富分析報告.xlsx` read-only, using cached formula values.
2. `backend/src/import.rs` inserts stocks, trades, 定期 deposits, the trade sheets' J–O 派息 rows, Month Stat rows/items, and the Overview manual cells (salary, pool rate, cash/asset balances) into SQLite, preserving the workbook's initial order (`sort_order`).
3. `backend/src/parity.rs` compares recomputed summaries, deposit rollups (定期!B1, month/bank rows, 定期Info year tables), dividend counts/totals, Month Stat stored/derived columns plus the yearly block, the Overview block (asset rows/shares, 半流動資金, B1/H1/J1, 美股 account cells, the averages block), and the YearInReview year blocks against the workbook's cached figures; live-linked, edited, and hand-frozen cells report as informational.
4. The workbook is never modified.

### Dividends (派息)

1. `DividendForm.vue` sends `POST /api/dividends` with the stock, pay date, and 每股派息 or 預期派息.
2. The backend freezes `shares_held`/`buy_cost` snapshots from trades with `trade_date <= pay_date`, so later buys never rewrite a recorded rate.
3. `DividendReceiveForm.vue` sends `PATCH /api/dividends/:id` with `received_amount` and optional `received_price`; the receipt price is stored on the dividend and only updates `stocks.manual_price` when 同時更新現價 is checked.
4. Derived on read: status, effective amount, `yield_on_cost` = amount ÷ buy_cost, `yield_on_price` = amount ÷ (received_price × shares_held), and estimate variance.

### Month Stat (月結)

1. `MonthStatView.vue` loads `/api/months/summary`, `/api/months?year=`, settings, and manual assets; payday entry sends `PATCH /api/months/:ym` which upserts the row — on create it snapshots the live 總數/流動資產 and defaults `salary` from `overview.salary`; the 新增月份 form takes no 月初 input, so `start_cash` stays NULL until 重新擷取 or a manual edit.
2. Derived on read over all stored rows: `end_cash = next row's start_cash − this row's salary`, then `month_spend`/`living_spend`/`saved`/`Changed` fall out; `interest` = auto events (**received** deposit interest ending in the month + received coupons + received HK dividends) + Σ `interest` items, so any 收訖 updates it with no write — an unreceived deposit previews muted in `interest_auto` without counting; NULL totals fill from live only for months at/after the current month (`*_live` flags tell the UI).
3. `GET /api/months/:ym` also returns `month_items` (adjustment / extra_spend / income / entertainment / interest — `interest` covers manual extras like bank 活期 or promos), `interest_auto` (the per-event 利息 breakdown), and auto-`suggestions` keyed by `auto_key` (deposit start/end, HK trades, HK dividends, coupons, AIA payments, pool input) for months ≥ the current month; accepting stores the key (409 on repeat), dismissing writes a tombstone in `month_item_dismissals`. The deposit 收訖 action itself records the `dep-end` item and optionally credits a cash `manual_assets` row (`POST /api/deposits/:id/receive`; `/unreceive` reverses both); the bond 收訖 does the same for the principal return (`bond-end:<id>`, `/api/bonds/:id/receive`). Coupon and dividend 收訖 likewise auto-create their `coupon:`/`div:` items and bank in — HK → the HS cash row, US → `ibkr.usd_cash` (no month item); clearing `received_amount` reverses the credit and deletes the item. Import writes each month's sheet-N leftover (`N − auto`) as an `interest` item, skipping blank cells.
4. `重新擷取` (`recapture: true`) re-snapshots both totals and `start_cash` (月初 = the live 活期 sum, Σ `cash` manual assets); `改為即時` stores NULL (live). The 開心Pool balance chains per year using `overview.pool_rate.<year>` with latest-earlier-year fallback.
5. Settings (`overview.salary`, per-year pool rates, `manual_assets`) and `deposits.start_date` are seeded once by import and never overwritten afterward.

### Overview (總覽)

1. `OverviewView.vue` loads `GET /api/overview`: `routes/overview.rs` reuses the live-totals components and returns the B1/H1/J1 headline, the A3:C10 asset table with C shares, the 半流動資金 block (已定期 = deposit **principal**, 活期 = manual `cash` rows, C14 = total − 25%×流動資產, A13 ratio), and the IBKR block.
2. Manual rows carry their `manual_assets` id and edit inline via `PATCH /api/manual-assets/:id`.
3. The 美股 sheet's IBKR cells (`ibkr.*` in `app_meta`, seeded once by import) are read-only there; they are edited on 美股 → 總覽 via `PATCH /api/ibkr` (non-negative finite numbers, `null` clears). The 轉入 input instead appends a dated row to `ibkr_transfers` (negative delta undoes an entry); 累計轉入 is the log's sum and each year's sum joins the 回顧 `invested`. `ibkr.now_value` stays manual — the IBKR app's implied FX rate differs from `aia.usd_hkd_rate`.
4. Live 總數/流動資產 count deposits at principal only and include IBKR cash, matching the workbook formulas exactly.

## Calculation conventions

- HK trade input uses `股數`, `單價`, and `buy total` including fee; the server derives `fee = buy total − 股數 × 單價`.
- US trade input uses `股數`, `單價`, and `fee`; the server derives `buy total = 股數 × 單價 + fee`.
- Per-trade `平均單價 = total ÷ 股數`; it is empty for zero-share adjustment rows.
- Per-stock holdings are `Σ BUY 股數 − Σ SELL 股數`.
- `總買入成本 = Σ BUY total`, so SELL rows do not reduce historical buy cost.
- `加權平均買入單價 = 總買入成本 ÷ Σ BUY 股數`. This intentionally divides by shares bought, not shares held, to match the spreadsheet; after a SELL it is not cost per currently held share.
- Money and prices are stored and calculated as SQLite `REAL`/`f64`, matching the spreadsheet. Do not round stored values. Display formatting only: money 2 decimal places, prices/averages up to 4, percentages 3.
- Stocks with no current price, zero holdings, or no derived market value are not included in market-value totals; the summary response lists them in `totals.excluded_codes`.
- Each stock has a persisted `sort_order` within its market. Imported stocks initially follow the workbook's `港股`/`美股` row order; dragging rows in 持倉總覽 calls `POST /api/stocks/order` with every stock id in that market.
- A stock with trades or dividend records cannot be deleted.

## Migration roadmap

Completed so far: HK/US trade registry, trade history, per-stock summaries, manual prices/metadata, 定期 deposits (registry, 未到期-until-收訖 list, optional bank-in to a cash manual asset, month/bank/year rollups), stock 派息 (estimate → receipt lifecycle with frozen holdings/cost/price snapshots), MPF (accounts, monthly balance updates with history, derived last-month/max, page note), market 上月/最高 figures (daily totals history with month-end backfill, year-end seeding), 債券 (registry with retained matured history, coupon schedule with 待定 → pending → received lifecycle, matured-bond principal 收訖 with optional bank-in), AIA (policy registry with excluded/in-account flags, one-action premium-payment and withdrawal events with delete-to-undo, manual USD→HKD rate), Month Stat 月結 (monthly ledger with derived spend/save columns, event-driven item suggestions with accept/dismiss, live-or-frozen asset totals, per-year pool chain, settings + manual Overview cells), Overview 總覽 (asset table with shares, 半流動資金 block, B1/H1/J1 headline, IBKR account block with manual inputs and derived cross-checks, inline manual-asset editing, 過去 12 個月平均 block with the 生活預算 預測 threshold), YearInReview 年結 → 回顧 (per-year ledger/investment/asset blocks derived live, `invested` = HK net trades + 當年 IBKR 轉入 + 調整, with seeded manual 收入/invested 調整/投資P/L and past-year 債券/定期 overrides in `year_review` + `year_snapshots.sold_pl`), workbook import, and parity check.

Remaining workbook content not covered by the app: the `YYYY回報率` projection sheets, the 開心Pool independent ledger, Mum, Dad, 香港年金, FIRE, ref1, and other unmigrated sections — keep maintaining those sheets by hand. The app should become the source of truth only after all sections are covered and verified.
