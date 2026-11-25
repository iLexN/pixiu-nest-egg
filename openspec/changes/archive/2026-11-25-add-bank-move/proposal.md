# Proposal

## Why

Moving money between the two 活期 bank accounts (HS → 渣打, e.g. to fund an SC 定期, or back) today takes two hand edits on 總覽: lower one row with 編輯, raise the other with 編輯. Two separate writes can disagree (a typo, or the second edit forgotten) and silently change the 活期 total, which feeds 月初 and every live total. One action that moves the amount between the accounts atomically removes both risks.

## What Changes

- New **活期調撥**: `POST /api/cash-accounts/moves` with `from_account_id`, `to_account_id`, `amount` subtracts `amount` from the source and adds it to the destination in one transaction, responding `200` with both updated accounts.
- Validation: `amount` finite and `> 0`; both accounts exist (else `404`), are HKD `bank` accounts (IBKR broker accounts are rejected on either side), and differ; a move larger than the source balance is rejected and changes nothing.
- Nothing is recorded: no log table, no history, no delete endpoint, no 月結 item, no 預測 line. The 活期 total is unchanged by a 調撥, so 月初, 總數/流動資產, 年結, and 預測 are unaffected. A mistaken 調撥 is undone by a 調撥 the other way.
- No fee field: the source loses exactly what the destination gains. A bank fee, if any, is corrected with the existing 編輯.
- 總覽 半流動資金 block: a 「調撥」 action on the 活期 row opens an inline form (from / to / amount) that saves through the new endpoint and reloads the view. The per-row 編輯 (direct balance set via `PATCH /api/cash-accounts/{id}`) is unchanged.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `cash-accounts`: the Cash account API gains `POST /api/cash-accounts/moves`, with its validation and atomicity rules.
- `overview`: the 總覽 半流動資金 block gains the 「調撥」 form beside the existing inline cash-row editing.

## Impact

- Backend: `models/cash.rs` (`NewBankMove`), `service/cash_accounts.rs` (`bank_move`, reusing `service::cash_movement::debit_bank`), `routes/cash_accounts.rs` (handler with `#[utoipa::path]`), `routes/mod.rs` (`api_router` registration and the `openapi_documents_every_api_operation` path list). No migration.
- Frontend: `src/api-types.ts` (regenerated), `src/api.ts` (`NewBankMove` alias, `api.bankMove`), `src/views/OverviewView.vue` (調撥 form).
- Docs: `GLOSSARY.md` (new term 活期調撥), `docs/adr/0010-cash-movement-semantics.md` (amendment), `docs/overview.md` (半流動資金 wiring).
- Unchanged: `PATCH /api/cash-accounts/{id}`, IBKR 轉入, deposit placement debits, receipt credits, and every month/forecast/year-review figure.
