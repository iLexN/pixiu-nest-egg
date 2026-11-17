# Design

## Context

See proposal.md — Why. The loader family sits in `service/months.rs` at `service/months.rs:291-364`. The candidate-parity tests (`suggestion_of` helpers in `service::{deposits,bonds,dividends,ibkr}` `#[cfg(test)]` modules) are the only non-month-read consumers of `load_events`; they rebuild the suggestion picture to assert it equals the recorded item — a sanctioned internal test seam, not a second public interface.

## Goals / Non-Goals

- Goals: the module's public surface states what is actually API (the `MonthBook` + write workflows); callers can't invent `pool_input`.
- Non-goals: relocating the loaders (their only production consumer is the month read — placement stays); renaming `load_events`/`load_interest_events`; touching `stat_rows_no_live` (already `pub(crate)` for the live-totals recursion constraint).

## Decisions

- **`pub(crate)`, not `pub`, for the two composers.** Callers are the route (same crate), `service::forecast` (same crate), and `#[cfg(test)]` modules in sibling service files (same crate). `pub` would claim the loaders are part of the crate's external API — `export_openapi`/`import_prices` bins are the only potential out-of-crate users and neither needs them. `pub(crate)` keeps the parity-test seam reachable without overstating the interface.
- **Private, not `pub(crate)`, for the three singular loaders.** Zero callers outside `service/months.rs`; they exist so `load_events` and `load_interest_events` read uniformly. Deletion test: making them private costs nothing, exposing them buys nothing.
- **`pool_input` self-loads via `store::months::load_month`.** The only production caller passes `stat.pool_input`, which `present()` copies straight from the stored row — the self-load returns the identical value. A missing month row defaults to `0.0`, which is what every test caller passes today. This trades one extra indexed read (local SQLite, negligible) for a parameter no caller can get wrong.
- **Not moving `load_events` into `routes` or a `service::suggestions` module.** A `service::suggestions` would hold exactly one production consumer; the candidate-catalogue learning's "no `service::candidates` pass-through" objection does not apply here (this is composition, not pass-through), but relocation still buys nothing while `show`'s read model hasn't moved.

## Open Questions

None.
