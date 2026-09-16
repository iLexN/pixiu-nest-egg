## 1. StockTable component

- [x] 1.1 Create `frontend/src/components/StockTable.vue` with `stocks: Stock[]` prop and `saved`/`remove`/`error` emits, moving the 11-column table markup from `StocksView.vue`; verify the table renders rows identically to before (vue-tsc passes).
- [x] 1.2 Add inline editing state and logic to `StockTable.vue` (`editingId`, string-valued `draft` for the ten stock fields, `rowError`, `saving`, `num()`/`optional()`/`optionalNumber()` helpers, `startEdit`/`cancelEdit`/`saveEdit` calling `api.updateStock`); verify by typecheck that `draft` covers all editable stock fields.
- [x] 1.3 Render the `.editing` `<tr>` (input per cell, `step="any"` on numbers, 儲存/取消 `.link` buttons disabled while `saving`) plus a `colspan="11"` error `<tr>` when `rowError` is set; verify `frontend/src/components/StockTable.vue` compiles via `pnpm exec vue-tsc --noEmit`.
- [x] 1.4 Add scoped styles (`.editing input` ~5rem width, `.note`, `.row-actions`); verify the editing row fits the existing card layout.

## 2. StocksView simplification

- [x] 2.1 Remove `editing` ref, `startEdit`, the update branch of `save()`, and the 取消 button from `frontend/src/views/StocksView.vue`; retitle the form to 新增-only and verify `save()` only calls `api.createStock`.
- [x] 2.2 Replace the inline table with `<StockTable :stocks="stocks" @saved="onSaved" @remove="remove" @error="onError" />` and add `onSaved` (`已更新 <code>` + reload) and `onError` handlers mirroring `TradesView`; verify `import StockTable` resolves and the template compiles.
- [x] 2.3 Move the `message`/`error` paragraphs into the table card above the table; verify feedback text renders there for add, edit, and delete actions.

## 3. Docs and verification

- [x] 3.1 Update `docs/DATA_FLOW.md` "Update stock metadata in 股票管理" to name `StockTable` inline editor instead of `StocksView` edit form; verify the flow text matches the "Edit a trade inline" wording.
- [x] 3.2 Run `cd frontend && pnpm exec vue-tsc --noEmit` and `pnpm build`; verify both succeed with no warnings.
- [x] 3.3 Manual check: in 股票管理 for HK and US, click 編輯 on a row and confirm inputs appear inline; 儲存 persists and reloads; 取消 restores the read-only row; clearing 股票代碼 shows the inline error row; 刪除 still prompts and removes; adding via the top form works before, during, and after an inline edit.
