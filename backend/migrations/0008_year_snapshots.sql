-- Frozen per-(market, year) figures for the yearly summary table: the values
-- the owner used to copy as raw numbers in the spreadsheet at year end.
-- A NULL column means no override is stored — the yearly row then shows the
-- figure recomputed from trades/dividends (or, for a past year's
-- market_value, nothing).
CREATE TABLE year_snapshots (
    market       TEXT    NOT NULL CHECK (market IN ('HK', 'US')),
    year         INTEGER NOT NULL,
    invested     REAL,
    cost         REAL,
    market_value REAL,
    updated_at   TEXT    NOT NULL,
    PRIMARY KEY (market, year)
);
