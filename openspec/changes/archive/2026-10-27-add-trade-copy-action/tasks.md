# Tasks

## 1. Row menu copy item

- [x] 1.1 In `frontend/src/components/RowActions.vue`, add an optional `copyable?: boolean` prop and a `copy` emit. When `copyable` is set, render a `複製` link button immediately before `編輯`, routed through `pick('copy')` so the menu closes. Verify that `pnpm exec vue-tsc --noEmit` passes and that other RowActions users (StockTable, DividendTable, DepositTable, Bonds/AIA/MPF/FamilyDeposits views) show no 複製 item.

## 2. Trade summary copy

- [x] 2.1 In `frontend/src/components/TradeTable.vue`, add `copyTrade(trade)` to build the four-line text from the spec.
  - BUY rows use `購買股數`/`買入價格`/`買入日期`; SELL rows use `賣出股數`/`賣出價格`/`賣出日期`.
  - Each line is the label, then `：`, a tab, and the value. Lines are joined with `\n`.
  - Values are `trade.code`, `String(trade.shares)`, `'$' + String(trade.unit_price)`, and `trade.trade_date`. Do not use the `format.ts` helpers.
  - Write the text with `navigator.clipboard.writeText`.
  - Verify by copying the 2026-06-04 新華保險 BUY row: the pasted text is exactly the spec's four lines, with `$47.02`.
- [x] 2.2 Pass `copyable @copy="copyTrade(trade)"` to the row's `<RowActions>`. Add feedback following `HappyPoolView.copyRow`: a `copiedId` ref that shows `已複製` beside `⋯` for about 1.5 s, and an inline `複製失敗` on rejection. Verify that the feedback appears after copying, and that a SELL row copies with 賣出 labels.
- [x] 2.3 Add a "Copy a trade" flow to `docs/stocks.md` next to "Edit a trade". It is frontend only, the action is 複製 in the TradeTable row's ⋯ menu, it uses the clipboard, and it makes no API call. Verify that the doc renders and names the correct component.

## 3. Integration checks

- [x] 3.1 Run `cd frontend && pnpm build && pnpm exec vue-tsc --noEmit` and verify both succeed.
- [x] 3.2 Restart the backend per AGENTS.md (kill :8787, start detached with nohup) and verify that http://127.0.0.1:8787/ serves the new build with 複製 in a 交易記錄 row menu.

## Workflow follow-up

- Archive with `/openspec-archive-change`, syncing the delta into `openspec/specs/stock-trades/spec.md`.
