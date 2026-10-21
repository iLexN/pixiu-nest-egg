# Tasks

## 1. Copy action in the plan editor

- [x] 1.1 Add `copyItem(item)` in `frontend/src/views/OverviewView.vue` next to `saveItem`/`removeItem`: read `drafts.value[item.id]`, validate amount via `numOrNull` (error `金額必須是數字` when invalid), compute target `ym` as `item.month` year+1, call `api.createForecastItem(target, { kind: item.kind, amount, note: draft.note.trim() || undefined })`, set `message.value` to `` `已複製到 ${target}` ``, then `await load()`. Verify `pnpm exec vue-tsc --noEmit` passes.
- [x] 1.2 Add the `複製` button after the 刪除 button in the item row (~L740), rendered only when `selected.kind === 'tax'`, with `title="複製到明年同月"`. Verify via `pnpm build` that the frontend compiles.

## 2. Docs

- [x] 2.1 Update the editor flow block in `docs/overview.md` (~L62-66) to record that Tax rows offer 複製, which POSTs `{kind, amount, note}` to `/api/forecast/{ym}/items` for the same month next year and lands stored-but-invisible outside the window. Verify the text matches the implemented behavior.

## 3. Verification

- [x] 3.1 Run `cd frontend && pnpm build && pnpm exec vue-tsc --noEmit` — both succeed.
- [x] 3.2 Restart the backend (`lsof -ti :8787 | xargs kill`, then detached `nohup cargo run -p wealth-backend --bin wealth-backend`) and confirm `http://127.0.0.1:8787` answers.
- [x] 3.3 Manual check: 總覽 → click a Tax cell → 複製 → message `已複製到 YYYY-MM`; `sqlite3 data/wealth.db "SELECT * FROM forecast_items WHERE month LIKE '<next-year>%'"` shows the new item with same kind/amount/note and the source unchanged.

## Workflow follow-up

- Archive the change after review: `openspec archive copy-tax-forecast-item`.
