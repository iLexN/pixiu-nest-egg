# Tasks

## 1. Summary view note rows

- [x] 1.1 In `frontend/src/views/SummaryView.vue` add `expandedNotes` (Set of stock ids), `editingNote`, and `noteDraft` refs, reset them when `props.market` changes, and verify `vue-tsc --noEmit` reports no errors
- [x] 1.2 Add the per-row `備註 ▸`/`備註 ▾` toggle in the 股票 cell (muted when the stock has no note) and the `<template v-for>` note sub-row (`<tr class="note-row">` with empty first `<td>` plus `<td colspan="15">`, no drag handlers); verify `pnpm build` succeeds
- [x] 1.3 Implement read mode (`<p class="note-text">` plus 編輯備註/新增備註 link), edit mode (`<textarea v-model="noteDraft" rows="4">` with 儲存/取消), and `saveNote` calling `api.updateStock(id, { note: trimmed || null })` then `load()`, with errors routed to `error` like `savePrice`
- [x] 1.4 Add the 全部展開備註/全部收起備註 button in `.summary-actions` and the scoped styles (`.note-row td`, `.note-text`, `.note-row textarea`, `.note-toggle`)

## 2. Multi-line notes elsewhere

- [x] 2.1 Change the note field in `frontend/src/components/StockForm.vue` from `<input type="text">` to `<textarea rows="3">`
- [x] 2.2 Add `white-space: pre-wrap` to `.note` in `frontend/src/components/StockTable.vue`
- [x] 2.3 If `docs/stocks.md` describes the 總覽 UI, add one line noting notes are viewable and editable there via `PATCH /api/stocks/{id}`

## 3. Verification

- [x] 3.1 Run `openspec validate show-stock-notes-in-summary` and confirm it passes
- [x] 3.2 Run `cd frontend && pnpm build && pnpm exec vue-tsc --noEmit` and confirm both succeed
- [x] 3.3 Restart the backend on :8787 per AGENTS.md and confirm `curl -s 'http://127.0.0.1:8787/api/summary?market=HK'` returns JSON with `note`
