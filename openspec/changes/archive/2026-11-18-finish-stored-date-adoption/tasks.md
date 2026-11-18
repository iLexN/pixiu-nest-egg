# Tasks

## 1. Adopt `stored_date` at the hand-rolled sites

- [x] 1.1 `store/bonds.rs` — 5 sites (`maturity_date` ×3, `pay_date` ×2)
- [x] 1.2 `store/trades.rs` — 2 sites (`trade_date`)
- [x] 1.3 `store/aia.rs` — 1 site (`event_date`); `store/ibkr.rs` — 1 site (`transfer_date`)
- [x] 1.4 `store/market_history.rs` — the `sqlx::Error::Decode` map becomes `stored_date("recorded_on", ..)`

## 2. Record the convention

- [x] 2.1 ADR-0005: one line naming `stored_date` the single parse for stored `YYYY-MM-DD` columns, with the leniency/validation boundary

## 3. Verify

- [x] 3.1 `cargo test` — all tests pass; `cargo clippy --all-targets -- -D warnings` + `cargo fmt --check` clean; `grep -rn 'parse_from_str' backend/src/store/` shows only `stored_date`, `parse_stored_month`, test code, and deliberate leniency
