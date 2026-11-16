# Tasks

## 1. Enable the lint

- [x] 1.1 Add `[workspace.lints.clippy] option_if_let_else = "warn"` to the root `Cargo.toml`; verify `cargo clippy` lists all 16 violations.

## 2. Rewrite the Option-match sites

- [x] 2.1 Simple sites → `unwrap_or`/`map_or`/`is_some_and`: routes/patch.rs (`merge_num`, `merge_text`), service/months.rs and service/mpf.rs (`merge_text` copies), routes/stocks.rs (`patched_trimmed`), service/wealth.rs (`pool_balance`), calc/suggestions.rs (`suggestions_enabled`).
- [x] 2.2 Complex arms → `map_or_else` keeping inner matches/`zip`/`and_then`: calc/months.rs (`total_change`, `liquid_change`), routes/mod.rs (`parse_status`), service/ibkr.rs (`transfer_date`).
- [x] 2.3 Vec get-or-insert → `iter().position().unwrap_or_else` + `&mut [index]`: calc/deposits.rs ×3, calc/dividends.rs, calc/stocks.rs.

## 3. Verification

- [x] 3.1 `cargo test` — all pass.
- [x] 3.2 `cargo clippy --all-targets -- -D warnings` — clean.
- [x] 3.3 `cargo fmt` then `cargo fmt --check` — clean.

## Workflow follow-up

- Archive the change (`openspec archive`) and sync delta specs into `openspec/specs/`.
