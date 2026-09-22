-- Itemize 利息: the N column's hand-kept totals become a derived figure —
-- Σ interest of deposits ending in the month + received coupons + received
-- HK dividends + `interest` items — so the part no app event explains (bank
-- 活期 interest, promo rebates) migrates into a labeled item per month and
-- the scalar column is dropped. SQLite cannot extend a CHECK constraint,
-- so month_items is rebuilt (same pattern as 0016).
--
-- A residual item is written only where the sheet cell was nonzero AND
-- differs from the auto events: months whose cell was blank (e.g. a future
-- month with a deposit scheduled to end) get no item, so their upcoming
-- interest simply shows when it arrives. Residuals under half a cent are
-- float noise on otherwise exact totals and are skipped too.

CREATE TABLE month_items_new (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    month               TEXT    NOT NULL REFERENCES month_stats (month) ON DELETE CASCADE,
    category            TEXT    NOT NULL CHECK (category IN ('adjustment','extra_spend','income','entertainment','interest')),
    label               TEXT,
    amount              REAL    NOT NULL,
    exclude_from_living INTEGER NOT NULL DEFAULT 0,
    auto_key            TEXT,
    note                TEXT,
    created_at          TEXT    NOT NULL
);

INSERT INTO month_items_new (id, month, category, label, amount, exclude_from_living, auto_key, note, created_at)
    SELECT id, month, category, label, amount, exclude_from_living, auto_key, note, created_at FROM month_items;

INSERT INTO month_items_new (month, category, label, amount, note, created_at)
    SELECT month, 'interest', '其他利息', residual, '匯入差額',
           strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
      FROM (
        SELECT ms.month,
               ms.interest - (
                   COALESCE((SELECT SUM(COALESCE(d.interest, 0)) FROM deposits d
                              WHERE d.end_date >= ms.month
                                AND d.end_date < date(ms.month, '+1 month')), 0)
                 + COALESCE((SELECT SUM(c.received_amount) FROM bond_coupons c
                              WHERE c.received_amount IS NOT NULL
                                AND c.pay_date >= ms.month
                                AND c.pay_date < date(ms.month, '+1 month')), 0)
                 + COALESCE((SELECT SUM(dv.received_amount) FROM dividends dv
                              JOIN stocks s ON s.id = dv.stock_id
                              WHERE s.market = 'HK'
                                AND dv.received_amount IS NOT NULL
                                AND dv.pay_date >= ms.month
                                AND dv.pay_date < date(ms.month, '+1 month')), 0)
               ) AS residual
          FROM month_stats ms
         WHERE ms.interest <> 0
      )
     WHERE ABS(residual) >= 0.005;

DROP TABLE month_items;
ALTER TABLE month_items_new RENAME TO month_items;
CREATE UNIQUE INDEX idx_month_items_auto
    ON month_items (month, auto_key) WHERE auto_key IS NOT NULL;

ALTER TABLE month_stats DROP COLUMN interest;
