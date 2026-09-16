## Why

Inline row editing (currently in `TradeTable.vue`, `DepositTable.vue`, `StockTable.vue`) couples "editable" to "is a table column" — a field without a column becomes un-editable (e.g. `is_active` is already patchable via the API but has no UI) — and it duplicates each entity's field markup plus a cut-down preview inside every table component.

## What Changes

- Each entity's form becomes the single editing surface for both 新增 and 編輯:
  - New `StockForm.vue` extracted from `StocksView.vue`, with an edit mode.
  - `TradeForm.vue` and `DepositForm.vue` gain an edit mode (`editing` prop, `cancelled` emit, 儲存/取消 actions, per-field errors and live preview reused as-is).
- `StockTable.vue`, `TradeTable.vue`, `DepositTable.vue` lose all inline-editing machinery (`editingId`, `draft`, `rowError`, `saving`, editing/error rows) and emit `edit`/`remove` from read-only rows.
- Views wire `editing` state between table and form: `StocksView`, `TradesView`, `DepositHistoryView`, and `DepositsView` (which gains `DepositForm`, shown while editing an upcoming deposit).
- `docs/DATA_FLOW.md` edit flows updated to name the forms instead of inline editors.

No API, persistence, or calculation behavior changes.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None — this is a frontend presentation refactor. The create/update/delete capabilities and their validation are unchanged; specs describe system behavior, not where the edit surface renders. `skip_specs: true` is set in `.openspec.yaml`.

## Impact

- New: `frontend/src/components/StockForm.vue`.
- Modified: `StockTable.vue`, `TradeTable.vue`, `DepositTable.vue`, `TradeForm.vue`, `DepositForm.vue`, `StocksView.vue`, `TradesView.vue`, `DepositsView.vue`, `DepositHistoryView.vue`, `docs/DATA_FLOW.md`.
- Net effect: removes ~300 lines of duplicated draft/preview code; each form gains edit-mode wiring.
- No backend, API, or database impact.
