# Tasks

## 1. Backend default rule

- [x] 1.1 In `backend/src/routes/dividends.rs::update`, when `received_amount` transitions NULL→Some and the merged `received_price` is NULL, resolve `SELECT manual_price FROM stocks WHERE id = ?` inside the existing transaction and store it as `received_price`; verify a blank-field 收訖 stores the stock's current 現價.
- [x] 1.2 Add tests in `backend/tests/api.rs`: (a) receipt with no `received_price` and stock `manual_price` 5.41 stores 5.41; (b) receipt with no price and no `manual_price` stores NULL; (c) clearing `received_price` on an already-received record keeps it NULL (no re-fill); verify with `cargo test`.
- [x] 1.3 Update `docs/stocks.md` (~the 收訖 PATCH description) and `docs/DATA_FLOW.md` (~L102, the received_price/同時更新現價 note) to document the blank → current 現價 default; verify the documented behavior matches the new spec scenarios.

## 2. Verification

- [x] 2.1 Run `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, and `cd frontend && pnpm exec vue-tsc --noEmit`; verify all pass.
