# Design

## Context

See proposal.md (Why). Current mechanics:

- **Market history writes.** `service::summary::build(pool, market)` summarizes, then calls `store::market_history::record`. `record` backfills a synthetic row for every fully elapsed empty month (via `calc::mpf_gap_month_ends`) and upserts today's row. `build` is called by:
  - `routes/summary.rs` (GET)
  - `service/yearly.rs::build` (GET yearly; also reached from `service/year_review.rs`)
  - `routes/yearly.rs` freeze
  - `prices.rs::apply` (bulk import, UI and CLI)
- **Pure market value.** `service::summary::market_value_only` already exists with no side effect.
- **Write paths that can change a market's totals:**
  - `routes/stocks.rs`: `create`, `update` (`manual_price`; `is_active` does not affect totals, per spec); `remove` is only allowed with no trades or dividends, so it cannot change totals today but is hooked anyway.
  - `routes/trades.rs`: `create`, `update` (may change `stock_id` across markets), `remove`.
  - `prices.rs::apply`.
- **MPF history.** `store::mpf::record_history(conn, account_id, previous, current, today, fallback_on)` runs inside the update transaction from `routes/mpf.rs`. It backfills gap month-ends from the latest row (or from `created_on` when there is no history), then upserts today's row.
- **上月.** `calc::mpf_last_month` reads "the latest row in the previous calendar month". It is used by `service/mpf.rs` (per account) and `service/summary.rs` (per market). The MPF portfolio figures (`calc::mpf_totals`) already use an as-of merge (`mpf_as_of`) at the previous month-end, falling back to current values for an account with no eligible row.
- **Live data** (read-only check): `market_history` HK has 21 rows (4 synthetic) and US has 18 (1 synthetic); both MPF accounts have history from `2026-08-31`.

## Goals / Non-Goals

**Goals:**
- No GET request writes `market_history`.
- History holds one row per change, and every write state is captured.
- 上月 stays correct when a month passes with no writes, for markets and MPF.
- Today's 上月 and 最高 values are unchanged on the live database.

**Non-Goals:**
- Deleting or rewriting existing history rows (duplicates and synthetic rows stay).
- Changing 最高, the seed marks, or the MPF portfolio as-of merge.
- Exposing market history in the UI.

## Decisions

### D1. One post-commit hook: `service::summary::record_market(pool, market)`

`record_market` summarizes the market through the existing side-effect-free path. If `totals.market_value` is present, it calls `store::market_history::record(pool, market, (buy_cost_priced, market_value), today)`.

It is called after the write commits:
- in the six stock/trade route handlers;
- in `prices::apply`, once per touched market, replacing the current `build` call.

`trades::update` collects the market of the old stock and the market of the new stock, and records each distinct one. `build` stops calling `record` and becomes pure. Yearly, year review and freeze keep calling `build`, or switch to `market_value_only`; either way they are now pure.

*Why post-commit rather than inside the write transaction:* summarizing reads three stores through `&SqlitePool`, and threading a transaction through them is a large refactor for no gain. If recording fails after the commit, the route returns the error while the write itself stands. The next write records the correct totals anyway, because recording compares against the latest row, not against a delta. That is the same failure shape `prices::apply` has today.

*Alternative rejected:* recording inside each store write. That would put a summary calculation in `store/` and break the layer rules.

### D2. Skip unchanged; same-day replace stays

`store::market_history::record` behaves as follows:
1. Load the market's latest row by `recorded_on DESC, id DESC`.
2. If it exists and both values are exactly equal (`f64 ==`, the stored REAL round-trips exactly), do nothing.
3. Otherwise upsert today's row (`ON CONFLICT (market, recorded_on) DO UPDATE`, `synthetic = 0`).

There is no backfill loop.

### D3. 上月 as-of, in the shared calc

`calc::mpf_last_month(history, today)` changes its filter from "same year and month as the previous month" to `recorded_on <= prev_end`, taking the latest row. It does not fall back to current values. Markets have no creation date, and an MPF account with no history keeps today's empty 上月. Both the per-account MPF figures and the market figures pick this up. `calc::mpf_totals` is untouched.

If `calc::mpf_gap_month_ends` has no callers left after D2 and D4, it is removed along with its tests.

Equivalence on live data, today `2026-10-20` with previous month-end `2026-09-30`:
- HK and US both have a `2026-09-30` row, so the old and the new rule pick the same row.
- The MPF accounts have rows through `2026-10-08` with September covered, so they also match.

### D4. MPF first-update baseline replaces backfill

`store::mpf::record_history` works as follows:
1. If the account has no history rows and `fallback_on` (its creation date) is earlier than `today`, insert `(account_id, fallback_on, previous.0, previous.1, synthetic = 1)`.
2. In every case, upsert today's row as now.
3. Delete the gap-month loop.

If the account was created today, step 1 is skipped: the baseline would share the date of today's row and be replaced by it. The baseline uses the creation date because the pre-update values have stood since the account was created. The as-of merge treats the row as covering every date until the next one, which is what the synthetic month-end rows used to approximate.

## Risks / Trade-offs

- **[A forgotten write path silently stops recording]**
  - Mitigation: the spec lists the write kinds.
  - Tests in `backend/tests/api.rs` drive each endpoint (stock price PATCH, trade create/update/delete, price upload) and assert a history row for the affected markets.
  - A test asserts that GETs on summary, yearly and year-review write nothing.
- **[Post-commit failure]** A write succeeds but recording fails, so the user sees an error. This is rare (SQLite local) and self-heals on the next write, as D1 explains.
- **[Intra-day peaks]** Two writes on one day keep only the last state, as today. This is accepted: the spec already defines same-day replace.
- **[Exact float comparison]** The latest row's values come from the same summarization arithmetic, so unchanged inputs give bit-identical results. If arithmetic ever differed, a duplicate row would be recorded, which is harmless.
- **[Existing duplicate rows]** These remain. They do not change any 上月 or 最高 value.

## Migration Plan

No schema migration. Deploy by restarting the backend. Rollback is reverting the code; no data shape changes.
