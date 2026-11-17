# Proposal

## Why

`service::overview::build` and `service::forecast::build` each restate the
same preamble: `wealth::totals_input` → `live_totals` → `Settings::load`
(loaded 3× per overview build — directly, inside `semi_liquid_target`, inside
`goal_constants`) → `MonthBook::open_with_live`. And inside the overview
build, `ibkr_block` re-loads five things `totals_input` just loaded: the two
broker accounts, the USD rate, the US market value, and `Settings`.

## What Changes

- `service::wealth::LiveContext` — `open(pool)` eagerly loads
  `input: LiveTotalsInput`, `totals: LiveTotals`, `stored: Settings`,
  `book: MonthBook` (opened `with_live(totals)`), plus the Option-preserving
  `IbkrReads` (`us_market_value`, `hkd_cash`, `usd_cash`) that `totals_input`
  would otherwise collapse to `f64`.
- `totals_input` splits: `ibkr_reads(pool)` + `totals_input_with(pool, reads)`;
  the public `totals_input(pool)` delegates unchanged.
- `ibkr_block` splits into load + compose; `build` calls a context-fed
  variant that loads only `transferred_hkd`.
- `service::settings` gains pure `semi_liquid_target_of(&Settings)` and
  `goal_constants_of(&Settings)`; the async wrappers delegate to them.
- `overview::build`/`forecast::build` open one `LiveContext`; `months::upsert`
  keeps calling `totals_input` directly — a write path has no use for the book.
- No behavior change: same responses; `null`/`0.0` distinctions preserved.

## Capabilities

### New Capabilities

None — pure refactor; `skip_specs: true` is set.

### Modified Capabilities

None.

## Impact

- `backend/src/service/wealth.rs` — `LiveContext`, `IbkrReads`, split loads.
- `backend/src/service/overview.rs` — `build` consumes context; `ibkr_block`
  decomposed.
- `backend/src/service/forecast.rs` — `build` consumes context.
- `backend/src/service/settings.rs` — pure `*_of` helpers.
