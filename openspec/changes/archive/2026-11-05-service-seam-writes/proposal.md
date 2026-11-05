# Proposal

## Why

The routes layer still owns write workflows. Eight handlers open `pool.begin()` and compose stores, and `routes::year_review::update` decodes `SqliteRow` columns — while deposits, trades, ibkr, dividends, and bonds writes already go through deep service functions. The repo's own learning records the failure mode: `routes/forecast.rs::convert` once bypassed `service::deposits::create` and skipped the 活期 debit and `dep-start:` item. As long as a second write path exists, new features can take it.

## What Changes

- **Rule adopted**: routes never open a transaction. Single-domain atomic ops are owned by the store (the existing `store::cash_accounts` pattern); multi-store and domain-rule workflows live in the service.
- **Service gains write fns**: `service::year_review::update`, `service::months::{upsert,create_item,update_item}`, `service::mpf::update`, and a new `service::aia` with `create_event`/`remove_event` (snapshot-prev + apply / undo-effects, same shape as `service::deposits`).
- **Store-owned transactions**: `store::aia::delete_with_events(pool)`, `store::bonds::delete_with_coupons(pool)`, `store::stocks::reorder(pool)` take over their route-side `pool.begin()` wrappers.
- **`store::year_review` returns a typed record** — `SqliteRow`/`try_get` leaves the route layer.
- **Domain validation moves with the workflow** into `ServiceError::Validation`; shape checks (`parse_ym`, year range, "at least one field") stay in routes.
- **Dedup**: `active_principal`/`active_principal_of` removed from `routes/bonds.rs`; the route calls `service::bonds`.

No externally observable behavior changes: same endpoints, request/response shapes, and figures.

## Capabilities

### New Capabilities

None — pure refactor; `skip_specs: true` is set.

### Modified Capabilities

None — no requirement changes; `skip_specs: true` is set.

## Impact

- New `backend/src/service/aia.rs`; edits to `service/{mod,year_review,months,mpf}.rs`
- `backend/src/routes/{year_review,aia,bonds,months,mpf,stocks}.rs` shrink to extract/validate/map
- `backend/src/store/{aia,bonds,stocks,year_review}.rs` gain pool-taking fns / typed records
- Inline `connect_memory()` tests at the new service fns
