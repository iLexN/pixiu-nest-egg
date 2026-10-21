# Design

## Context

See proposal.md — Why. `store::months::record_receipt_item` is the shared helper every receipt flow already uses: it inserts an `adjustment` month item keyed by `auto_key`, skips when the item exists or the month has no row, and clears any matching dismissal. Deposits use it for `dep-end:<id>` inside `service::deposits::receive`, which wraps the credit + flag + item in one transaction.

## Goals / Non-Goals

**Goals:**

- Deposit creation records the `dep-start:<id>` item atomically with the insert.
- The existing suggestion remains the fallback when the item was skipped at create time.

**Non-Goals:**

- Keeping the item in sync when the deposit is later edited or deleted — the same accepted limitation coupon/bond-end items carry today.
- Changing the suggestion builder, month-stat endpoints, or the frontend.

## Decisions

- **Record inside the create transaction, in `service::deposits`.** `insert_deposit` already takes a generic executor, so the route's insert moves into a new `service::deposits::create` that runs insert + `record_receipt_item` in one transaction — the same shape as `receive`. Alternative (route calls both stores): rejected, layer rules keep store composition in `service`.
- **Item amount is −`principal`, label falls back to `定期`** — exactly what the dep-start suggestion emits.
- **Write whenever `start_date` and a nonzero `principal` are present, regardless of month age.** dep-end already writes to past event months; backfilled history stays the user's responsibility. The suggestion cutoff (current month only) is about *suggesting*, not about recording receipts.
- **Suggestion code unchanged.** Once the item is stored, `stored_keys` suppresses the suggestion automatically; when the create-time write was skipped (no month row), the suggestion still surfaces later.

## Risks / Trade-offs

- **Editing `start_date`/`principal` leaves a stale item** → accepted; matches coupon/bond-end precedent, and the item is a normal month row the user can delete.
- **Deleting the deposit leaves the item** → same precedent; the cash did move when the deposit existed.
