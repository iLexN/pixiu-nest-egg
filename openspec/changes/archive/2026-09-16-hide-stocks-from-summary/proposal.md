## Why

A stock that has been fully sold cannot be deleted once it has trade or 派息 records (e.g. 香港寬頻), so it lingers in 持倉總覽 forever. The owner wants a way to hide such positions from the summary without losing their history.

## What Changes

- The existing `stocks.is_active` flag (already persisted, patchable via `PATCH /api/stocks/:id`, and serialized in both `GET /api/stocks` and `GET /api/summary`) becomes the hide flag.
- 股票管理 gains a 隱藏/顯示 row action; hidden stocks stay listed there, visually muted, so they can be unhidden.
- 持倉總覽 omits hidden stocks from the table but still counts them in market totals and sector rollups — hiding is display-only, matching the workbook where every row counts. Hidden stocks remain in the API response so reorder and parity are unaffected.
- A muted note in 持倉總覽 reports how many stocks are hidden.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `stock-trades`: the stock registry gains a hide/unhide action; hidden stocks remain listed and keep all records.
- `stock-portfolio-summary`: the summary table omits hidden stocks while still counting them in totals and rollups.

## Impact

- `frontend/src/components/RowActions.vue` — optional 隱藏/顯示 menu item.
- `frontend/src/components/StockTable.vue` — hide emit, muted styling for hidden rows.
- `frontend/src/views/StocksView.vue` — toggles `is_active` via `updateStock`.
- `frontend/src/views/SummaryView.vue` — filters hidden rows for display; hidden-count note.
- `docs/DATA_FLOW.md` — documents the hide action.
- No backend or schema changes: `is_active` is already stored and serialized.
