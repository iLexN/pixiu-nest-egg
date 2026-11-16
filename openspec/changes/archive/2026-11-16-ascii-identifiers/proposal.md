# Proposal

## Why

The editor's "Non-ASCII characters" inspection flags three test function names
that embed Chinese domain labels (`定期`, `轉入`). Rustc's `non_ascii_idents`
lint is allow-by-default on the current toolchain, so `cargo clippy` stays
silent — the warning exists only in the editor, where code review happens.
Renaming the identifiers and enabling the lint brings the toolchain in line
with what the editor reports.

## What Changes

- Rename three `#[cfg(test)]` functions to ASCII names:
  - `dep_start_needs_a_start_date_and_falls_back_to_定期` →
    `dep_start_needs_a_start_date_and_falls_back_to_the_default_label`
  - `ibkr_is_minus_amount_labelled_轉入` →
    `ibkr_is_minus_amount_labelled_ibkr_transfer`
  - `an_unlabeled_deposit_records_dep_start_as_定期` →
    `an_unlabeled_deposit_records_dep_start_with_the_default_label`
- Enable `non_ascii_idents = "warn"` via `[workspace.lints.rust]` in the root
  `Cargo.toml` and `[lints] workspace = true` in `backend/Cargo.toml`, so
  `cargo check`/`clippy` (and rust-analyzer, which reads the same table) flag
  any future non-ASCII identifier. `cargo clippy -D warnings` escalates it to
  a hard error in verification.

No behavior changes: the renames touch test code only, and the lint is
tooling configuration. Chinese text in string literals, comments, and docs
remains untouched — it is domain content, not identifiers.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None — pure refactor plus tooling configuration; `skip_specs: true` is set.

## Impact

- `backend/src/calc/candidates.rs`, `backend/src/service/deposits.rs` —
  test names only.
- `Cargo.toml`, `backend/Cargo.toml` — lint table.
- Any future non-ASCII identifier now warns in `cargo check`/`clippy` and
  fails `cargo clippy -D warnings`.
