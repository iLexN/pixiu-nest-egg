## 1. Stocks

- [x] 1.1 Create `frontend/src/components/StockForm.vue`: extract the add form from `StocksView.vue` with props `market: Market`, `editing: Stock | null` and emits `saved`/`cancelled`; edit mode fills from `editing`, titles `編輯 <code>`, submits via `api.updateStock`, shows 取消. Verify `pnpm exec vue-tsc --noEmit` passes.
- [x] 1.2 Strip `frontend/src/components/StockTable.vue` to read-only rows emitting `edit`/`remove` (remove `editingId`/`draft`/`rowError`/`saving`/`startEdit`/`cancelEdit`/`saveEdit`, the `.editing`/error rows, and `saved`/`error` emits). Verify no editing state remains via typecheck.
- [x] 1.3 Update `frontend/src/views/StocksView.vue`: `editing` ref, `<StockForm :market :editing @saved @cancelled>`, table `@edit`; `onSaved` shows 已新增/已更新 and clears `editing`; clear `editing` on market switch. Verify add + edit both work via the form.
- [x] 1.4 Match the other views' form toggle: `showForm` ref + 新增股票 toggle button in `StocksView.vue`, `@edit` opens the form via `startEdit`, `onSaved` closes it; `immediate: true` on `StockForm`'s `editing` watcher since it now mounts via `v-if`. Verify toggle shows/hides the form and 編輯 opens it filled.

## 2. Trades

- [x] 2.1 Add edit mode to `frontend/src/components/TradeForm.vue` (`editing: Trade | null` prop, `cancelled` emit, fill from trade, `api.updateTrade` with `stock_id` resolved from `form.code` and `input_mode: editing.input_mode`, 編輯 title + 取消). Verify typecheck passes.
- [x] 2.2 Strip `frontend/src/components/TradeTable.vue` editing machinery (`editingId`/`draft`/`preview`/`num`/`startEdit`/`cancelEdit`/`saveEdit`/`saving`/`rowError`, editing + error rows, `saved`/`error` emits); emits become `edit`/`remove`/`toggle-order`; keep `totalCost` footer. Verify typecheck.
- [x] 2.3 Update `frontend/src/views/TradesView.vue`: `editing` ref, pass `:editing` to `TradeForm`, table `@edit`, clear `editing` on save/cancel/market switch. Verify edit a HK trade and a US trade derive fee/total correctly.

## 3. Deposits

- [x] 3.1 Add edit mode to `frontend/src/components/DepositForm.vue` (`editing: Deposit | null` prop, `cancelled` emit, fill with rate ×100, `api.updateDeposit`, 編輯 title + 取消). Verify typecheck.
- [x] 3.2 Strip `frontend/src/components/DepositTable.vue` editing machinery and move/remove the `bank-codes` datalist (already in `DepositForm`); emits become `edit`/`remove`; keep `showStatus`/`emptyText`/`columns`. Verify typecheck.
- [x] 3.3 Update `frontend/src/views/DepositsView.vue`: `editing` ref, render `<DepositForm v-if="editing" :editing @saved @cancelled>` at top of section, table `@edit`. Verify editing an upcoming deposit opens the form.
- [x] 3.4 Update `frontend/src/views/DepositHistoryView.vue`: `editing` ref, `@edit` sets `editing` + `showForm = true`, pass `:editing` to `DepositForm`, clear on save/cancel/year change. Verify editing a history row opens the form filled.

## 4. Docs and verification

- [x] 4.1 Update `docs/DATA_FLOW.md`: "Edit a trade inline" → `TradeForm` edit mode; deposit edit flow (~L349) → `DepositForm` edit mode; stock metadata flow → `StockForm`; fix the "Form + inline edit" wording in the summary table (~L428). Verify each flow's first line names the form.
- [x] 4.2 Run `cd frontend && pnpm exec vue-tsc --noEmit` and `pnpm build`; verify both succeed with no warnings.
- [x] 4.3 Manual check all three entities: 編輯 fills the top form; 儲存 persists and reloads; 取消 restores add mode; per-field errors appear on invalid input; deposit editing works from both 未到期 and 記錄 views; 刪除 still works everywhere.
