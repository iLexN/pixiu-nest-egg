# Proposal

## Why

The 取消收訖 (unreceive) actions on deposits, family deposits, and bonds are never used. Removing them entirely — UI affordances, API endpoints, spec requirements, and docs — keeps the spec as the honest source of truth instead of carrying a live-but-unused capability.

## What Changes

- **BREAKING** Remove `POST /api/deposits/:id/unreceive`, `POST /api/family/deposits/:id/unreceive`, and `POST /api/bonds/:id/unreceive` along with their route handlers.
- Remove the 取消收訖 buttons and `unreceive` handlers from the deposit history, family deposits, and bonds views; remove `unreceiveDeposit`/`unreceiveFamilyDeposit`/`unreceiveBond` from the frontend API client.
- Remove the now-dead store helpers `credited_of`/`clear_receipt` (deposits, bonds) and `clear_receipt` (family) that exist solely to serve unreceive.
- Trim the unreceive assertions from `backend/tests/api.rs`; keep the re-receive 409 checks.
- Reword the `credited_asset_id`/`credited_amount` doc text — the columns stay as provenance of where a 收訖 bank-in went; no migration.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `time-deposits`: the 收訖 lifecycle requirement drops the unreceive endpoint and its reversal scenario.
- `family-deposits`: the 收訖 lifecycle requirement drops the unreceive action and its scenario.
- `bonds`: the 收訖 lifecycle requirement drops the unreceive endpoint, its scenario, and the received-row 取消收訖 action.

## Impact

- Backend: `routes/deposits.rs`, `routes/family.rs`, `routes/bonds.rs`, `routes/mod.rs` (registrations + openapi pin list), `store/deposits.rs`, `store/family.rs`, `store/bonds.rs`, `tests/api.rs`.
- Frontend: `api.ts`, `components/DepositTable.vue`, `views/DepositHistoryView.vue`, `views/FamilyDepositsView.vue`, `views/BondsView.vue`.
- Docs: `docs/deposits.md`, `docs/investments.md`, `docs/database.md`, `docs/DATA_FLOW.md`.
- Unchanged by design: coupon `received_amount` clearing and dividend edit reversal still undo their own bank-ins (field-edit semantics, not a standalone undo); `credited_asset_id`/`credited_amount` keep being written on 收訖 as provenance.
- Consequence: a mistaken 收訖 on a deposit/family deposit/bond no longer has an in-app undo; fixing it means editing the affected cash row and month item manually, or delete + recreate.
