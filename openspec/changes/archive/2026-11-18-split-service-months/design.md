# Design

## Decisions (from grilling)

- **D1 — Directory module with `mod.rs`-style root, not flat siblings.** `service/months.rs` stays the module root (Rust 2018: `months.rs` + `months/` children, no `mod.rs` needed) declaring `mod book; mod read; mod write;` and re-exporting the full public surface. `crate::service::months::X` resolves identically for `routes/months.rs`, `service/{bonds,deposits,dividends,ibkr,wealth,year_review,forecast}.rs` — zero call-site churn. _Rejected_: flat `months_book.rs`/`months_read.rs`/`months_write.rs` — uglier names and still needs a `months` namespace shim for the `service::months::X` paths.
- **D2 — Seam = audience.** `book` is the cross-domain read engine (book + loaders used by other services); `read` is the 月結 page surface; `write` is the mutation workflows. Writes touch the book exactly once (`upsert` re-reads via `MonthBook::open` after mutating — stays as a `super::book::MonthBook` call).
- **D3 — Tests follow the code they cover, helpers stay single-sourced.** Book tests (`open_*`, `tail_*`, `pool_balance_*`, `months_and_month_present`) need `MonthBook`'s **private fields** (`stat_rows`, `tail`, `live`) — they must live in `book.rs`. Write tests (`upsert_create_*`, `create_item_*`, `update_item_*`) go to `write.rs` and import the shared seed helpers via `book`'s `pub(crate) mod tests` — no duplication, no loosened field visibility.

## File map

| New file | Contents | Lines |
|---|---|---|
| `service/months.rs` (root) | `mod` declarations + `pub use` re-exports + module doc | ~15 |
| `service/months/book.rs` | `live_from`, `MonthBook` + `open`/`open_with_live`/`assemble`/`present_at`/`months`/`month`/`year_summaries`/`running_averages`/`trailing_averages`/`pool_balance`/`pool_rate`, `stat_rows_no_live`, `to_stat_rows`, `present`, `needs_live`, `wants_live`, `live_tail`, `load_deposit_events`/`load_dividend_events`/`load_coupon_events`/`load_interest_events`/`load_events` + `pub(crate) mod tests` (seed helpers + book tests) | ~670 |
| `service/months/read.rs` | `MonthSummaryResponse`, `MonthDetailResponse`, `list`, `summary`, `detail` | ~95 |
| `service/months/write.rs` | `upsert`, `create_item`, `update_item` + `mod tests` (3 tests + `empty_patch`/`item` helpers) | ~280 |

## Visibility notes

- `load_events`, `load_interest_events`, `stat_rows_no_live` are `pub(crate)` today — keep identical visibility, re-exported through the root so `crate::service::months::load_events` still resolves.
- Items in `book`/`read`/`write` that reference each other switch to `super::book::…`/`super::read::…` paths; the root's `pub use` hides the structure from outside.
- `MonthBook` fields stay private — book tests compile only inside `book.rs` (D3).

## Risks / Trade-offs

- **[A re-export gets missed]** → compile errors on every existing path; `cargo build` is the guard — zero-churn is mechanically verified, not eyeballed.
- **[First `service/` subdirectory]** → new convention by example only; documented in the change; if a second service file outgrows flat it has the precedent.
- **[`use` header drift between the three files]** → each file imports exactly what it uses; `cargo clippy` catches unused imports.
