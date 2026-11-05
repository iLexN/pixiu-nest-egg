# Proposal

## Why

The month-stat read pipeline's protocol leaks into its callers. Four call sites (`routes::months::{list,summary}`, `routes::forecast::forecast`, plus `service::months::present_month`) each hand-assemble the same sequence — load months + items + interest events, gate on `wants_live`, compute live totals, build stat rows, derive the tail — about ten calls across the seam that the caller must sequence correctly. `service::months` also hosts `live_totals_input` (the Overview!B1/H1 總資產/流動資產 aggregate), which forecast and overview consume even though it is not a months concept.

## What Changes

- **`service::months` gains `MonthBook`** — `{ stored, stat_rows, items, live, tail }` — with `open(pool)` and `open_with_live(pool, totals)` as the interface. The pipeline's protocol (`wants_live`, `needs_live`, `to_stat_rows`, `live_tail`) becomes private implementation; superseded entry points (`load_stat_rows`, `load_stat_rows_live`, `live_if_needed`, `present_month`) are removed.
- **New `service::wealth` module** takes `live_totals_input` (renamed `totals_input`) and `live_if_needed` (renamed `live_totals`) out of `service::months`.
- **All consumers migrate**: `routes::months::{list,summary,show,upsert}`, `routes::forecast::forecast`, `service::overview::build`, `service::year_review::build`, and the `routes::deposits` tests.
- **Gating-rule tests** pin the live-fill and tail rules at the book's interface over `connect_memory()`.
- **`GLOSSARY.md`** is created with the *Month book* term.

No externally observable behavior changes: same endpoints, same figures.

## Capabilities

### New Capabilities

None — pure refactor; `skip_specs: true` is set.

### Modified Capabilities

None — no requirement changes; `skip_specs: true` is set.

## Impact

- `backend/src/service/months.rs`, new `backend/src/service/wealth.rs`, `backend/src/service/mod.rs`
- `backend/src/routes/{months,forecast,deposits}.rs`, `backend/src/service/{overview,year_review}.rs`
- `GLOSSARY.md` (new), `docs/DATA_FLOW.md`
