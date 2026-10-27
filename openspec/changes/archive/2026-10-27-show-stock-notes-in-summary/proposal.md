# Proposal

## Why

The 持倉總覽 table already receives each stock's 備註 (`note`) from `/api/summary`, but never shows it: the user must switch to 股票 > 管理 to read or edit a note. Notes with line breaks also cannot be entered comfortably, because the management form uses a single-line input.

## What Changes

- Each stock row in 股票 > 總覽 gets a `備註 ▸` / `備註 ▾` toggle that expands a full-width sub-row beneath it showing the full note (line breaks preserved).
- The expanded sub-row lets the user edit the note in place with a textarea; saving goes through the existing stock update endpoint (`PATCH /api/stocks/{id}`), and saving blank text clears the note.
- A `全部展開備註` / `全部收起備註` button expands or collapses every visible stock's note row at once.
- The note input in the 管理 form (`StockForm.vue`) becomes a textarea, and the 管理 table renders notes with preserved line breaks.
- Expanded state resets when the market tab changes. No backend changes.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `stock-portfolio-summary`: adds the requirement "Stock notes in the summary" — the summary table SHALL let the user expand a stock's row to show its full 備註 and edit it in place via the stock update endpoint.

## Impact

- `frontend/src/views/SummaryView.vue` — expandable note sub-rows, inline edit state, expand/collapse-all button, scoped styles.
- `frontend/src/components/StockForm.vue` — note field becomes a textarea.
- `frontend/src/components/StockTable.vue` — `.note` style gains `white-space: pre-wrap`.
- `docs/stocks.md` — one line noting notes are viewable/editable in 總覽 (if the file describes that UI).
- No backend, API, or database changes: `SummaryStock` already carries `note`, and `PATCH /api/stocks/{id}` already accepts it.
