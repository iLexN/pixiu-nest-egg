# Tasks

## 1. Rename non-ASCII test identifiers

- [x] 1.1 In `backend/src/calc/candidates.rs`, rename `dep_start_needs_a_start_date_and_falls_back_to_定期` to `dep_start_needs_a_start_date_and_falls_back_to_the_default_label` and add a doc comment naming the pinned `定期` literal; verify with `cargo test -p wealth-backend dep_start_needs`.
- [x] 1.2 In `backend/src/calc/candidates.rs`, rename `ibkr_is_minus_amount_labelled_轉入` to `ibkr_is_minus_amount_labelled_ibkr_transfer`; verify with `cargo test -p wealth-backend ibkr_is_minus`.
- [x] 1.3 In `backend/src/service/deposits.rs`, rename `an_unlabeled_deposit_records_dep_start_as_定期` to `an_unlabeled_deposit_records_dep_start_with_the_default_label`; verify with `cargo test -p wealth-backend an_unlabeled_deposit`.

## 2. Enable the non_ascii_idents lint

- [x] 2.1 Add `[workspace.lints.rust] non_ascii_idents = "warn"` to the root `Cargo.toml` and `[lints] workspace = true` to `backend/Cargo.toml`; verify by temporarily adding a non-ASCII fn name and confirming `cargo check` warns, then revert.

## 3. Verification

- [x] 3.1 `cargo test` passes with all renamed tests.
- [x] 3.2 `cargo clippy --all-targets -- -D warnings` is clean.
- [x] 3.3 `cargo fmt --check` is clean.

## Workflow follow-up

- Archive the change (`openspec archive`) and sync delta specs into `openspec/specs/`.
