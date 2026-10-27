# Proposal

## Why

The owner regularly re-types a trade's name, share count, price, and date into other forms and documents. A one-click copy of a fixed, labeled summary from the 交易記錄 list removes that manual transcription and the typos that come with it.

## What Changes

- Add a `複製` action to each row's `⋯` menu in 股票 → 交易記錄, placed directly above `編輯`.
- The action copies a four-line plain-text summary of the trade to the clipboard. Each line is a label ending in `：`, a tab, and the value:
  - BUY: `資產名稱/代號`, `購買股數`, `買入價格`, `買入日期`
  - SELL: `資產名稱/代號`, `賣出股數`, `賣出價格`, `賣出日期`
- Values are the stored raw values:
  - code
  - 股數, with no thousands separator
  - `$` + the entered 單價, not rounded and not the fee-included 平均單價
  - 日期 as `YYYY-MM-DD`
- Brief success feedback is shown, and a visible error if the clipboard write fails.
- Other tables that share the row menu are unchanged.

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `stock-trades`: adds a requirement for copying a single trade's labeled summary from the trade history.

## Impact

- Frontend only:
  - `frontend/src/components/RowActions.vue`: optional `複製` menu item.
  - `frontend/src/components/TradeTable.vue`: builds the text and writes it to the clipboard.
- Docs: `docs/stocks.md` gets a "Copy a trade" flow.
- No backend, API, schema, or dependency changes.
