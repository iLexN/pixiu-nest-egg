# Proposal

## Why

`service/months.rs` is 984 lines holding three clusters with three different audiences: the **`MonthBook` read engine** (~350 lines: assemble/present/stat-rows, live-tail logic, and the `load_*_events` loaders that four other services — bonds, deposits, dividends, ibkr — call for the receive protocol, while `wealth` and `year_review` consume the book itself), the **page read models** (~95 lines: `MonthSummaryResponse`/`MonthDetailResponse` + `list`/`summary`/`detail`), and the **write workflows** (~190 lines: `upsert`/`create_item`/`update_item`). Every file in `service/` is flat today only because none has outgrown it; this one has.

## What Changes

- **`service/months.rs` becomes a module root** — `mod book; mod read; mod write;` + `pub use` re-exports, so every existing path (`crate::service::months::{MonthBook, load_events, live_from, list, summary, detail, upsert, create_item, update_item, MonthSummaryResponse, MonthDetailResponse}`) resolves verbatim. **Zero caller changes** across routes and the six consuming services.
- **`service/months/book.rs`** — `MonthBook` + assembly/presentation internals + `live_from` + event loaders; its `#[cfg(test)]` mod carries the `open_*`/`tail`/`pool_balance` tests and is `pub(crate)` so the shared seed helpers (`insert`/`past`/`future`/`month_key`) stay single-sourced.
- **`service/months/read.rs`** — the two response envelopes + `list`/`summary`/`detail`.
- **`service/months/write.rs`** — `upsert`/`create_item`/`update_item` + its 3 tests (importing book's test helpers).
- **Behavior unchanged** — verbatim moves; `skip_specs: true` is set.

## Capabilities

### New Capabilities

None — pure refactor; `skip_specs: true` is set.

### Modified Capabilities

None — module reorganization only; `skip_specs: true` is set.
