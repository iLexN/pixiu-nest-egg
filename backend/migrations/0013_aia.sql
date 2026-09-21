-- AIA policies: one row per policy line in the AIA sheet's D-O table. The
-- sheet keeps single cumulative figures (buy usd / now usd / Withdrew), so the
-- table stores the same plus the two totals flags the sheet's SUM ranges
-- encode: excluded (in the account but not the user's money, e.g. irene 20%)
-- and in_account (counted in the sheet's "AIA display value" figure, e.g. the
-- irene 年金 row is the user's money held in someone else's account).
-- balance %% and the totals are derived on read and never stored.
CREATE TABLE aia_policies (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    label            TEXT    NOT NULL,
    policy_no        TEXT,
    next_pay_date    TEXT,          -- next premium due date
    premium_usd      REAL    NOT NULL DEFAULT 0,
    value_usd        REAL    NOT NULL DEFAULT 0,
    value_updated_at TEXT,
    remaining_years  REAL,
    withdrew_usd     REAL    NOT NULL DEFAULT 0,
    note             TEXT,
    link             TEXT,
    excluded         INTEGER NOT NULL DEFAULT 0,
    in_account       INTEGER NOT NULL DEFAULT 1,
    sort_order       INTEGER NOT NULL,
    created_at       TEXT    NOT NULL,
    updated_at       TEXT    NOT NULL
);

-- Premium payments and withdrawals. Each row logs one recorded event; the
-- prev_* columns snapshot the policy fields the event touched so deleting the
-- event restores them exactly (undo for a misrecorded entry).
CREATE TABLE aia_events (
    id                   INTEGER PRIMARY KEY AUTOINCREMENT,
    policy_id            INTEGER NOT NULL REFERENCES aia_policies (id),
    kind                 TEXT    NOT NULL,      -- 'payment' | 'withdrawal'
    event_date           TEXT    NOT NULL,
    amount_usd           REAL    NOT NULL,
    note                 TEXT,
    prev_next_pay_date   TEXT,
    prev_remaining_years REAL,
    created_at           TEXT    NOT NULL
);

CREATE INDEX idx_aia_events_policy_date ON aia_events (policy_id, event_date);
