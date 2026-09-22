-- Month Stat: one row per month of the balance-difference ledger. Only what
-- the user types is stored: the 活期 balance at payday (start_cash, already
-- including that month's salary), the era's salary snapshot, the interest /
-- entertainment / pool figures, and optional frozen 總數/流動資產 totals
-- (NULL = derived live from the portfolio). 月尾, 月支出, 生活支出, 存 and the
-- Changed columns are all derived on read and never stored.
CREATE TABLE month_stats (
    month          TEXT PRIMARY KEY,          -- 'YYYY-MM-01'
    start_cash     REAL,                      -- F 月初(出糧後)
    salary         REAL,                      -- salary in effect that month
    total_assets   REAL,                      -- B 總數; NULL = live
    liquid_assets  REAL,                      -- D 流動資產; NULL = live
    interest       REAL NOT NULL DEFAULT 0,   -- N 利息
    entertainment  REAL NOT NULL DEFAULT 0,   -- O 娛樂支出
    pool_input     REAL NOT NULL DEFAULT 0,   -- P Irene + 開心 Pool
    note           TEXT,
    created_at     TEXT NOT NULL,
    updated_at     TEXT NOT NULL
);

-- Labeled line items per month, replacing the sheet's opaque G/J/L sum
-- formulas: adjustment (bank in/out that must not count as 支出),
-- extra_spend (real spending excluded from 生活支出), income (non-salary
-- income added to 存). auto_key links an item to the app event it came from;
-- the partial unique index prevents accepting a suggestion twice.
CREATE TABLE month_items (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    month      TEXT    NOT NULL REFERENCES month_stats (month) ON DELETE CASCADE,
    category   TEXT    NOT NULL CHECK (category IN ('adjustment','extra_spend','income')),
    label      TEXT,
    amount     REAL    NOT NULL,
    auto_key   TEXT,                          -- NULL = manual or imported
    note       TEXT,                          -- imported formula text / origin
    created_at TEXT    NOT NULL
);
CREATE UNIQUE INDEX idx_month_items_auto
    ON month_items (month, auto_key) WHERE auto_key IS NOT NULL;

-- Tombstones for dismissed auto-suggestions so they never reappear.
CREATE TABLE month_item_dismissals (
    month      TEXT NOT NULL,
    auto_key   TEXT NOT NULL,
    created_at TEXT NOT NULL,
    UNIQUE (month, auto_key)
);

-- The Overview cells that feed the live monthly totals: cash rows are the
-- 活期 behind start_cash (B16 HS, B17 渣打), asset rows feed only 總數
-- (B7 Irene, B8 HS人壽).
CREATE TABLE manual_assets (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    label      TEXT    NOT NULL,
    kind       TEXT    NOT NULL CHECK (kind IN ('cash','asset')),
    amount     REAL    NOT NULL DEFAULT 0,
    sort_order INTEGER NOT NULL,
    updated_at TEXT    NOT NULL
);

-- When the principal left the bank account; feeds dep-start suggestions.
ALTER TABLE deposits ADD COLUMN start_date TEXT;
