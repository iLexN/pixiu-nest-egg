# Proposal

## Why

The 收訖 form prefills 存入金額 with the deposit's total when it opens and always submits that number. Correcting 實收利息 therefore leaves the cash credit stale, and it then disagrees with the dep-end month item, which uses the corrected interest. The user never credits a different amount, so the override only causes errors.

## What Changes

- **BREAKING (API):** `POST /api/deposits/:id/receive` no longer accepts `credit_amount`. When a cash row is chosen, the credit is always `principal + interest`, using the corrected interest when one is sent. This is the same amount the dep-end item records.
- The 收訖 form drops the 存入金額 input. It shows the amount that will be credited as read-only text.
- Bonds and family deposits are unchanged.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `time-deposits`: "Deposit 收訖 lifecycle". The credited amount is fixed at principal + (corrected) interest and is no longer user-supplied.

## Impact

- Backend: `routes/deposits.rs` (`ReceiveDeposit`, `receive`, tests). The OpenAPI schema loses the field.
- Frontend: `api.ts` `ReceiveDepositBody`, `views/DepositsView.vue`.
- Docs: `docs/deposits.md`.
- No migration. `credited_amount` stays stored for 取消收訖.
