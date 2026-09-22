-- Itemize 娛樂支出: the O column's hand-written sums (e.g. `=500 + 4700 + 75`)
-- become line items, and an `exclude_from_living` flag lets one entry also
-- subtract from 生活支出 — the sheet typed such amounts twice (inside the O
-- sum AND the `=I − …` J formula). SQLite cannot extend a CHECK constraint,
-- so month_items is rebuilt; each nonzero month_stats.entertainment migrates
-- into a single entertainment item and the scalar column is dropped.

CREATE TABLE month_items_new (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    month               TEXT    NOT NULL REFERENCES month_stats (month) ON DELETE CASCADE,
    category            TEXT    NOT NULL CHECK (category IN ('adjustment','extra_spend','income','entertainment')),
    label               TEXT,
    amount              REAL    NOT NULL,
    exclude_from_living INTEGER NOT NULL DEFAULT 0,
    auto_key            TEXT,
    note                TEXT,
    created_at          TEXT    NOT NULL
);

INSERT INTO month_items_new (id, month, category, label, amount, auto_key, note, created_at)
    SELECT id, month, category, label, amount, auto_key, note, created_at FROM month_items;

INSERT INTO month_items_new (month, category, label, amount, note, created_at)
    SELECT month, 'entertainment', '娛樂支出', entertainment, NULL,
           strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
      FROM month_stats WHERE entertainment <> 0;

DROP TABLE month_items;
ALTER TABLE month_items_new RENAME TO month_items;
CREATE UNIQUE INDEX idx_month_items_auto
    ON month_items (month, auto_key) WHERE auto_key IS NOT NULL;

ALTER TABLE month_stats DROP COLUMN entertainment;
