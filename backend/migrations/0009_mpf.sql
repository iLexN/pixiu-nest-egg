-- MPF (強積金) accounts: one row per account on the workbook's MPF sheet.
-- seed_max_rate/seed_max_gain are the imported all-time high-water marks;
-- the reported max is derived on read as MAX(seed, history, current), so
-- deleting a mistyped history row recomputes cleanly.
CREATE TABLE mpf_accounts (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    label         TEXT    NOT NULL,
    trustee       TEXT,
    contributions REAL    NOT NULL DEFAULT 0,
    balance       REAL    NOT NULL DEFAULT 0,
    plan_name     TEXT,
    contract_no   TEXT,
    member_no     TEXT,
    sort_order    INTEGER NOT NULL,
    seed_max_rate REAL,
    seed_max_gain REAL,
    created_at    TEXT    NOT NULL,
    updated_at    TEXT    NOT NULL
);

-- Every balance/contribution update appends a row; same-day edits replace it.
-- Synthetic rows backfill months with no update so "last month" never gaps.
CREATE TABLE mpf_history (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id    INTEGER NOT NULL REFERENCES mpf_accounts (id),
    recorded_on   TEXT    NOT NULL,
    contributions REAL    NOT NULL,
    balance       REAL    NOT NULL,
    synthetic     INTEGER NOT NULL DEFAULT 0,
    UNIQUE (account_id, recorded_on)
);

-- Small key-value store for section-level state: the MPF page note and the
-- portfolio-level seeded maxima that per-account history cannot rebuild.
CREATE TABLE app_meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
