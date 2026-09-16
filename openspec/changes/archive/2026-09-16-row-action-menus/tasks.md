## 1. Shared RowActions component

- [x] 1.1 Create `frontend/src/components/RowActions.vue`: a ⋯ toggle button inside a `position: relative` wrapper that opens an absolutely positioned menu with 編輯 and 刪除 items emitting `edit`/`remove`; verify `pnpm exec vue-tsc --noEmit` passes.
- [x] 1.2 Add dismissal: fixed transparent backdrop closing on any outside click, a document `keydown` listener closing on Escape, and listener cleanup in `onBeforeUnmount`; verify the menu closes on option pick, outside click, and Escape.

## 2. Adopt in the three tables

- [x] 2.1 In `TradeTable.vue`, replace the two link buttons in `.row-actions` with `<RowActions @edit="emit('edit', trade)" @remove="emit('remove', trade)" />`; verify `vue-tsc` passes.
- [x] 2.2 In `StockTable.vue`, do the same passing `stock`; verify `vue-tsc` passes.
- [x] 2.3 In `DepositTable.vue`, do the same passing `deposit`; verify `vue-tsc` passes.

## 3. Docs and verification

- [x] 3.1 Update `docs/DATA_FLOW.md` lines describing edit mode entry ("via 編輯 on a …Table row") to mention opening the row's ⋯ menu; verify wording matches the new UI.
- [x] 3.2 Run `cd frontend && pnpm exec vue-tsc --noEmit && pnpm build`; verify both succeed with no warnings.
- [x] 3.3 Manual check: on 交易記錄, 股票管理, and 定期記錄, each row shows only ⋯; clicking it opens a menu with 編輯/刪除; picking an option triggers the existing flow; outside click, second ⋯ click, and Escape close the menu; opening another row's menu closes the first.
