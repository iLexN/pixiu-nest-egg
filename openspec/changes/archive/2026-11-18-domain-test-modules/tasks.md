# Tasks

## 1. Extract the shared interface

- [x] 1.1 Create `tests/common/mod.rs` with the shared `use` header and all 16 helpers (`app`, `app_with_pool`, `send`, `approx`, `seed_cash`, `cash_amount`, `history_rows`, `history_count`, `create_stock`, `create_stock_with_ticker`, `create_deposit`, `create_buy`, `create_family_deposit`, `set_price`, `buy_trade`, `ym_add`), made `pub`, moved verbatim

## 2. Create the domain binaries

- [x] 2.1 `tests/stocks.rs` (5), `tests/trades.rs` (3), `tests/summary.rs` (2), `tests/market_history.rs` (4) — per the design map
- [x] 2.2 `tests/deposits.rs` (4), `tests/receives.rs` (7), `tests/dividends.rs` (5), `tests/family.rs` (5) — per the design map
- [x] 2.3 `tests/forecast.rs` (3), `tests/months.rs` (1), `tests/cash_accounts.rs` (1), `tests/settings.rs` (3), `tests/year_review.rs` (2), `tests/errors.rs` (1) — per the design map
- [x] 2.4 Delete `tests/api.rs`; every moved test keeps its exact name and body

## 3. Verify

- [x] 3.1 `cargo test` — all 46 tests pass, now distributed across the per-domain binaries; `cargo clippy --all-targets -- -D warnings` + `cargo fmt --check` clean
