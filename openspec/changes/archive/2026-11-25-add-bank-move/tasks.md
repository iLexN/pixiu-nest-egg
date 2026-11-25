# Tasks

## 1. Backend move service

- [x] 1.1 Add `NewBankMove { from_account_id, to_account_id, amount }` (serde + `utoipa::ToSchema`, doc comment naming `POST /api/cash-accounts/moves`) to `backend/src/models/cash.rs` and re-export it like `NewCashAccount`; verify `cargo build`
- [x] 1.2 Add `service::cash_accounts::bank_move(pool, &NewBankMove) -> Result<Vec<CashAccount>, ServiceError>` per design decision 3 (pre-transaction checks on `amount` and `from != to`; in one transaction `cash_movement::debit_bank(.., "from_account_id", "amount", ..)`, then a destination check for HKD `bank` → `to_account_id`, then `add_amount`; return `[from, to]` re-loaded); verify with inline tests: HS→渣打 moves the amount and keeps the sum; a move back restores both; overdraft → Validation on `amount` with both unchanged; broker destination → `to_account_id` with the source unchanged; broker source → `from_account_id`; same account → `to_account_id`; `0`, negative, and NaN amounts → `amount`; unknown destination → `NotFound` with the source unchanged; no `month_items` row is written

## 2. API

- [x] 2.1 Add the `bank_move` handler in `backend/src/routes/cash_accounts.rs` with `#[utoipa::path]` (`operation_id = "cash_accounts_move"`, `post`, `/api/cash-accounts/moves`, tag `cash-accounts`, `request_body = NewBankMove`, 200 `[CashAccount]`, 400/404 `ErrorBody`); register it in `api_router` with its own `utoipa_axum::routes!(cash_accounts::bank_move)` call; add `("/api/cash-accounts/moves", "post")` to `openapi_documents_every_api_operation`; verify `cargo test` (that test included)
- [x] 2.2 Extend `backend/tests/cash_accounts.rs` with an HTTP round trip: POST a move, expect 200 with both accounts in source-then-destination order, then an overdraft POST that returns 400 with a field error; verify `cargo test --test cash_accounts`

## 3. Frontend

- [x] 3.1 Run `cd frontend && pnpm gen:api`; in `src/api.ts` add `type NewBankMove = S['NewBankMove']` and `bankMove(body: NewBankMove): Promise<CashAccount[]>` posting to `/cash-accounts/moves`; verify `pnpm exec vue-tsc --noEmit`
- [x] 3.2 In `src/views/OverviewView.vue`, add the 「調撥」 link on the 活期 row and its inline form (source/destination `<select>`s over `data.semi_liquid.cash_rows`, auto-pairing when exactly two rows, numeric amount via `numOrNull`, 儲存/取消 in a flex-gap wrapper), submitting through `run(..., { message: '已調撥' })`; per-row 編輯 unchanged; verify `pnpm build` and `pnpm exec vue-tsc --noEmit`

## 4. Docs and domain model

- [x] 4.1 Add **活期調撥** to `GLOSSARY.md` under Cash & accounts (definition, why nothing is recorded, _Avoid_: transfer, 轉入); verify it contains no implementation detail
- [x] 4.2 Append an amendment bullet to `docs/adr/0010-cash-movement-semantics.md` (bank→bank, atomic, overdraft-checked, records nothing, undo is a reverse 調撥) and add this change to its Source line; verify the ADR reads consistently
- [x] 4.3 Update the 半流動資金 paragraph of `docs/overview.md` (and the cash entry in `docs/DATA_FLOW.md` if it lists cash-row actions): 「調撥」 → `POST /api/cash-accounts/moves` → `cash_accounts` (two rows, one transaction); verify both docs name the endpoint

## 5. Integration

- [x] 5.1 Run `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, and `cd frontend && pnpm build && pnpm exec vue-tsc --noEmit`; all pass
- [x] 5.2 Against a scratch DB (`WEALTH_DB=/tmp/wealth-test.db`, spare `WEALTH_ADDR`), seed HS/渣打, POST a move and confirm both balances and an unchanged `semi_liquid.cash_sum` in `GET /api/overview`; POST an overdraft and confirm 400 and no change
- [x] 5.3 Restart the real backend detached per AGENTS.md and confirm `GET /api/cash-accounts` answers and `/scalar` lists `POST /api/cash-accounts/moves`

## Workflow follow-up

- Run `trail check` before committing.
- Archive with `openspec-archive-change` after the user verifies a real 調撥 on 總覽.
