# Tasks

## 1. Shorten the StoreError paths

- [x] 1.1 Add `use crate::store::StoreError;` to the `tests` module in `backend/src/routes/forecast.rs` and replace both `Err(crate::store::StoreError::NotFound(_))` patterns with `Err(StoreError::NotFound(_))`; verify `cargo check` is clean and the editor warning is gone.
- [x] 1.2 Confirm the other `crate::`-inside-macro sites are legitimate, not violations: `crate::calc::approx_eq` in `service/ibkr.rs` (×2) and `crate::models::BondStatus` in `routes/bonds.rs` are not imported in those files, so the full paths are required.

## 2. Verification

- [x] 2.1 `cargo test` — all pass.
- [x] 2.2 `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` — clean.

## Workflow follow-up

- Archive the change (`openspec archive`) and sync delta specs into `openspec/specs/`.
