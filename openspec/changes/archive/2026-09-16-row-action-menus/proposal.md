## Why

Every row in the trade, stock, and deposit tables always shows 編輯 and 刪除 link buttons, but the owner edits or deletes rows only occasionally. The always-visible actions add visual noise to tables that are mostly read.

## What Changes

- Extract a shared `RowActions.vue` component that renders a compact ⋯ button per row; clicking it opens a small dropdown menu with the 編輯 and 刪除 options.
- The dropdown closes after picking an option, clicking outside the menu, or pressing Escape. Only one row's menu can be open at a time.
- Replace the inline 編輯/刪除 buttons in `TradeTable.vue`, `StockTable.vue`, and `DepositTable.vue` with the shared component; the existing `edit`/`remove` emits and view handlers are unchanged.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None — this is a frontend presentation change. How row actions are revealed is not spec-level behavior; the edit/delete requirements in `stock-trades` and `time-deposits` are unchanged. `skip_specs: true` is set in `.openspec.yaml`, matching the precedent in the `collapsible-add-forms` and archived `inline-stock-row-editing` changes.

## Impact

- `frontend/src/components/RowActions.vue` — new shared dropdown component.
- `frontend/src/components/TradeTable.vue`, `StockTable.vue`, `DepositTable.vue` — swap the two link buttons for `<RowActions>` in each row.
- `frontend/src/style.css` — minor additions for the menu container/items if not kept scoped in the component.
- `docs/DATA_FLOW.md` — minor wording updates to the edit/delete flows.
- No backend, API, or database impact.
