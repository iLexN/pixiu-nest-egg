# Design

## Context

Deferred from `forecast-grid-module` ("architecture review round 2, #3"): the
總覽 and 預測 page reads restate the same ~5-load preamble, and each pays for
duplicate loads inside it (Settings ×3, broker accounts ×2, rate ×2, US market
value ×2 per overview build).

## Goals / Non-Goals

**Goals:**
- One `LiveContext::open` per page build; the shared plan answers in one place.
- `ibkr_block`'s derivation composes from context reads — `build` only loads
  `transferred_hkd` extra.
- Observable `Option` semantics preserved: `IbkrBlock.hkd_cash`/`usd_cash`/
  `stock_value_usd` stay `None` when the row is absent, not `0.0`.

**Non-Goals:**
- No response or error changes; `months::upsert` and route tests keep
  `totals_input`/`live_totals`.
- No concurrency restructuring — loads stay sequential.

## Decisions

### D1: `LiveContext` lives in `service::wealth`

`service::wealth` already owns `totals_input` ("the same aggregation the later
endpoint reuses") — the context is that aggregation plus the things both
pages layer on: `stored: Settings` and `book: MonthBook`.

### D2: `IbkrReads` preserves Option-ness

`totals_input` unwraps `us_market_value`/broker cash to `0.0` for
`live_totals`, but `IbkrBlock` reports `None`. `ibkr_reads(pool)` loads the
three Option values once; `totals_input_with(pool, &reads)` consumes them for
the input; the context keeps `reads` for `ibkr_block`'s compose step.

### D3: `ibkr_block` splits into load + compose

`compose_ibkr_block(transferred_hkd, now_value, hkd_cash, usd_cash, rate,
stock_value_usd)` holds the derivation. `ibkr_block(pool)` keeps its
signature (route + `update_ibkr`); `build` calls
`ibkr_block_from(pool, &ctx)` = `transferred_hkd` + compose.

### D4: Settings-derived values come from `stored`

`semi_liquid_target_of`/`goal_constants_of(&Settings)` are pure projections;
the async wrappers delegate, and the page builds call the `_of` forms —
one `Settings::load` per build instead of three.

## Risks / Trade-offs

- `LiveContext` eagerly loads `MonthBook` — right for both pages today; a
  future caller wanting totals-only still has `totals_input`.
