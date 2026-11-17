# Tasks

## 1. Tighten the loader interface in `service/months.rs`

- [x] 1.1 Make `load_deposit_events`, `load_dividend_events`, `load_coupon_events` private `fn` — zero callers outside the file.
- [x] 1.2 Change `load_events` and `load_interest_events` to `pub(crate)`.
- [x] 1.3 Drop the `pool_input` parameter from `load_events`; inside it, load `store::months::load_month(pool, &month.to_string())` and use `.map(|m| m.pool_input).unwrap_or(0.0)`.

## 2. Update call sites

- [x] 2.1 `routes/months.rs::show` — call `load_events(&state.pool, month)`; drop the `stat.pool_input` argument.
- [x] 2.2 `service/{deposits,bonds,dividends,ibkr}.rs` test helpers — drop the `0.0` argument at each `load_events` call site.

## 3. Verify

- [x] 3.1 `cargo test` — the parity tests still pass with the self-loaded `pool_input` (they seed no `pool_input`, so the `0.0` default is identical).
- [x] 3.2 `cargo clippy --all-targets -- -D warnings && cargo fmt --check`.
