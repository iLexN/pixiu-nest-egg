# Proposal

## Why

The editor still flags `Err(crate::store::StoreError::NotFound(_))` in
`routes/forecast.rs` tests after the `unused_qualifications` lint landed —
the path sits inside a `matches!` invocation, which rustc's lint does not
inspect (macro hygiene gap). The codebase convention everywhere else is
`use crate::store::StoreError` plus a bare `StoreError`.

## What Changes

- Import `StoreError` in `routes/forecast.rs`'s test module and shorten both
  pattern sites (lines ~310, ~417) to `Err(StoreError::NotFound(_))`.
- The `store` alias (`crate::store::forecast as store`) is a dead end for
  shortening — `store::StoreError` does not resolve, since `StoreError`'s
  import inside `store::forecast` is private. The editor's suggestion
  resolves only by importing.

No lint-table change: rustc cannot see into `matches!`, so this class is
editor-only enforcement. `skip_specs: true` — behavior unchanged.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None — pure refactor; `skip_specs: true` is set.

## Impact

- `backend/src/routes/forecast.rs` — one import, two shortened patterns.
