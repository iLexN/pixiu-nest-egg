# Design

## Decisions

### D1: One module named after the glossary term

`service::cash_movement` subsumes `receipts` — the glossary's "Cash movement"
covers credit, debit, 轉入, and settlement, so the module owns all of them.
Keeping `receipts` for the credit half would split one concept across two
modules.

### D2: Three new verbs, shaped by what callers actually repeat

- `debit_bank` returns the validated `CashAccount` (callers record
  `account.id` on their own rows — `set_debit` vs `from_account_id`), takes
  `field` for the validation error name.
- `apply_to_broker` owns the `Conflict` message; sign is the caller's (`delta`).
- `shift` is the atomic tolerant move — `Option<i64>` in, `load_one`→`add_amount`,
  `NotFound` skips. `reverse`/`refund` stay as the `Credit`-recorded
  convenience wrappers; `shift` serves raw `(id, delta)` legs like ibkr's.

### D3: Message text converges on the deposits wording

`"must be an HKD bank account"` becomes the shared role/currency message;
the field name stays the caller's (`debit_account_id`/`from_account_id`),
so error payloads still name the request field.

## Risks / Trade-offs

- `debit_bank` returns the account (not a `Credit`) because deposits needs
  the label for its overdraft check *inside* the same transaction — the
  check is in the verb, so the return is only used for `account.id`.
