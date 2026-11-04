# Design

## Context

- `service::deposits::create` (backend/src/service/deposits.rs) inserts the row and records `dep-start:<id>` via `months::record_receipt_item` in one transaction. It touches no cash account.
- `routes::forecast::convert` (backend/src/routes/forecast.rs) opens its own transaction, calls `store::deposits::insert_deposit` and `store::forecast::delete_item_in_tx`, and bypasses the service entirely.
- Cash movement already has two precedents:
  - `service::receipts::credit` / `reverse` (收訖 credits, recorded as `credited_account_id` / `credited_amount`);
  - `service::ibkr::create_transfer` / `delete_transfer` (HKD-bank source check, in-transaction overdraft check, exact reversal skipping deleted accounts; ids stored by migration 0034).
- `store::deposits::insert_deposit` marks a deposit received at insert when `end_date <= today`.
- The forecast anchors its first column on the current month's stored `start_cash` (月初) and falls back to the live 活期 sum only while that is unset. Deposit placements are already subtracted in their start month.

## Goals / Non-Goals

**Goals:**
- One placement code path, so create and convert can never drift again.
- The debit is exact and reversible from stored data, independent of later default-account changes.

**Non-Goals:**
- No backfill of debits for existing deposits.
- No re-syncing the debit when a deposit's principal or bank is edited.
- No change to 收訖, the receipt crediting rule, or the forecast math.

## Decisions

1. **Shared tx helper in the service layer.** It lives in `service::deposits`:

   ```rust
   async fn place(
       tx: &mut sqlx::SqliteConnection,
       deposit: &ValidatedDeposit,
       body: &NewDeposit,
   ) -> Result<i64, ServiceError>
   ```

   In order, it:
   1. takes `next_sort_order_tx`;
   2. runs `insert_deposit`;
   3. applies the optional debit;
   4. calls `record_receipt_item` for dep-start.

   - `pub async fn create(pool, deposit, body)` opens a transaction, calls `place`, and commits.
   - `pub async fn convert_plan(pool, item_id, deposit, body)` opens a transaction, calls `place`, then `store::forecast::delete_item_in_tx(item_id)`, and commits.

   The route keeps the plan lookup, its 409 for a non-deposit plan, the bank/principal defaults and validation, then calls `convert_plan`. Calling `store::forecast` from `service` respects the layer rules.
   - *Alternative considered:* have the convert route call `service::deposits::create` and delete the plan afterwards. Rejected: two transactions would let a failed delete leave both the plan and the deposit, which double-counts in the forecast.

2. **Explicit `debit_account_id`, no server-side default.** `NewDeposit` gains `debit_account_id: Option<i64>`. Absent means no debit, which keeps the API additive and the backend unambiguous.
   - The UI owns the bank→account preselection (HS → label `HS`; SC → label `渣打`; else the HKD `is_default_receipt` account). It is the same UX pattern as the 收訖 form's bank preselection.
   - *Alternative considered:* a `bank_out` bool plus a server-side default. Rejected: label-matching banks to accounts on the server is fragile, and the receipt default (HS) would wrongly debit HS for SC deposits.

3. **Validation reuses the IBKR rules.** Inside the transaction:
   - `cash_accounts::load_one` (unknown id → 404 via `StoreError::NotFound`, as IBKR does);
   - require `role == Bank && currency == Hkd`, else Validation on `debit_account_id`;
   - require `amount >= principal`, else Validation `"<label> holds only <amount>"`.

   Validation errors return before commit, so nothing is written. The debit applies only when `principal > 0` and the inserted row is not auto-received.

4. **Storage.** Migration `0035_deposit_debit.sql`:

   ```sql
   ALTER TABLE deposits ADD COLUMN debited_account_id INTEGER REFERENCES cash_accounts(id) ON DELETE SET NULL;
   ALTER TABLE deposits ADD COLUMN debited_amount REAL;
   ```

   A store helper `set_debit(conn, id, account_id, amount)` writes them after insert. Reading the pair back for delete reuses the existing `Credit` struct shape. `store::cash_accounts::delete` also runs `UPDATE deposits SET debited_account_id = NULL WHERE debited_account_id = ?`, matching how it treats `credited_account_id`.

5. **Delete reversal.** `service::deposits::delete` loads the stored debit. If present, it adds `debited_amount` back to `debited_account_id` when that account still exists (the `receipts::reverse` semantics with the sign flipped), then runs the existing dep-start cleanup and row delete in the same transaction. Reverse by passing the negated amount to `receipts::reverse`, or write a small sibling `refund` helper in `receipts`; pick whichever reads cleaner, but keep the "deleted account → no-op" rule.

6. **Frontend.**
   - `DepositForm.vue` loads `api.listCashAccounts()` (existing client) when not editing.
   - It shows a 扣自 `<select>` over HKD `bank` accounts plus 不扣活期 (value `null`).
   - It preselects from 銀行 and re-preselects on 銀行 changes until the user picks manually.
   - It sends `debit_account_id` on create/convert only.
   - `Deposit` / payload types in `api.ts` gain the new fields.
   - `DepositsView.vue` hint line 1 becomes `新增定期（自動：扣活期 + 月結調整 −本金）`.

## Risks / Trade-offs

- [Forecast double count while 月初 is unset: the live 活期 sum drops by the debit *and* the placement subtracts it again] → This matches today's manual HS edit and the IBKR transfer treatment, and the current month's `start_cash` is set. No forecast change in this change.
- [The user edits principal after placing] → The stored debit stays as actually moved, and delete reverses exactly that. Documented in the spec scenario "Editing leaves the debit alone".
- [Historic entry with a past `end_date` and a preselected account] → Auto-received inserts never debit (spec scenario), so back-filling old deposits can't move today's balance.
- [Existing deposit `0089` was placed without a debit or item] → No backfill. The user accepts its 2026-11 suggestion and adjusts HS manually if needed.

## Migration Plan

Migration 0035 is additive (nullable columns) and applies on startup. Rollback means reverting the code; the extra columns are harmless to older code paths since `DEPOSIT_COLUMNS` lists columns explicitly.
