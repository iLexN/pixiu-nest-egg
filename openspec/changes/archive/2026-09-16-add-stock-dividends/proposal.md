## Why

Stock dividends (派息) are still tracked by hand in the `港股Trade`/`美股Trade` sheets' J–O columns (and today in Google Sheets): each event needs a manually computed estimate (`每股派息 × 股數`, adjusted for FX/fees/withholding), a yield-on-cost rate whose 總買入成本 denominator is hardcoded so later buys don't rewrite history, and — when the money arrives — a final amount plus a yield-on-price rate with another hardcoded price/shares snapshot. Migrating this section removes the hand-typed snapshot numbers, which are the most error-prone part, because the app can derive them from stored trades.

## What Changes

- New `dividends` table: one row per dividend event per stock, with `pay_date`, optional `per_share`, frozen `shares_held`/`buy_cost` snapshots, `estimated_amount`, `received_amount`/`received_price`, and a note. A row is pending until `received_amount` is set.
- On create, the server derives `shares_held` (ΣBUY − ΣSELL shares) and `buy_cost` (Σ BUY total) from trades with `trade_date <= pay_date`, unless the caller supplies them — reproducing the sheet's hardcoded snapshot denominators.
- Derived on read: effective amount (`COALESCE(received_amount, estimated_amount)`), yield on cost (`amount ÷ buy_cost`, the sheet's L column), yield on price (`amount ÷ (received_price × shares_held)`, the sheet's N column), estimate variance, and status.
- HTTP API: `GET/POST /api/dividends` (market/status/year/order filters), `PATCH/DELETE /api/dividends/:id`, `GET /api/dividends/summary?market=` (pending list plus per-year and per-stock received rollups).
- Receiving a dividend is a PATCH setting `received_amount` (+ `received_price`); a `refresh_snapshots` flag re-derives the snapshots when the pay date was edited.
- Market-scoped 派息 tab in the frontend (HK/US toggle like 交易記錄): record form with `per_share × shares` preview, pending list with a 收訖 action (with a `同時更新現價` option that also updates the stock's 現價), and received history with year filter and rollups.
- `import_xlsx` imports the existing J–O dividend block from both trade sheets (cached values; buy_cost recovered from `M ÷ L`, receipt price from `M ÷ (N × O)`, estimate formula text kept as the note), idempotently.
- `check_parity` gains a light dividend section comparing row counts and Σ amounts per market.
- Deleting a stock that has dividend records is refused, like stocks with trades.
- Not in this change: Month Stat / Overview / 回報率 integration (the yearly return-rate sheets still read the workbook), MPF, 債券, AIA, price auto-fetch.

## Capabilities

### New Capabilities

- `stock-dividends`: recording, listing, editing, receiving and deleting per-stock dividend events with point-in-time holdings/cost/price snapshots, yield-on-cost and yield-on-price rates, and pending/received lifecycle.

### Modified Capabilities

- `spreadsheet-trade-import`: the importer now also reads the trade sheets' J–O dividend block into the new registry, and the parity check reports dividend totals per market.

## Impact

- `backend/migrations/0005_dividends.sql`, `models.rs`, `calc.rs`, new `routes/dividends.rs`, `routes/mod.rs` wiring, and the stock delete guard in `routes/stocks.rs`.
- `xlsx.rs` parses the J–O block (including Excel serial dates); `import.rs` inserts dividends idempotently; `parity.rs` gains a dividend section.
- `frontend/src/api.ts`, new `DividendForm.vue`/`DividendTable.vue`/`DividendsView.vue`, and a 派息 tab in `App.vue`.
- `財富分析報告.xlsx` remains read-only; its J–O block stays untouched and can keep being maintained until this change is verified.
