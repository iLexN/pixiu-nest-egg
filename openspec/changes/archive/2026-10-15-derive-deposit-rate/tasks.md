# Tasks

## 1. Backend derivation

- [x] 1.1 Add `derive_rate` to `backend/src/calc/deposits.rs` with inline tests (140000/1279/2026-10-13→2027-02-02 = 1279/140000*365/112; missing start, principal 0, missing interest, end ≤ start → None; interest 0 → Some(0.0)); verify with `cargo test -p wealth-backend derive_rate`
- [x] 1.2 Add `rate_derived: bool` to `Deposit` in `backend/src/models.rs` and fill it in `row_to_deposit` (stored rate wins; otherwise `derive_rate`); add a test that a deposit created without a rate reads back with the derived rate and `rate_derived = true`, and one with a stored rate reads back unflagged; verify with `cargo test`
- [x] 1.3 Update `docs/deposits.md` with the derived-rate rule; verify the doc names the formula and the eligibility conditions
- [x] 1.4 In `backend/src/routes/deposits.rs` `update`, merge `patch.rate` against `None` when `existing.rate_derived` so a PATCH that leaves rate absent never persists the derived rate; covered by the PATCH case in the route test

## 2. Frontend

- [x] 2.1 Add `rate_derived: boolean` to `Deposit` in `frontend/src/api.ts`, and add the `title="由利息及日期計算"` hover hint to the 利率 cell in `DepositTable.vue` when derived; verify with `pnpm exec vue-tsc --noEmit`
- [x] 2.2 In `DepositForm.vue`, when editing a derived-rate deposit, load the rate field empty and show the derived percent as its placeholder (default `2.8`); verify with `pnpm build`

## 3. Integration

- [x] 3.1 Run `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, `cd frontend && pnpm build && pnpm exec vue-tsc --noEmit`; all pass
- [x] 3.2 Restart the backend on :8787 and confirm `GET /api/deposits` shows stored rates unchanged and `rate_derived: false` on them, and that rows without enough inputs (e.g. SC-0044) still have a null rate

## Workflow follow-up

- Archive the change with `/openspec-archive-change` after review.
