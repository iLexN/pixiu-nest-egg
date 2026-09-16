-- Time deposits (定期): one row per entry of 定期Info's 表_定期List.
-- label/principal/rate/interest are nullable: the source list holds
-- interest-only rows (no id/principal) and label-only rows.
-- total (= principal + interest), status (End once end_date passes), and the
-- end month/year are all derived on read and never stored.
CREATE TABLE deposits (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    label      TEXT,
    principal  REAL,
    rate       REAL,
    interest   REAL,
    end_date   TEXT    NOT NULL,
    note1      TEXT,
    note2      TEXT,
    sort_order INTEGER NOT NULL,
    created_at TEXT    NOT NULL,
    updated_at TEXT    NOT NULL
);

CREATE INDEX idx_deposits_end_date ON deposits (end_date);
