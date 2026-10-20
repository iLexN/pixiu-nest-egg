-- cash_accounts is the single store for 活期 money: bank rows (HS, 渣打) and
-- the IBKR broker cash in each currency. kind='cash' manual_assets rows are
-- copied over keeping their ids, so the credited_asset_id values already on
-- deposits/bonds keep pointing at the right account when those columns are
-- renamed in 0030. is_default_receipt marks the bank-in target per currency
-- (unique partial index enforces at most one); role='broker' rows are the
-- IBKR cash figures previously stored in app_meta. Bank accounts are HKD-only.
CREATE TABLE cash_accounts (
    id                 INTEGER PRIMARY KEY AUTOINCREMENT,
    label              TEXT    NOT NULL UNIQUE,
    currency           TEXT    NOT NULL CHECK (currency IN ('HKD','USD')),
    role               TEXT    NOT NULL CHECK (role IN ('bank','broker')),
    amount             REAL    NOT NULL DEFAULT 0,
    is_default_receipt INTEGER NOT NULL DEFAULT 0 CHECK (is_default_receipt IN (0,1)),
    sort_order         INTEGER NOT NULL,
    updated_at         TEXT    NOT NULL,
    CHECK (role = 'broker' OR currency = 'HKD')
);
CREATE UNIQUE INDEX idx_cash_accounts_default ON cash_accounts (currency) WHERE is_default_receipt = 1;
CREATE UNIQUE INDEX idx_cash_accounts_broker  ON cash_accounts (currency) WHERE role = 'broker';

INSERT INTO cash_accounts (id, label, currency, role, amount, sort_order, updated_at)
SELECT id, label, 'HKD', 'bank', amount, sort_order, updated_at
FROM manual_assets
WHERE kind = 'cash';

-- HS is the HKD default, matching the hard-coded label = 'HS' lookups the
-- receipt flows use today.
UPDATE cash_accounts SET is_default_receipt = 1 WHERE label = 'HS' AND role = 'bank';

-- IBKR cash, previously ibkr.hkd_cash / ibkr.usd_cash in app_meta. Both rows
-- are always inserted so a fresh database has the broker accounts too.
INSERT INTO cash_accounts (label, currency, role, amount, is_default_receipt, sort_order, updated_at)
SELECT 'IBKR HKD', 'HKD', 'broker',
       COALESCE((SELECT CAST(value AS REAL) FROM app_meta WHERE key = 'ibkr.hkd_cash'), 0),
       0,
       COALESCE((SELECT MAX(sort_order) FROM cash_accounts), 0) + 1,
       strftime('%Y-%m-%dT%H:%M:%fZ','now');

INSERT INTO cash_accounts (label, currency, role, amount, is_default_receipt, sort_order, updated_at)
SELECT 'IBKR USD', 'USD', 'broker',
       COALESCE((SELECT CAST(value AS REAL) FROM app_meta WHERE key = 'ibkr.usd_cash'), 0),
       1,
       COALESCE((SELECT MAX(sort_order) FROM cash_accounts), 0) + 1,
       strftime('%Y-%m-%dT%H:%M:%fZ','now');

DELETE FROM app_meta WHERE key IN ('ibkr.hkd_cash', 'ibkr.usd_cash');

-- manual_assets keeps only the asset rows; SQLite cannot drop the CHECK'ed
-- kind column in place, so rebuild the table. The sequence is restored to the
-- old max id so new asset ids never reuse the cash ids taken by cash_accounts.
CREATE TABLE manual_assets_new (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    label      TEXT    NOT NULL,
    liquidity  TEXT    NOT NULL DEFAULT 'long' CHECK (liquidity IN ('short','long')),
    amount     REAL    NOT NULL DEFAULT 0,
    sort_order INTEGER NOT NULL,
    updated_at TEXT    NOT NULL
);

INSERT INTO manual_assets_new (id, label, liquidity, amount, sort_order, updated_at)
SELECT id, label, liquidity, amount, sort_order, updated_at
FROM manual_assets
WHERE kind = 'asset';

DROP TABLE manual_assets;
ALTER TABLE manual_assets_new RENAME TO manual_assets;

-- The high-water mark is the max id over ALL old rows: the asset ids now in
-- the rebuilt table plus the bank ids now in cash_accounts (the broker ids
-- are fresh and must not count).
UPDATE sqlite_sequence
SET seq = (
    SELECT MAX(v) FROM (
        SELECT MAX(id) AS v FROM manual_assets
        UNION ALL
        SELECT MAX(id) FROM cash_accounts WHERE role = 'bank'
    )
)
WHERE name = 'manual_assets';

-- When the old table had no asset rows the rebuild created no sequence entry;
-- add one so the mark still holds.
INSERT INTO sqlite_sequence (name, seq)
SELECT 'manual_assets', v FROM (
    SELECT MAX(m) AS v FROM (
        SELECT MAX(id) AS m FROM manual_assets
        UNION ALL
        SELECT MAX(id) FROM cash_accounts WHERE role = 'bank'
    )
)
WHERE v IS NOT NULL
  AND NOT EXISTS (SELECT 1 FROM sqlite_sequence WHERE name = 'manual_assets');
