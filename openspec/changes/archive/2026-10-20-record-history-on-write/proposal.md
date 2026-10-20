# Proposal

## Why

Computing a market summary writes `market_history`. That happens on `GET /api/summary`, on the yearly table, on the year review, and on freeze. As a result, history depends on which pages the user happened to open. Days with no change still get duplicate rows (live data has 10-05/10-06, 10-13/10-14 and 10-15/10-20 with identical values). A high that existed between two page views is never recorded, so 最高 can miss it.

Market totals change only on writes, because 現價 is entered or imported by hand. Recording on writes therefore captures every state exactly, and reads can become pure.

Separately, 上月 for both markets and MPF reads "the latest row in the previous calendar month". That only works because synthetic month-end rows are backfilled. An as-of lookup gives the same answer without needing them.

## What Changes

- **Market history records on writes, not reads.**
  - After any successful write to stocks or trades, and after a price import (UI or CLI), the system records each affected market's totals for today.
  - A row is added only when the totals differ from that market's latest row; same-day writes still replace that day's row.
  - **BREAKING (behavior):** computing or viewing a summary, the yearly table, the year review, or freezing a year no longer writes history.
- **上月 becomes an as-of lookup** for markets and MPF accounts/portfolio: the latest history row dated on or before the last day of the previous calendar month. A month with no writes no longer blanks 上月.
- **No more synthetic month-end backfill** for markets or MPF, because the as-of lookup makes it unnecessary.
  - For an MPF account whose first contributions/balance update arrives while it has no history yet, the update first records a baseline row carrying the pre-update values, dated the account's creation day and marked synthetic. This keeps those values in the as-of record.
  - Existing synthetic rows stay and remain valid.
- **最高 is unchanged in definition:** seed marks, every history row, and the current values. It becomes more accurate, because every write is now captured.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `stock-portfolio-summary`:
  - "Market totals history" changes from recording on summary computation to recording on writes that may change totals, and adds the skip-unchanged rule.
  - "Month-end history backfill" is removed.
  - "Last-month and max unrealized figures" switches 上月 to the as-of lookup.
- `mpf-accounts`:
  - "Month-rollover backfill" is replaced by the first-update baseline row.
  - "Automatic last-month and max figures" switches the last-month figures to the as-of lookup.

## Impact

- **Backend:**
  - `service/summary.rs` (`build` becomes pure; new `record_market`)
  - `store/market_history.rs` (`record` without backfill, plus skip-unchanged)
  - `store/mpf.rs` (`record_history` without backfill, plus a baseline row)
  - `calc/mpf.rs` (`mpf_last_month` as-of; `mpf_gap_month_ends` removed if unused)
  - routes for stocks and trades (post-write hook)
  - `prices.rs` (hook instead of `build`)
  - `service/yearly.rs`, `routes/yearly.rs` (freeze), `service/year_review.rs` (pure market value)
- **Data:** no migration and no rows deleted.
- **Frontend:** none. The 上月 and 最高 fields keep their shape.
- **Docs:** `docs/database.md`, `docs/stocks.md`, `docs/investments.md`.
