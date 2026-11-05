# Design

## Context

`service::months` exposes the month-stat pipeline as ~10 public functions whose correct sequencing is the caller's burden: `load_all_months` + `load_all_items` + `load_interest_events` → `wants_live`? → `live_if_needed` → `to_stat_rows` → `live_tail` → `month_derived_with_tail` → `present`. Three callers re-implement it inline; a fourth (`present_month`) re-runs it internally; a fifth (`load_stat_rows_live`) is a half-facade adopted by only `service::year_review`. `live_totals_input` — the aggregate behind Overview!B1/H1 — touches ~10 stores and lives in `service::months` by accident; forecast and overview already reach in for it.

## Goals / Non-Goals

**Goals:**
- One call crosses the seam: every month-derived read opens the book.
- The live-gating protocol becomes private implementation, pinned by tests at the book's interface.
- `live_totals_input` gains a home matching what it computes.
- Zero externally observable change: endpoints, response shapes, and figures identical.

**Non-goals:**
- Moving write workflows out of routes (a separate deepening candidate).
- Moving settings defaults (`goal_constants`, `semi_liquid_target`) — unrelated to the book.
- Lazy/on-demand loading — eager matches today's cost and keeps async out of accessors.

## Decisions

- **Eager `MonthBook` over lazy accessors.** `open` computes live totals only when `wants_live` says a stored NULL or the current-month tail needs them — the existing gate, now internal. `open_with_live` accepts totals a caller already holds (overview, forecast compute live for their own responses), preserving today's unconditional `Some(totals)` semantics: `to_stat_rows` still gates per-row at `live_from`, `live_tail` still gates on last row == current month.
- **Presentation stays pure.** `present`, `month_item_sums`, `month_year_summaries`, `month_running_averages`, `pool_balances` remain functions over the book's fields. `present_one(&stored)` on the book covers the single-row callers (`show`, `upsert`) including the not-found mapping.
- **Replace, don't layer.** `load_stat_rows`, `load_stat_rows_live`, `live_if_needed`, `present_month` are deleted; `wants_live`/`needs_live`/`to_stat_rows`/`live_tail` demoted private. Event loaders and the auto-item helpers (`record_receipt_item`, `delete_auto_item*`) stay public — they feed suggestions and write workflows, a different concern. `live_from` stays public (forecast/overview use it for the window start).
- **`service::wealth` for the aggregate.** `totals_input` (was `live_totals_input`) and `live_totals` (was `live_if_needed`) move verbatim; months' live gate calls `wealth::live_totals` internally. Alternative considered — leave it in months: works, but keeps forecast/overview importing a months module for a wealth concept.
- **Interface is the test surface.** New `connect_memory()` tests pin the gating rules (live fills NULL totals only ≥ current month; tail only when last row == current; `wants_live` fires for the tail case). `backend/tests/api.rs` is untouched — it already covers observable behavior at the HTTP seam.

## Risks / Trade-offs

- [Book always loads all months + items + interest events, even for `present_one` on a single month] → Same cost as today (`present_month` already did exactly that); correct because derivation is running-total over history.
- [A caller forgets the book and rebuilds the pipeline] → Private helpers make mis-sequencing impossible; `open` is the only way to get stat rows.
- [`skip_specs` hides a real behavior delta] → Verified by unchanged `api.rs` integration tests plus response-identical spot check of `/api/months`, `/api/months/summary`, `/api/forecast`.
