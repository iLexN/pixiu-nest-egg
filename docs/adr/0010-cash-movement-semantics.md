# Which domain events move 活期 cash

Cash movement is asymmetric by design — the map every cash-touching change must follow:

- **收訖 credits.** Marking a deposit, bond coupon, or HK dividend received adds the amount to a cash account; `credited_account_id`/`credited_amount` are stored so reversal is exact (a deleted account is skipped).
  - Amendment (2026-11-12): US dividends credit too — into the USD (IBKR) broker account, never 活期, and never counted in month 利息.
- **Deposit placement debits.** Creating a 定期 with a `start_date` debits `debit_account_id` — explicit, no server default (UI preselects bank→account by label); delete reverses the stored debit; PATCH never re-syncs it; auto-received inserts (past `end_date`) never debit.
- **IBKR 轉入 moves bank→broker** atomically (`/api/ibkr/transfers` resource, not a PATCH field), records both account ids on the row, checks overdraft inside the transaction, and rejects future dates — the cash moves now. Log-only rows (NULL account ids) move nothing.
- **HK trades never move cash.** They only appear as month 調整 candidates — cash accounting stays manual there.
- **US trades settle inside IBKR.** Broker cash is the settlement account; no bank touch — which is also why US dividends are never counted in month 利息.
- **家人 deposits move nothing.** Record-only.

New money-moving behavior must reuse `service::receipts::credit`/`reverse` (credit pattern), `record_receipt_item` + `auto_key` (month item), and the transfer's in-transaction account-id recording (exact reversal) — not invent parallel paths.

Source: `openspec/changes/archive/2026-10-20-consolidate-cash-fx-settings`, `2026-11-03-deposit-placement-debits-cash`, `2026-11-03-ibkr-transfer-moves-cash`, `2026-11-05-us-trade-cash-settlement`.
