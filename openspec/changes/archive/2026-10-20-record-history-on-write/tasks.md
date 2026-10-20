# Tasks

## 1. Baseline

- [x] 1.1 Before any code change, copy `data/wealth.db` to `/tmp/rh-src.db`. Run the HEAD backend on a copy of it (spare port) and save `/api/summary?market=HK`, `/api/summary?market=US` and `/api/mpf` under `/tmp/rh-baseline/`. Then record the row counts: `SELECT market, count(*) FROM market_history GROUP BY market` on that copy, after the GETs. Verify: three non-empty JSON files and the counts are saved.

## 2. As-of 上月 and history recording

- [x] 2.1 Change `calc::mpf_last_month` to the latest row with `recorded_on <= previous month-end` (design D3). Verify with inline tests: the existing previous-month test passes; a new test with rows only on `2026-08-28` viewed on `2026-10-05` returns the August figures; rows only in the current month return none.
- [x] 2.2 Rewrite `store::market_history::record` per D2: no backfill; skip when the latest row's values are equal; same-day upsert otherwise. Verify with inline tests: unchanged totals add nothing; changed totals on a new day add one row; same-day change replaces; gap months create no synthetic rows. The existing `first_record_writes_no_backfill` and `same_day_rebuilds_replace_the_row` tests still pass.
- [x] 2.3 Rewrite `store::mpf::record_history` per D4: a baseline row dated `fallback_on` (synthetic) only when the account has no history and `fallback_on < today`; no gap loop. Verify with inline tests covering both scenarios of the "First-update baseline row" requirement and the same-day-creation case (no extra row). Remove `calc::mpf_gap_month_ends` and its tests if it has no callers left.

## 3. Hooks and pure reads

- [x] 3.1 Add `service::summary::record_market(pool, market)` (D1). Remove the `record` call from `build`, and make `prices::apply` call `record_market` per touched market. Verify: `service::summary` tests are updated so `build` writes nothing and `record_market` writes the row; the `prices.rs` test for both markets still passes.
- [x] 3.2 Call `record_market` after the write commits in `routes/stocks.rs` (`create`, `update`, `remove`) and `routes/trades.rs` (`create`, `update` covering both the old and new stock's market, `remove`). Verify with integration tests in `backend/tests/api.rs`:
  - a 現價 PATCH records an HK row;
  - a trade create/delete records;
  - moving a trade from an HK stock to a US stock records both markets;
  - a note-only stock PATCH records nothing;
  - repeated `GET /api/summary`, `GET /api/summary/yearly`, `GET /api/year-review` and a yearly freeze leave `market_history` unchanged.
- [x] 3.3 Run `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`. Verify: all three pass.

## 4. Docs

- [x] 4.1 Update `docs/database.md` (`market_history`, `mpf_history`: recorded on writes, no synthetic month-end backfill, first-update baseline, `synthetic` meaning), `docs/stocks.md` (the history recording flow), and `docs/investments.md` (the MPF update flow and 上月 as-of). Verify: grepping `docs/` for "backfill" only finds wording that describes historical rows.

## 5. Integration checks

- [x] 5.1 On a fresh copy of `/tmp/rh-src.db`, run the new backend (spare port) and fetch the same three endpoints into `/tmp/rh-after/`. Compare the `last_month`/`max` fields for HK and US and the MPF per-account and portfolio last-month/max figures against `/tmp/rh-baseline/`. Then re-run the `market_history` count query after the GETs. Verify: every compared figure is identical, and no `market_history` rows were added by the GETs (after-run counts equal `/tmp/rh-src.db`'s own counts).
- [x] 5.2 Restart the real backend per AGENTS.md and confirm `/api/summary?market=HK` answers. Verify: opening the summary twice adds no `market_history` row.
