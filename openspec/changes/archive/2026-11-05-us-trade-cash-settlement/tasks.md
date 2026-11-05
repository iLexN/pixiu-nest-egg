# Tasks

## 1. Schema and store

- [x] 1.1 Add `backend/migrations/0036_trade_settlement.sql` (`ALTER TABLE trades ADD COLUMN settled_amount REAL`) and update the `trades` section of `docs/database.md`; verify a migration test passes that the column exists and defaults NULL
- [x] 1.2 Extend `store::trades`: `insert_trade`/`update_trade` write `settled_amount`, and add a read of the recorded amount for the update/delete paths; verify `cargo test` passes for the store module

## 2. Service settlement

- [x] 2.1 Add `service::trades` create/update/delete wrappers that run the store op plus `cash_accounts::add_amount` on the USD broker row in one `pool.begin()` transaction (market == US && 股數 > 0 applies `calc::trade_amount`; update reverses the recorded amount then re-applies; delete reverses; NULL stays NULL); verify inline tests cover buy/sell/zero-share/HK/negative/pre-existing/unwind-to-zero-shares
- [x] 2.2 Reject `stock_id` repoints across markets in the update path with a `ServiceError::Validation` field error on `stock_id`; verify a test shows a US trade cannot take an HK stock and vice versa

## 3. Routes

- [x] 3.1 Swap `store::insert_trade`/`update_trade`/`delete_trade` for the service functions in `routes::trades` (keep `record_market` calls after the service call); verify `cargo test` passes including existing trade route tests and `openapi_documents_every_api_operation`

## 4. Docs and verification

- [x] 4.1 Update `docs/stocks.md` (trade → USD broker cash flow) and `docs/overview.md` (IBKR 現金 also moves via trades); verify the documented flow matches the code
- [x] 4.2 Run `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, and `cd frontend && pnpm build && pnpm exec vue-tsc --noEmit`; verify all pass
- [x] 4.3 Restart the backend (`lsof -ti :8787 | xargs kill` then detached `nohup cargo run`), record a US BUY against the live API, and verify `usd_cash` moved by the trade total

## Workflow follow-up

- Archive the change after the project's review requirements are satisfied (sync delta specs into `openspec/specs/` as part of archive).
- Verify the archived result.
