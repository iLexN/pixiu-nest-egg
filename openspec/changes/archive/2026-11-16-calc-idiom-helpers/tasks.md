# Tasks

## 1. Name the repeated idioms

- [x] 1.1 `year_review.rs`: add `rate()`/`monthly()` helpers; swap the five `d > 0.0` ratio matches and four `/12.0` maps; reuse `stored_record` for the second `records.get`.
- [x] 1.2 `forecast.rs`: add `unsigned_zero()` with the `-0.0` rationale comment; use at all seven sites.
- [x] 1.3 `months.rs`: fold the `total_change`/`liquid_change` twin blocks into one helper.

## 2. Verification

- [x] 2.1 `cargo test` — all pass unmodified.
- [x] 2.2 `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` — clean.

## Workflow follow-up

- Archive the change (`openspec archive`) and sync delta specs into `openspec/specs/`.
