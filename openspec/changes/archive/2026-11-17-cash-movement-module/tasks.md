# Tasks

## 1. Rename + new verbs

- [x] 1.1 `service/receipts.rs` → `service/cash_movement.rs`; `mod` + caller repoints (bonds, deposits, dividends)
- [x] 1.2 `debit_bank(conn, account_id, amount, field)` — validate + apply + return account
- [x] 1.3 `apply_to_broker(conn, currency, delta)` — broker lookup + `add_amount`
- [x] 1.4 `shift(conn, Option<i64>, delta)` — tolerant delta application

## 2. Adopt in the writers

- [x] 2.1 `deposits::place` uses `debit_bank`
- [x] 2.2 `ibkr::create_transfer` uses `debit_bank` + `apply_to_broker`; `delete_transfer` uses `shift`
- [x] 2.3 `trades` uses `apply_to_broker` (drop `usd_broker_id`)

## 3. Verify

- [x] 3.1 `cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check`
- [x] 3.2 Restart backend; spot-check a deposit placement + an ibkr transfer + a US trade

## Workflow follow-up

- Archive with `openspec archive` (skip_specs — nothing to sync)
