# Tasks

## 1. Storage

- [x] 1.1 Add `backend/migrations/0035_deposit_debit.sql` adding nullable `deposits.debited_account_id INTEGER REFERENCES cash_accounts(id) ON DELETE SET NULL` and `deposits.debited_amount REAL`; document both columns under `deposits` in `docs/database.md`. Verify the migration tests in `backend/src/db.rs` still pass (`cargo test -p wealth-backend db::`).
- [x] 1.2 Extend `store::deposits`:
  - `DEPOSIT_COLUMNS` / `row_to_deposit` read the two columns;
  - `models::Deposit` exposes `debited_account_id` / `debited_amount`;
  - add `set_debit(conn, id, account_id, amount)` and a loader for the stored debit.

  `models::NewDeposit` gains `debit_account_id: Option<i64>`, with serde default and utoipa docs. Verify `cargo build` and the existing deposit tests pass.
- [x] 1.3 `store::cash_accounts::delete` also clears `deposits.debited_account_id` for the deleted account. Extend `delete_refuses_broker_rows_and_nulls_credited_ids` (or add a sibling test) asserting the debited id is cleared and `debited_amount` kept.

## 2. Placement service

- [x] 2.1 In `service::deposits`, extract the tx helper `place` (sort order → insert → optional debit → dep-start item), with `create` wrapping it. The debit:
  - validates an HKD `bank` account (400 on `debit_account_id` otherwise; unknown id → 404);
  - rejects when the balance is below the principal, with the IBKR wording;
  - skips when principal ≤ 0 or the row was auto-received;
  - subtracts the principal and stores the debit.

  Tests in `service/deposits.rs`:
  - debit applied and stored;
  - no `debit_account_id` → no change;
  - overdraft → error, and no deposit/item/balance change;
  - broker or USD account → error;
  - past `end_date` → no debit.

  Verify with `cargo test -p wealth-backend service::deposits`.
- [x] 2.2 Add `service::deposits::convert_plan(pool, item_id, deposit, body)`: `place`, then `store::forecast::delete_item_in_tx`, in one transaction. Verify with the tests in task 3.1.
- [x] 2.3 `service::deposits::delete` reverses the stored debit (adds `debited_amount` back to `debited_account_id`; no-op when the account is gone) in the same transaction as the dep-start cleanup. Tests:
  - delete restores HS even after the default moved to 渣打;
  - a deleted debited account → the delete succeeds and no account changes.

  Verify with `cargo test -p wealth-backend service::deposits`.
- [x] 2.4 Update `docs/deposits.md` "Add a deposit" and "Edit or delete a deposit" flows with the debit step and its reversal.

## 3. Convert route

- [x] 3.1 `routes::forecast::convert` keeps the lookup, the 409 for a non-deposit plan, the defaults and validation, then calls `service::deposits::convert_plan`; the inline transaction is removed. Tests in `routes/forecast.rs`:
  - converting with a month row stores `dep-start:<id>` = −principal and deletes the plan;
  - with `debit_account_id`, HS is debited;
  - an overdraft returns 400 and the plan still exists;
  - a monthless start month records no item.

  Verify with `cargo test -p wealth-backend routes::forecast`. Also verify `routes::tests::openapi_documents_every_api_operation` is unchanged and passing.
- [x] 3.2 Update the convert wiring in `docs/overview.md` (and the `docs/DATA_FLOW.md` index line if it names the path).
- [x] 3.3 Add one end-to-end test in `backend/tests/api.rs`: `POST /api/deposits` with `debit_account_id` lowers the HS balance; `DELETE` restores it; the response JSON carries `debited_account_id` / `debited_amount`. Verify with `cargo test --test api`.

## 4. Frontend

- [x] 4.1 In `frontend/src/api.ts`, add `debited_account_id` / `debited_amount` to `Deposit` and `debit_account_id` to the deposit create/convert payload type. Verify with `pnpm exec vue-tsc --noEmit`.
- [x] 4.2 In `DepositForm.vue` (create and convert only), add a 扣自 select over HKD `bank` accounts from `api.listCashAccounts()` plus 不扣活期 (`null`):
  - preselect HS → `HS`, SC → `渣打`, else the HKD default, else 不扣活期;
  - re-preselect on 銀行 change until the user picks manually;
  - send `debit_account_id`;
  - show the `debit_account_id` field error.

  Verify in the browser: 轉為定期 on an `hs_deposit` plan preselects HS.
  (implemented; build + vue-tsc pass; browser check confirmed by the user)
- [x] 4.3 In `DepositsView.vue`, change the start-step line 1 to `新增定期 / 轉為定期（自動：扣活期 + 月結調整 −本金）`. Verify it shows on the 定期 → 總覽 page.

## 5. Integration checks

- [x] 5.1 Run `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, and `cd frontend && pnpm build && pnpm exec vue-tsc --noEmit`; all must pass.
- [x] 5.2 Restart the backend per AGENTS.md (kill :8787, `nohup cargo run ... & disown`). Confirm `GET /api/deposits` returns the new fields and `/scalar` shows `debit_account_id` on both create and convert.

## Workflow follow-up

- Archive the change with `/openspec-archive-change`, syncing the delta specs into `openspec/specs/`.
- For the existing deposit `0089`: accept its 2026-11 `dep-start` suggestion; adjust HS by hand if the 130000 hasn't been deducted.
