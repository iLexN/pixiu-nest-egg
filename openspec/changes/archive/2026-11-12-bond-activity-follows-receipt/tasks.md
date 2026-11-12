# Tasks

## 1. Receipt-based bond activity (backend)

- [x] 1.1 Change `calc::bond_active` in `backend/src/calc/bonds.rs` to the receipt predicate (active iff `received_at` is unset). Replace `bond_active_until_maturity_date` with tests covering: unreceived past maturity → active, received before maturity → not active, unreceived future maturity → active. Verify with `cargo test -p wealth-backend bonds`
- [x] 1.2 Derive `BondStatus` in `store::bonds::row_to_bond` from `received_at`, dropping the `today` parameter if nothing else needs it. Switch the `store::bonds::list` filter to `b.received_at IS NULL` / `IS NOT NULL` and remove the `today` bind. Update the `BondStatus` / `Bond::status` doc comments in `backend/src/models/bonds.rs` to say `MATURED` means principal 收訖'd. Verify with `cargo build`
- [x] 1.3 Add route/service tests, modelled on `routes/deposits.rs::matured_unreceived_deposit_counts_until_received`. Seed a bond matured in the past with `received_at` NULL and assert: `GET /api/bonds/summary` lists it in `active`, `totals.active_principal` includes it, its unpaid coupons are in `upcoming_coupons`, and `GET /api/bonds?status=active` returns it. Then `POST /api/bonds/:id/receive` and assert it moves to `matured`, leaves `active_principal`, and a second receive returns 409. Verify with `cargo test`
- [x] 1.4 Add a test that the live month totals (`service::wealth`) include an unreceived matured bond's principal and drop it after 收訖. Verify with `cargo test`

## 2. 債券 page (frontend)

- [x] 2.1 In `frontend/src/views/BondsView.vue`, for an active bond whose `maturity_date <= summary.date`, show a 已到期未收 marker and the existing 收訖 button (reusing `startBondReceive`) in its active header. In the 已到期 section, drop the 本金未收 branch and the 收訖 button, keeping the 收訖日. Verify with `cd frontend && pnpm build && pnpm exec vue-tsc --noEmit`

## 3. Docs and glossary

- [x] 3.1 `docs/investments.md` 債券 section (lines ~40–57): describe receipt-based activity and the 已到期未收 flag in the active section, replacing the "stays maturity-date based … 本金未收" sentence. Verify no remaining mention of 本金未收 or "maturity-date based" in docs/
- [x] 3.2 `docs/database.md`: in the `bonds.received_at` row, replace "matured" with "NULL while the principal is still on the books"; change the "Bond status (`matured` once maturity_date …)" line to `matured` once 收訖; change the deposit status line ("`End` once end_date is today or past") to the receipt rule; add the missing `deposits.start_date` row (optional placement date — a create with it debits `debited_account_id`); reword the `cash_accounts` intro to "one row per cash account: the 活期 bank rows (HS, 渣打) plus the IBKR broker cash in each currency". Verify by re-reading the diff
- [x] 3.3 `GLOSSARY.md`: 收訖 lists bond principal; 債券 is active until principal 收訖; 已到期未收 covers an unreceived 定期 or 債券 past its end/maturity date; 活期 is the bank-role accounts only (HS, 渣打) — IBKR broker cash is a Cash account, not 活期; Cash account is the umbrella term for bank + broker. No implementation details. Verify the entries follow the existing `**Term**:` / `_Avoid_` format
- [x] 3.4 `docs/adr/0010-cash-movement-semantics.md`: add a short amendment to the 收訖 bullet saying US dividends credit the USD (IBKR) broker account and never touch 活期 or month 利息. Verify the existing ADR text is otherwise unchanged

## 4. Integration check

- [x] 4.1 Run `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, and `cd frontend && pnpm build && pnpm exec vue-tsc --noEmit`. All must pass
- [x] 4.2 Restart the backend per AGENTS.md (`lsof -ti :8787 | xargs kill`, then the detached `nohup cargo run …`) and confirm `GET /api/bonds/summary` on the live DB lists any unreceived matured bond under `active`

## Workflow follow-up

- Run `trail check`, then archive with `openspec-archive-change` (sync the `bonds` delta into `openspec/specs/`).
