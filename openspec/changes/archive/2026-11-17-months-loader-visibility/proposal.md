# Proposal

## Why

`service::months` exposes five `pub` event loaders beside `MonthBook`. Three — `load_deposit_events`, `load_dividend_events`, `load_coupon_events` — are one-line delegations to store with zero callers outside the file. Two — `load_events`, `load_interest_events` — are real compositions, but every caller is inside the crate (`routes/months::show`, `service::forecast::build`, and the candidate-parity tests in `service::{deposits,bonds,dividends,ibkr}`), so `pub` overstates the interface. `load_events` also takes a `pool_input` parameter its only production caller had already loaded and every test caller fakes with `0.0`.

Over-wide visibility invites the "call whichever variant is nearest" bypass the MonthBook learnings warn about; an inventable parameter lets callers pass a wrong value the module could have known itself.

## What Changes

- `load_deposit_events`, `load_dividend_events`, `load_coupon_events` become private — plumbing shared by the two composers, not interface.
- `load_events` and `load_interest_events` become `pub(crate)` — reachable by the route, forecast, and in-crate test modules; no longer presented as API.
- `load_events(pool, month, pool_input)` becomes `load_events(pool, month)`: it reads `month_stats.pool_input` itself, defaulting to `0.0` when the month has no row — the value test callers already pass.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None — pure refactor; the month detail and every suggestion stay byte-identical.

## Impact

- `backend/src/service/months.rs` — visibility and signature changes only.
- `backend/src/routes/months.rs` — `show` drops the `stat.pool_input` argument.
- `backend/src/service/{deposits,bonds,dividends,ibkr}.rs` — test call sites drop the `0.0` argument.
- No API, schema, or behavior change. Verified by `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`.
