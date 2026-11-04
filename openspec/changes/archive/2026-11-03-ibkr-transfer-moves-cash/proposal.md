# Proposal

## Why

Recording an IBKR 轉入 today only appends a row to the `ibkr_transfers` log, so every transfer needs three more hand edits: lowering the HS bank balance, raising IBKR HKD 現金, adding a 月結 `調整`, and adding a 預測 → 股票 line (the October transfer missed the last one). An HK BUY already flows into 月結 and 預測 on its own; a transfer should be just as complete, and because it moves real cash it must be undoable exactly.

## What Changes

- New `POST /api/ibkr/transfers` records a bank→IBKR transfer as a real money movement in one transaction: appends the log row, debits the source HKD bank account (default: the bank account labelled `HS`), credits the IBKR HKD broker account, and auto-records a 月結 `adjustment` item (`auto_key` `ibkr:<id>`, label `IBKR 轉入`, `−amount`).
- Validation: amount finite and `> 0` (withdrawals are out of scope for now); `transfer_date` `YYYY-MM-DD`, not after today, defaulting to today; the source must be an HKD `bank` account; a transfer larger than the source balance is rejected.
- New `GET /api/ibkr/transfers` lists the log (newest first, with the source account label); new `DELETE /api/ibkr/transfers/{id}` exactly reverses a transfer's balances and its 月結 item, then deletes the row.
- 月結: when the month row does not exist yet, an `ibkr:<id>` candidate is suggested instead (same fallback as deposits).
- 預測: transfers dated in the window appear as read-only `ibkr_lines` in the `股票` row and reduce `cash` in their month, like HK trade lines.
- **BREAKING**: `PATCH /api/ibkr` no longer accepts `transfer_hkd` / `transfer_date`; it edits only `now_value`, `hkd_cash`, `usd_cash`.
- IBKR card: the 編輯 form drops the 轉入 inputs; a separate 「轉入」 form (amount, date, source account preselecting `HS`, balance preview) replaces them. The card shows no transfer history; a new 美股-only 股票 → 轉入記錄 tab lists the log with a 累計 total and a confirmed 刪除 per row.
- The two existing log rows (2026-06-02 131000, 2026-10-30 90000) stay log-only: no account link, so deleting them moves no cash, and the manual October `to IBKR` item is untouched.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `overview`: IBKR transfer entry becomes a real cash movement with its own create/list/delete endpoints; `PATCH /api/ibkr` loses the transfer fields; the IBKR card gains a 轉入 form; the forecast gains `ibkr_lines` in the 股票 row and `cash`.
- `stock-portfolio-summary`: the 美股 IBKR card's editable inputs change (轉入 moves to its own form).
- `app-navigation`: the 股票 group gains a 轉入記錄 sub-tab shown only while the market is 美股.
- `month-stat`: a transfer auto-records its `ibkr:<id>` 調整, with a candidate fallback; the "IBKR transfer stays manual" scenario is replaced.

## Impact

- Backend: new migration `0034_ibkr_transfer_accounts.sql` (`from_account_id`, `broker_account_id` on `ibkr_transfers`); `store/ibkr.rs`, `store/cash_accounts.rs`, `models/overview.rs`, `models/forecast.rs`, `service/overview.rs` (or a new `service/ibkr.rs`), `service/months.rs`, `calc/suggestions.rs`, `calc/forecast.rs`, `routes/overview.rs`, `routes/forecast.rs`, `routes/mod.rs` (OpenAPI path list).
- Frontend: `src/api.ts`, `src/components/IbkrCard.vue`, `src/views/OverviewView.vue` (股票 cell).
- Docs: `docs/overview.md`, `docs/database.md`, `docs/DATA_FLOW.md` where the IBKR / 月結 / 預測 wiring is described.
- Unchanged: 累計轉入, 淨額/回報率, the year review's IBKR 轉入 (all `SUM(ibkr_transfers)`), and 總資產/流動資產 (bank cash and IBKR HKD cash are both counted).
