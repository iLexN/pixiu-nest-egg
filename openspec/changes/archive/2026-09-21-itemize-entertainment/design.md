# Design

## Context

`add-month-stat` kept `interest`/`entertainment`/`pool_input` as scalar columns because the sheet tracks each as one cell. In practice O is a hand-written sum whose terms sometimes also appear in the J formula's exclusion list — the same figure typed twice, unlinked. Itemizing 娛樂支出 removes that double entry.

## Schema (migration `0016_itemize_entertainment.sql`)

SQLite cannot extend a CHECK constraint, so `month_items` is rebuilt:

```sql
CREATE TABLE month_items_new (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    month      TEXT    NOT NULL REFERENCES month_stats (month) ON DELETE CASCADE,
    category   TEXT    NOT NULL CHECK (category IN ('adjustment','extra_spend','income','entertainment')),
    label      TEXT,
    amount     REAL    NOT NULL,
    exclude_from_living INTEGER NOT NULL DEFAULT 0,
    auto_key   TEXT,
    note       TEXT,
    created_at TEXT    NOT NULL
);
INSERT INTO month_items_new (id, month, category, label, amount, auto_key, note, created_at)
    SELECT id, month, category, label, amount, auto_key, note, created_at FROM month_items;
-- Migrate the scalar into a single item per month (parity-neutral).
INSERT INTO month_items_new (month, category, label, amount, note, created_at)
    SELECT month, 'entertainment', '娛樂支出', entertainment, NULL, <now>
      FROM month_stats WHERE entertainment <> 0;
DROP TABLE month_items;
ALTER TABLE month_items_new RENAME TO month_items;
CREATE UNIQUE INDEX idx_month_items_auto
    ON month_items (month, auto_key) WHERE auto_key IS NOT NULL;
ALTER TABLE month_stats DROP COLUMN entertainment;
```

- `ALTER TABLE … DROP COLUMN` needs SQLite ≥ 3.35 — sqlx's bundled SQLite is far newer; the existing migrations already rely on modern SQLite.
- FK cascade and the partial unique index are preserved verbatim.
- The migration runs inside the migration transaction; `PRAGMA foreign_keys` is off during `sqlx::migrate!` so the swap is safe.

## Derivation changes (`calc.rs`)

- `MonthItemSums` gains `entertainment` and `entertainment_excluded` (Σ of entertainment items with `exclude_from_living`).
- `MonthStatRow` drops `entertainment`; the derived `entertainment` figure = `sums.entertainment`, and `living_spend = month_spend − sums.extra_spend − sums.entertainment_excluded`.
- `pool_balances`/`month_year_summaries` already receive the items map — they read `sums.entertainment` instead of the column.
- The response keeps an `entertainment` figure (renamed `entertainment_sum` for consistency with `adjustment_sum`/`extra_spend_sum`/`income_sum`).

## API

- `MonthItemCategory::Entertainment` (`"entertainment"`); `MonthItem`/`NewMonthItem`/`MonthItemPatch` gain `exclude_from_living` — `true` rejected on non-entertainment categories (field error on `exclude_from_living`); changing category away from entertainment clears it.
- `MonthStatPatch.entertainment` is removed; `MonthStat.entertainment` becomes the derived `entertainment_sum`.

## Import

- O cells materialize exactly like G/J/L already do: one `entertainment` item with the cached total and the formula text as `note`. `exclude_from_living` stays 0 — history keeps its existing `extra_spend` items rather than attempting to parse and re-link the O↔J overlap (the user may split a lump manually if they care).
- The scalar-column migration inside `0016` covers rows already in the DB; a fresh import produces items directly. Both paths converge on "one item per month's O value".

## Parity

- Compare sheet O against the derived `entertainment` sum per month (same tolerance/informational rules); no other comparisons change.

## Frontend

- Item editor: category dropdown gains 娛樂; a 不計入生活支出 checkbox appears for entertainment items only; the stored flag shows on entertainment rows (e.g. a `−生活` marker).
- Month editor: the 娛樂支出 `<input>` becomes a read-only display of the derived sum (placed with the totals hint), keeping the sheet's column visible without a second source of truth.
- Month table's 娛樂支出 column reads `entertainment_sum` (rename only).

## Risks / edge cases

- **Double counting during transition**: after migration a month could hold both a scalar-migrated entertainment item AND an extra_spend item for the same purchase (2026-08's 4700) — identical to today's state, so no regression; the flag only helps new itemized entries.
- **Quick total entry**: users who just want the monthly total add one item — same effort as the old field.
- **`pool_input`/`interest` stay scalar** — deliberately unchanged; only 娛樂支出 needed itemization.
