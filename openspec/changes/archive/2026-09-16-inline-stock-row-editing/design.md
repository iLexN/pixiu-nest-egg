## Context

`StocksView.vue` currently owns everything: the stock list `<table>`, a single reactive `form` used for both 新增 and 編輯, and `editing: Stock | null` state. `startEdit(stock)` copies the row into the top form, which retitles itself `編輯 <code>` and switches its submit button to 儲存.

`TradeTable.vue` and `DepositTable.vue` instead own inline editing inside the table component: `editingId`, a string-valued `draft`, `rowError`, and `saving`; the editing `<tr>` renders inputs per cell with 儲存/取消 `.link` buttons, plus a full-width error `<tr>`; the component calls `api.update*` itself and emits `saved`/`remove`/`error` to the parent view, which reloads and shows a message.

## Goals / Non-Goals

**Goals:**

- Row editing in 股票管理 looks and behaves like 交易記錄 and 定期記錄: inputs appear in the row being edited.
- Keep the established component split: table component owns row interactions; the view owns data loading, the add form, delete confirmation, and feedback messages.
- `api.updateStock` receives the same payload shape it does today (trimmed `code`, empty optionals → `null`, parsed numbers).

**Non-Goals:**

- No backend, API, or spec changes.
- No change to add-stock fields, validation rules, or delete confirmation wording.
- Not restyling the read-only table or changing column order.

## Decisions

- **Extract `frontend/src/components/StockTable.vue` instead of keeping everything in `StocksView.vue`.** Both `TradeTable` and `DepositTable` already separate table components from their views; a third table-in-view would keep StocksView divergent structurally even after the interaction matches. Alternative considered: add `editingId`/`draft` directly inside `StocksView` — less code churn, but leaves an ~400-line view owning three concerns and misses the point of the inconsistency report.
- **Copy the shared editing pattern rather than abstracting it.** `TradeTable` and `DepositTable` each carry their own `editingId`/`draft`/`num()`/`startEdit`/`cancelEdit`/`saveEdit` code; `StockTable` does the same (a `Draft` with all-string fields mirroring the stock form's ten fields). Extracting a composable now would refactor two untouched components for no behavioral gain.
- **Move `message`/`error` display into the table card above the table**, matching `TradesView`, so add, edit, and delete feedback all render in one place near the list. The add form keeps submitting with its own button; it no longer needs the 取消 button since it never edits.
- **Market switch cancels any in-progress edit implicitly**: `StocksView` reloads `stocks` on `market` change and the row being edited simply reverts to read-only, same as `TradeTable` behaves when its `trades` prop reloads.

## Risks / Trade-offs

- [11 editable columns make the editing row cramped] → Same trade-off `DepositTable` already accepts; set narrow `.editing input` widths (~5rem) and keep `row-actions` `white-space: nowrap`.
- [`code` editable inline could be cleared, causing a validation error] → Shown in the inline error row via `rowError`, same failure surface as the top form today; backend validation is unchanged.
- [User mid-edit when list reloads (market switch, delete elsewhere)] → `editingId` may point at a row that re-renders; `cancelEdit` on next interaction is sufficient since reload replaces `stocks` and the editing row just falls back to read-only display.
