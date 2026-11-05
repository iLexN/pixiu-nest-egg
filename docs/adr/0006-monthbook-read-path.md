# MonthBook is the only month-read path

Every month-derived read (list, summary, detail, forecast, overview, year-review) opens `service::months::MonthBook` — `open(pool)` computes live totals only when `wants_live` says a stored NULL or the current-month tail needs them; `open_with_live(pool, totals)` takes totals a caller already holds. The pipeline's sequencing (`wants_live`/`to_stat_rows`/`live_tail`) is private — the earlier public helpers were deleted, not layered over, because parallel interfaces invite bypass. The 總資產/流動資產 aggregate lives in `service::wealth` (`totals_input`/`live_totals`), not `service::months` — forecast and overview consume it but it isn't a months concept.

### Considered options
- Lazy accessor methods — rejected: async-in-accessors friction for no savings; `wants_live` already skips the ~10-store live compute when unneeded.
- Keep `load_stat_rows*` public beside the book — rejected: a second interface gets copied by proximity, which is how the half-adopted `load_stat_rows_live` happened.

New month reads: open the book; never reload months/items/events or rebuild the live gate in a route or service.

Source: `openspec/changes/archive/2026-11-05-month-book`.
