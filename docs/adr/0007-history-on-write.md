# Market/MPF history recorded on write, never on read

`market_history` (and MPF history) rows are written only by write paths — stock/trade creates/edits/deletes, bulk price import — via `service::summary::record_market` **after** the write commits. No GET, view, summary, yearly, review, or freeze path writes history. Rows are one-per-day-per-market (same-day replace) and only when `buy_cost_priced`/`market_value` actually changed; months without writes carry forward through the as-of lookup, and unpriced markets record nothing.

### Considered options
- Recording inside the write transaction — rejected: summarizing reads three stores through `&SqlitePool`; threading a tx through them is a large refactor for no gain, and the next write self-heals because recording compares against the latest row, not a delta.
- Recording inside each store write — rejected: puts a summary calculation in `store/` and breaks the layer rules.
- Reads writing history — this was the old behavior (`GET /api/summary` wrote `market_history`); deliberately removed.

Source: `openspec/changes/archive/2026-10-20-record-history-on-write`.
