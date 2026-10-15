# Tasks

## 1. Backend

- [x] 1.1 Remove `credit_amount` from `ReceiveDeposit` in `backend/src/routes/deposits.rs`. In `receive`, compute `principal + interest` (corrected interest when given) once and use it for both the cash credit and the dep-end item. Update the doc comments and the existing test that set `credit_amount: None`. Verify with `cargo test`.
- [x] 1.2 Add a route test: receive with interest corrected 993 → 1000 into a cash row. The row gains principal + 1000, the stored `credited_amount` and the dep-end item equal that amount, and the stored interest is 1000. Verify with `cargo test`.
- [x] 1.3 Update the 收訖 flow in `docs/deposits.md`: credited amount = principal + (corrected) interest, not user-entered. Verify the doc reads that way.

## 2. Frontend

- [x] 2.1 Remove `credit_amount` from `ReceiveDepositBody` in `frontend/src/api.ts`, and from the `DepositsView.vue` receive state, `startReceive`, `saveReceive` and the 存入金額 input. Show a read-only "存入 X" (principal + 實收利息) when a cash row is chosen. Verify with `pnpm build && pnpm exec vue-tsc --noEmit`.

## 3. Integration

- [x] 3.1 Run `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, `cd frontend && pnpm build && pnpm exec vue-tsc --noEmit`. All pass.
- [x] 3.2 Restart the backend detached (per AGENTS.md) and confirm http://127.0.0.1:8787/ answers 200 from a separate shell.

## Workflow follow-up

- Archive with `/openspec-archive-change` after review.
