# Design

## Context

HS and 渣打 are `bank` rows in `cash_accounts`; 總覽 edits each one's balance directly through `PATCH /api/cash-accounts/{id}`. Every existing money event that touches 活期 follows ADR 0010: it moves cash into or out of 活期, so it is recorded with the event (account ids stored for exact reversal) and, where it is not spending, produces a month 調整 Candidate. IBKR 轉入 (`service::ibkr::create_transfer`) is the nearest analogue: one transaction, `cash_movement::debit_bank` for the source with the overdraft check inside the transaction, a credit to the destination, a log row, and a Candidate.

A 調撥 differs in one way that drives the whole design: both legs are inside 活期, so the 活期 total, and with it 月初, live 總數/流動資產, 年結, and the 預測 cash chain (which only reads the 活期 sum; its `SC` row is 定期 placements by bank, not the 渣打 balance), are unchanged. Nothing in the app reads a single bank account's balance except the 總覽 rows and the overdraft check.

## Goals / Non-Goals

**Goals:**

- One atomic write replacing the two manual edits, with the same overdraft protection as every other bank debit.
- Reuse the existing debit helper and error-field conventions; add no new table.

**Non-Goals:**

- A 調撥 history, list endpoint, or delete/undo endpoint.
- Fees, dates, cross-currency moves, or moves involving IBKR (轉入 remains the only bank→broker path).
- Prompting a 調撥 from the 定期 form when the debit account is short (possible follow-up).

## Decisions

### 1. No log row

Every consumer of the 轉入 log (美股 B1 cumulative, 年結 invested, the `ibkr:<id>` 調整 Candidate, the 預測 cash flow) exists because 轉入 leaves 活期. None applies to a move within 活期, so a log would be a table with no reader. The exact-undo argument for logging is covered by the move being symmetric: a 調撥 the other way restores both balances exactly.

*Alternative*: a `bank_moves` table with list and delete, like 轉入. Rejected for now: a migration, three endpoints, and a history view with no consumer. It can be added later without changing the move's behavior.

### 2. Endpoint `POST /api/cash-accounts/moves`

The move is an action on cash accounts, so it sits under the existing `cash-accounts` resource and tag. The plural noun keeps it a collection-style POST, mirroring `POST /api/ibkr/transfers`. The static segment `moves` does not collide with `/api/cash-accounts/{id}`, which has no POST. It gets its own `utoipa_axum::routes!(cash_accounts::bank_move)` call in `api_router`, one call per path.

Body `NewBankMove { from_account_id: i64, to_account_id: i64, amount: f64 }` in `models/cash.rs`. Response `200` with `[CashAccount; 2]` (source, destination) as `Vec<CashAccount>`: a caller gets the new balances without a second read. 總覽 reloads anyway. `200`, not `201`, because nothing is created.

*Alternative*: `PATCH` with a delta on each account. Rejected: two requests again, which brings back the non-atomic risk the change removes.

### 3. Service in `service::cash_accounts::bank_move`, reusing `cash_movement::debit_bank`

`bank_move` validates `amount` (finite, > 0 → field `amount`) and `from != to` (→ field `to_account_id`) before opening the transaction, then in one transaction:

1. `cash_movement::debit_bank(&mut tx, from, amount, "from_account_id", "amount", &now)`: checks HKD `bank` and overdraft, debits, and returns the source.
2. `store::cash_accounts::load_one(&mut *tx, to)`: `NotFound` → 404; role must be `bank` and currency HKD, else Validation on `to_account_id` with debit_bank's message ("must be an HKD bank account").
3. `store::cash_accounts::add_amount(&mut tx, to, amount, &now)`.
4. Re-load both accounts, commit, and return `[from, to]`.

The service goes in `service::cash_accounts`, next to account CRUD, rather than in `service::cash_movement`. In the glossary a Cash movement is *recorded with the event so it can be undone exactly*, and a 調撥 is deliberately not recorded. It uses the cash-movement debit rule without being a Cash movement.

The destination check runs after the debit. That is safe because any error rolls back the whole transaction, and it keeps `debit_bank` as the single source of the source-side rules.

### 4. Frontend

`OverviewView.vue` gains a 「調撥」 link on the 活期 row of the 半流動資金 table. The link opens an inline form with two `<select>`s over `data.semi_liquid.cash_rows` and an amount input (`type="number"`, coerced with `numOrNull`). With exactly two rows, picking one side sets the other side to the other row. Submit goes through the page's `run(..., { message: '已調撥' })`, so errors surface and the view reloads, per the Page lifecycle. Adjacent link buttons sit in a flex-gap wrapper. `api.ts` gains `NewBankMove = S['NewBankMove']` and `bankMove(body): Promise<CashAccount[]>` after `pnpm gen:api`.

### 5. Domain docs

- `GLOSSARY.md`, Cash & accounts: **活期調撥**: a move of HKD between two 活期 bank accounts. The 活期 total is unchanged, so nothing is recorded, no month item is suggested, and the undo is a 調撥 back. _Avoid_: transfer, 轉入.
- ADR 0010 amendment: 活期調撥 moves bank→bank atomically with the overdraft check and records nothing, because no 活期-total reader is affected. Adding a log later is a new decision, not a fix.

## Risks / Trade-offs

- [No history: a mistaken 調撥 leaves no trace] → The form shows a success message; the balances on 總覽 are the record; a reverse 調撥 fixes it exactly.
- [A stale balance blocks a legitimate move] → Intended. The overdraft error names the source's stored balance, prompting a 編輯 first.
- [Someone later "fixes" the missing log or month item by analogy with 轉入] → The ADR amendment and the glossary entry state why nothing is recorded.

## Migration Plan

No schema change. Deploy by restarting the backend. Rollback is removing the endpoint and the form; no data depends on them.
