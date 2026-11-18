# Proposal

## Why

`backend/tests/api.rs` is one 2831-line file interleaving 46 tests across every domain the API serves, with 16 helpers scattered between them (`send` ×226 call sites, `approx` ×126, `create_stock` ×30). The helpers are a de facto interface — every test leans on them — but they have no named home, and finding a test means scrolling or grepping a file ordered by when its section was written. Adding a test means appending to the pile.

## What Changes

- **`tests/common/mod.rs`** — all 16 helpers move in, one flat module: app wiring (`app`, `app_with_pool`), HTTP (`send`), assertions (`approx`), seeding/reads (`seed_cash`, `cash_amount`, `history_rows`, `history_count`), creators (`create_stock`, `create_stock_with_ticker`, `create_deposit`, `create_buy`, `create_family_deposit`, `set_price`, `buy_trade`), `ym_add`. The absolute rule: helpers live in `common`, domain files own only tests — so anything a second file needs is already there.
- **13 domain test files** — separate test binaries, one per API surface: `stocks`, `trades`, `summary`, `deposits`, `dividends`, `family`, `forecast`, `months`, `cash_accounts`, `settings`, `year_review`, `errors` — plus two thematic files for the cross-cutting protocols: `receives` (receive → credit cash → record month item across deposit/bond/coupon/dividend + reversal-follows-account) and `market_history` (the audit-trail contract across manual patches, trades, freeze). Each file does `mod common;` + `use common::*;`.
- **`tests/api.rs` is deleted** — nothing remains at the root.
- **Behavior unchanged** — tests move verbatim; names, bodies, and the 46-test count are identical; `cargo test` output stays grouped by binary. No production code is touched.

## Capabilities

### New Capabilities

None — pure refactor; `skip_specs: true` is set.

### Modified Capabilities

None — test-suite reorganization only; `skip_specs: true` is set.
