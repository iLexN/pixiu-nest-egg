-- YearInReview: one stored record per year for the figures the sheet enters
-- by hand (收入, the invested add-on) plus nullable overrides for the cells
-- whose history was deleted from the workbook (pre-app bonds and deposits).
-- NULL columns derive live, matching the year_snapshots convention.
CREATE TABLE year_review (
    year                INTEGER PRIMARY KEY,
    income              REAL,
    invested_adjustment REAL,
    bond_principal      REAL,
    bond_interest       REAL,
    deposit_principal   REAL,
    deposit_interest    REAL,
    updated_at          TEXT    NOT NULL
);

-- 賣出損益 per (market, year): the workbook never recorded SELL trades, so the
-- figure stays manual. NULL = not entered; the yearly row and YearInReview's
-- 投資P/L report it absent.
ALTER TABLE year_snapshots ADD COLUMN sold_pl REAL;
