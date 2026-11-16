# Tasks

## 1. Drop unnecessary path prefixes

- [x] 1.1 `backend/src/service/aia.rs` — `crate::store::now_timestamp()` → `now_timestamp()` in the test helper; verify the warning is gone.
- [x] 1.2 Add `unused_qualifications = "warn"` to `[workspace.lints.rust]` in the root `Cargo.toml`; verify `cargo check` lists every remaining violation.
- [x] 1.3 Apply the machine fixes (`cargo fix --lib -p wealth-backend --tests`): bonds.rs ×3, months.rs, store/bonds.rs, store/mpf.rs ×2, year_review.rs, settings.rs ×2; review the diff.

## 2. Verification

- [x] 2.1 `cargo test` — all pass.
- [x] 2.2 `cargo clippy --all-targets -- -D warnings` — clean.
- [x] 2.3 `cargo fmt` then `cargo fmt --check` — clean.

## Workflow follow-up

- Archive the change (`openspec archive`) and sync delta specs into `openspec/specs/`.
