## Why

In 交易記錄 (trade history) there is no way to see how the current market price compares to what was actually paid per trade. The user must cross-reference 持倉總覽 manually to judge whether each BUY is above or below water at today's 現價.

## What Changes

- Add a `現價` column to the trade history table in 交易記錄, placed immediately after `平均單價（含 fee）`, showing the stock's stored 現價 for every row.
- For BUY rows, color the 現價 cell green when 現價 is higher than that row's 平均單價（含 fee）, and red when lower — the same green/red styling used by 未實現金額 in 持倉總覽. Equal values and missing data stay uncolored.
- SELL rows show the 現價 value without coloring.
- Frontend-only change: no API, schema, or stored-data changes.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `stock-trades`: the "Browsing trade history" requirement gains a per-row 現價 column that is sign-colored against the row's 平均單價（含 fee）for BUY trades.

## Impact

- `frontend/src/components/TradeTable.vue` — new column, price lookup from the already-loaded `stocks` prop, reuse of `signClass` and the global `.positive`/`.negative` classes.
- No backend, API, database, or import/parity impact.
