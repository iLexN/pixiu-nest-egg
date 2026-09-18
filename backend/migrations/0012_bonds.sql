-- Bonds (債券): one row per bond in the 債券 sheet's registry table, and one
-- row per scheduled coupon in its 付息日 block. The sheet only ever holds
-- active bonds (ended ones were deleted), so matured history accumulates
-- here going forward. status (active/matured), the coupon's expected amount
-- (per_10k × principal ÷ 10000), its status (待定 → pending → received) and
-- variance are all derived on read and never stored.
CREATE TABLE bonds (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    label         TEXT    NOT NULL,
    issue_no      TEXT,
    principal     REAL    NOT NULL,
    maturity_date TEXT    NOT NULL,
    note          TEXT,
    sort_order    INTEGER NOT NULL,
    created_at    TEXT    NOT NULL,
    updated_at    TEXT    NOT NULL
);

CREATE INDEX idx_bonds_maturity ON bonds (maturity_date);

CREATE TABLE bond_coupons (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    bond_id         INTEGER NOT NULL REFERENCES bonds (id),
    pay_date        TEXT    NOT NULL,
    fixing_date     TEXT,
    -- 年息率 / 每1萬港元債券利息; NULL = 待定 (rate not yet fixed).
    annual_rate     REAL,
    per_10k         REAL,
    received_amount REAL,
    note            TEXT,
    created_at      TEXT    NOT NULL,
    updated_at      TEXT    NOT NULL
);

CREATE INDEX idx_bond_coupons_bond_date ON bond_coupons (bond_id, pay_date);
