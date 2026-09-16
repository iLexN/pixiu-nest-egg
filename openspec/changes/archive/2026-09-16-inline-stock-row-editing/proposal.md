## Why

In 股票管理 (`StocksView.vue`), clicking 編輯 fills the shared form at the top of the page, while 交易記錄 (`TradeTable.vue`) and 定期記錄 (`DepositTable.vue`) edit the row in place. The inconsistent interaction makes stock editing harder to discover and forces scrolling between the row and the form.

## What Changes

- Extract the stock list table from `frontend/src/views/StocksView.vue` into a new `frontend/src/components/StockTable.vue` that follows the `TradeTable`/`DepositTable` pattern: props in, `saved`/`remove`/`error` events out, inline editing state (`editingId`, `draft`, `rowError`, `saving`), an `.editing` row with an input per cell plus 儲存/取消 buttons, and an inline error row.
- Reduce the form in `StocksView.vue` to add-only: remove the `editing` state, `startEdit`, the update branch of `save()`, and the 取消 button; handle the table's `saved`/`remove`/`error` events with the same `onSaved`/`onError` pattern as `TradesView`.
- Update `docs/DATA_FLOW.md` so the 股票管理 update flow names the inline editor.

No API, persistence, or calculation behavior changes.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None — this is a frontend presentation refactor. The stock create/update/delete capability and its validation already exist under `stock-trades`, and the specs describe system behavior, not where the edit surface renders. `skip_specs: true` is set in `.openspec.yaml`.

## Impact

- `frontend/src/components/StockTable.vue` (new) — stock table with inline row editing.
- `frontend/src/views/StocksView.vue` — add-only form plus `<StockTable>`; edit state moves into the component.
- `docs/DATA_FLOW.md` — one-line update to the "Update stock metadata in 股票管理" flow.
- No backend, API, or database impact.
