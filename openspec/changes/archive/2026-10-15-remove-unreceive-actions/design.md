# Design

## Context

取消收訖 exists three times with the same shape: a `POST .../unreceive` endpoint that reverses the stored bank-in, deletes the auto month item (`dep-end:<id>` / `bond-end:<id>`), and clears `received_at` — plus a frontend button emitted only from rows where `received_at` is set. The family-deposits variant is record-only (no cash/month-item side effects). See proposal.md for scope and motivation.

## Goals / Non-Goals

**Goals:**
- Delete the three unreceive endpoints, their UI affordances, API client methods, dead store helpers, and spec/doc/test coverage of unreceive.

**Non-Goals:**
- No schema migration: `credited_asset_id`/`credited_amount` stay.
- No change to `收訖` (receive) behavior, deposit CRUD, or the coupon/dividend flows.

## Decisions

- **Keep `credited_asset_id`/`credited_amount`.** They are still written on every 收訖 and now serve purely as provenance of which cash row received the bank-in and for how much. Dropping them would need a destructive migration for zero behavioral gain — the columns cost nothing and remain useful audit data.
- **Keep the coupon/dividend reversal semantics.** Clearing a coupon's `received_amount` (bonds.rs PATCH `(Some, None)` arm) and un-receiving a dividend via edit still reverse their credits — that is field-edit semantics, not a standalone undo action, and it shares `months_store::add_manual_asset_amount`/`delete_auto_item`, which must therefore stay.
- **Delete `credited_of`/`clear_receipt` (deposits, bonds) and `clear_receipt` (family).** Once the three handlers go, nothing else calls them; leaving them would be dead code. `mark_received` stays — it writes the credited columns on 收訖.
- **DepositTable loses the `unreceive` emit and `v-else` button**, not a prop: the upcoming list never renders the button (all rows are unreceived), and the history view is the only consumer of the emit.

## Risks / Trade-offs

- [A mistaken 收訖 has no in-app undo] → Accepted: the credited cash row and month item would need a manual edit, or the record deleted and re-created. Documented in the proposal's Impact section as a deliberate removal, not an oversight.
