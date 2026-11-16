# Proposal

## Why

The editor flags `crate::store::now_timestamp()` in `service/aia.rs` tests as
"Path prefix not necessary" — `now_timestamp` is already in scope via
`use super::*`. Rustc's `unused_qualifications` lint is allow-by-default, so
`cargo clippy` stays silent, same gap as `non_ascii_idents` before it
(archived `2026-11-16-ascii-identifiers`).

## What Changes

- Drop the unnecessary `crate::store::` prefix at
  `backend/src/service/aia.rs` (test helper `policy`).
- Add `unused_qualifications = "warn"` to the existing
  `[workspace.lints.rust]` table in the root `Cargo.toml` and fix every other
  violation it surfaces across the backend, so editor and toolchain report
  the same thing.

No behavior changes — path rewrites resolve to the same items;
`skip_specs: true`.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None — pure refactor plus tooling configuration; `skip_specs: true` is set.

## Impact

- `backend/src/service/aia.rs` plus any other files the lint flags.
- `Cargo.toml` — one more entry in the workspace lint table.
