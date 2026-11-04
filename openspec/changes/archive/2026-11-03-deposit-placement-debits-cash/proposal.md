# Proposal

## Why

Placing a 定期 is a single real-world event: money leaves a 活期 account and gets locked in a deposit. The app records it differently depending on the entry point:

- 新增定期 records the start month's `dep-start` 調整 item automatically.
- 轉為定期 (converting a 預測 plan) skips that item. The month shows a 建議 the user must accept by hand, which is what happened to deposit `0089`.
- Neither entry point takes the principal out of HS/SC. Until the user edits the balance, live 流動資產 counts the principal twice: once in the 活期 balance and once in 已定期.

收訖 already moves cash back automatically. Placement should do the mirror-image move.

## What Changes

- 新增定期 and 轉為定期 share one placement rule. In one write it does three things:
  - inserts the deposit;
  - optionally debits the principal from a chosen HKD bank account;
  - records the `dep-start:<id>` adjustment item.
- `POST /api/deposits` and `POST /api/forecast-items/{id}/convert` accept an optional `debit_account_id`. When it is absent, nothing is debited. It must name an HKD bank account holding at least the principal, else 400.
- A deposit that is already received when created (past `end_date`) never debits.
- The deposit stores the account it debited and the amount. Deleting an unreceived deposit adds that amount back to that account (skipped when the account is gone), alongside the existing dep-start cleanup. Editing a deposit does not touch the debit.
- The 新增定期 / 轉為定期 form offers a 扣自 choice: the HKD bank accounts plus 不扣活期. It is preselected from the deposit's bank (`HS` → HS, `SC` → 渣打, else the HKD default account).
- The 定期 start-step reminder describes the automatic debit.
- Existing deposits are not backfilled; their debit fields stay empty.

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `time-deposits`: creation records the debit as well as the dep-start item; deletion reverses the debit; the reminder text changes.
- `overview`: "Convert plan to deposit" follows the same placement rule as `POST /api/deposits`.
- `cash-accounts`: deleting a bank account also clears deposits' recorded debit account, as it already clears receipt credits.

## Impact

- Backend:
  - new migration `0035` adding `deposits.debited_account_id` / `debited_amount`;
  - `models::deposits` (`NewDeposit.debit_account_id`; `Deposit` exposes the debit fields);
  - `service::deposits` (shared placement helper, new `convert_plan`, delete reversal);
  - `routes::forecast::convert` (calls the service);
  - `store::deposits`, and `store::cash_accounts::delete`.
- Frontend: `api.ts` types, `DepositForm.vue` (扣自 select), `DepositsView.vue` reminder text.
- Docs: `docs/deposits.md`, `docs/overview.md`, `docs/database.md`.
- API compatibility: additive only. Clients that omit `debit_account_id` keep today's no-debit behavior.
