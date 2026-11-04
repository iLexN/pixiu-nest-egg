# Design

## Context

See proposal.md — Why. Current wiring:

- `ibkr_transfers` (migration 0022) is a bare `(transfer_date, amount_hkd, created_at)` log; `service::overview::update_ibkr` appends to it from `PATCH /api/ibkr` in the same transaction that `set_amount`s the broker cash rows — so an inline transfer and the prefilled `hkd_cash` would race, the later `set_amount` wiping any credit.
- Cash movement already has a shared pattern: `service/receipts.rs` credits an account with `cash_accounts::add_amount` and stores the credit so `reverse` can undo it exactly, skipping a deleted account.
- 月結 auto items already have a shared pattern: `store::months::record_receipt_item` inserts/refreshes a keyed `adjustment` when the month row exists (writes nothing otherwise), and `delete_auto_items_by_key` removes it plus dismissals in any month. Deposits use it for `dep-start:<id>`, and `calc::suggestions::build_suggestions` produces the same key as a candidate so a missing month row falls back to a suggestion.
- The forecast's real 股票 side is `trade_lines` → `trade_flows` → `cash` in `calc/forecast.rs` / `routes/forecast.rs`, rendered by `cellTradeLines` in `OverviewView.vue`.

## Goals / Non-Goals

**Goals:**
- One request performs the whole 轉入 atomically and one request undoes it exactly.
- Reuse the receipt, month-item, and forecast trade-line patterns rather than inventing parallel ones.

**Non-Goals:**
- Withdrawals (IBKR → bank), editing a transfer in place (delete + re-add instead), FX inside IBKR, and back-filling account links on legacy rows.
- Changing how HK trades interact with cash accounts.

## Decisions

1. **Dedicated `/api/ibkr/transfers` resource instead of a PATCH field.** A create/list/delete resource gives exact reversal and removes the `set_amount` overwrite hazard. Alternative — keep `transfer_hkd` on PATCH and send only dirty fields from the UI — leaves the hazard in the API and gives no undo. PATCH silently ignores unknown fields today (no `deny_unknown_fields`), so the removed fields become no-ops rather than errors; acceptable because the only client is updated in the same change.

2. **Record both account ids on the row** (`from_account_id`, `broker_account_id`, nullable, `ON DELETE SET NULL`) via migration `0034_ibkr_transfer_accounts.sql`. Reversal reads what was actually moved, not today's defaults — the same reason receipts store `credited_account_id`. NULL on both marks a log-only row (the two legacy rows), which deletion and the 月結 candidate rule both key off. Alternative — a boolean `moved_cash` — loses which account to reverse.

3. **Default source by label `HS`, not `is_default_receipt`.** User decision: the transfer source is a fixed habit, independent of where receipts bank in. Resolved server-side (the backend is authoritative) with a new `cash_accounts::bank_by_label`; the UI preselects the same account.

4. **Overdraft check inside the transaction.** Load the source with `load_one` on the transaction connection and compare `amount >= amount_hkd` before `add_amount`, so the check and the debit see the same row state. Exact comparison on `f64` (no epsilon), matching how balances are stored.

5. **No future dates.** The cash moves now, so a future `transfer_date` would put money in IBKR before its 月結/預測 month. Planned transfers remain `stock` forecast plan items.

6. **月結 item via `record_receipt_item` + candidate fallback.** Key `ibkr:<id>`, label `IBKR 轉入`, amount `−amount_hkd`, month = first of `transfer_date`'s month. `SuggestionEvents` gains `ibkr_transfers` loaded by `service::months::load_events` for the month range, restricted to rows with a non-NULL `from_account_id` so legacy rows (already covered by manual items) never suggest. Suggestions remain gated to the current month and later by the existing `suggestions_enabled`.

7. **Forecast: separate `ibkr_lines`, not folded into `trade_lines`.** `ForecastTrade` carries `trade_id`/`code`/`trade_type`, which a transfer doesn't have; a separate `ForecastIbkrTransfer { transfer_id, transfer_date, amount }` keeps both shapes honest. `ForecastInput` gains `ibkr_transfers`, `ForecastMonthDerived` gains `transfer_flows`, added into the `cash` chain only (not `locked`). All log rows in the window count, including log-only ones — a transfer reduced 活期 in its month either way; in practice none of the legacy rows is in the window.

8. **Service placement.** New `service/ibkr.rs` holds `create_transfer` / `delete_transfer` (composes `store::ibkr`, `store::cash_accounts`, `store::months`); `service/overview.rs` keeps `ibkr_block` and the trimmed `update_ibkr`. Routes stay thin adapters in `routes/overview.rs`, registered with `utoipa_axum::routes!`.

9. **Frontend refresh.** `IbkrCard` refetches `api.ibkr()` after a transfer create and emits `saved` with the fresh block; `OverviewView` already reloads everything on `saved`, `SummaryView` replaces its block. The card shows no transfer history (user decision); the history lives in a new `IbkrTransfersView` on a 美股-only 股票 → 轉入記錄 tab. `App.vue`'s `NAV` tab list for the 股票 group is filtered by market, and a `watch` on `market` sends the user to 股票 → 總覽 when 港股 is selected while 轉入記錄 is open — so the existing "tab always belongs to some group" lookup never sees a hidden tab.

## Risks / Trade-offs

- [A stale HS balance blocks a real transfer] → intended; the 400 message names the account and its current amount so the user can correct HS first.
- [Reversal can drive IBKR HKD negative if the cash was already converted to USD] → accepted, same rule as receipt reversals; the user fixes cash via 編輯.
- [Plan item and real line both count in 預測] → consistent with HK trades; documented in the spec scenario.
- [Old clients sending `transfer_hkd` get silently ignored] → only our frontend, updated together.

## Migration Plan

- `0034` adds two nullable FK columns; existing rows stay NULL. Applied automatically on startup; no data rewrite.
- Rollback: revert the code; the extra columns are harmless to the old code (it never selects them). Never edit `0034` once applied.
