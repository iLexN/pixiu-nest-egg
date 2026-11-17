# Proposal

## Why

The `page-read-models-in-service` change moved every page's read model into
`service/` — except the biggest one. `routes::months::{list,summary,show}`
still compose in the route: `show` loads the book, items, events and
dismissed keys, then calls three `calc` functions; `summary` calls
`calc::pool_totals`; `list` filters by year. ADR 0005's amendment says routes
only parse and delegate.

## What Changes

- `service::months::{list, summary, detail}` own the read models:
  `list(year: Option<i32>)`, `summary()`, `detail(month: NaiveDate)` →
  `Err(StoreError::NotFound)` for an unknown month.
- `MonthSummaryResponse` and `MonthDetailResponse` move to
  `service::months`, `pub use`-re-exported from the route — OpenAPI schemas
  and `api-types.ts` stay identical.
- Routes keep `parse_ym` and `ListQuery` parsing, then delegate. The route
  drops its `calc` imports (`build_suggestions`, `interest_components`,
  `pool_totals`, `suggestion_actions_enabled`) and its direct `store::`
  loads (`load_items`, `dismissed_keys`).
- No behavior change: same responses, same error mapping.

## Capabilities

### New Capabilities

None — pure refactor; `skip_specs: true` is set.

### Modified Capabilities

None.

## Impact

- `backend/src/service/months.rs` — read models + moved response structs.
- `backend/src/routes/months.rs` — shrinks to thin handlers.
- Tests stay where they are: the existing route tests exercise the same
  handlers through the delegate.
