-- Family deposits (家人 定期): one row per time deposit held on behalf of a
-- family member — the workbook's Mum / Dad sheets. `holder` names the family
-- member (媽媽, 爸爸, Irene); `note` carries the bank's stepped-rate schedule
-- as free text (there is no rate column). This is a record-only registry:
-- the money is outside the user's own totals, so nothing computing 定期!B1,
-- Overview, Month Stat or 回顧 figures reads this table, and 收訖 never banks
-- into manual_assets nor writes month items. total (= principal + interest),
-- status (End once 收訖 — never auto-set from end_date), and the end
-- month/year are all derived on read and never stored.
CREATE TABLE family_deposits (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    holder      TEXT    NOT NULL,
    label       TEXT,
    bank        TEXT,
    principal   REAL,
    interest    REAL,
    start_date  TEXT,
    end_date    TEXT    NOT NULL,
    received_at TEXT,
    note        TEXT,
    sort_order  INTEGER NOT NULL,
    created_at  TEXT    NOT NULL,
    updated_at  TEXT    NOT NULL
);

CREATE INDEX idx_family_deposits_holder_end ON family_deposits (holder, end_date);
