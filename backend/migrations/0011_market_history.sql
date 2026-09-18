-- Per-market totals history: one row per market per day carrying the priced
-- cost basis and market value, from which 上月 (latest row in the previous
-- calendar month) and 最高 (all-time maxima) unrealized figures are derived
-- on read. Same-day rebuilds replace the day's row; synthetic rows backfill
-- months with no record so last month never gaps, and seed year-end rows
-- from year_snapshots give max real history at import.
CREATE TABLE market_history (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    market          TEXT    NOT NULL CHECK (market IN ('HK', 'US')),
    recorded_on     TEXT    NOT NULL,
    buy_cost_priced REAL    NOT NULL,
    market_value    REAL    NOT NULL,
    synthetic       INTEGER NOT NULL DEFAULT 0,
    UNIQUE (market, recorded_on)
);
