# Proposal

## Why

`GLOSSARY.md` names one concept — **Cash movement**: "a recorded change to a
Cash account caused by a money event — a 收訖 credit, a 定期 placement debit,
an IBKR 轉入, or a US trade settlement… an undo skips an account that no
longer exists." But only the credit half lives behind a module
(`service::receipts`). The other three movement kinds are hand-rolled in
their writers: `deposits::place` and `ibkr::create_transfer` repeat the same
HKD-bank + overdraft validation + `add_amount(-x)`; `trades` repeats
`usd_broker_id` + `add_amount` four times; `ibkr::delete_transfer` re-rolls
the tolerant-reversal rule that `receipts` already owns. Queued by the
`candidate-catalogue` review.

## What Changes

- `service::receipts` → `service::cash_movement`; `credit`/`reverse`/`refund`
  move unchanged; the three caller files (bonds, deposits, dividends) repoint.
- New verbs:
  - `debit_bank(conn, account_id, amount, field)` — HKD-bank role/currency +
    overdraft validation, applies the debit, returns the account. Adopted by
    `deposits::place` and `ibkr::create_transfer`.
  - `apply_to_broker(conn, currency, delta)` — broker lookup (`Conflict "no
    IBKR {currency} account"`) + `add_amount`. Replaces trades' `usd_broker_id`
    pattern and ibkr's broker step.
  - `shift(conn, Option<i64>, delta)` — apply a delta when the account still
    exists (the tolerant-undo rule); `ibkr::delete_transfer` calls it per leg.
- One observable nuance: ibkr's role/currency error text standardizes to the
  deposits wording `"must be an HKD bank account"` (field name unchanged);
  no test pins the old text.
- `overview::update_ibkr`'s `set_amount` stays out — an absolute set is not a
  Cash movement.

## Capabilities

### New Capabilities

None — pure refactor; `skip_specs: true` is set.

### Modified Capabilities

None.

## Impact

- `backend/src/service/receipts.rs` → `backend/src/service/cash_movement.rs`.
- `backend/src/service/{mod,deposits,ibkr,trades,bonds,dividends}.rs`.
