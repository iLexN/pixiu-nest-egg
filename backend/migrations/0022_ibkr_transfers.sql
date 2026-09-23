-- IBKR bank→broker transfers as a dated log. 累計轉入 (美股!B1) is the log's
-- sum and each year's 轉入 feeds the year review's invested on top of the HK
-- net invested and the manual adjustment.
CREATE TABLE ibkr_transfers (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    transfer_date TEXT NOT NULL,
    amount_hkd    REAL NOT NULL,
    created_at    TEXT NOT NULL
);

-- The seeded cumulative cell becomes one entry dated to the first US trade —
-- funding precedes the first buy — falling back to today.
INSERT INTO ibkr_transfers (transfer_date, amount_hkd, created_at)
SELECT COALESCE(
           (SELECT MIN(t.trade_date) FROM trades t
             JOIN stocks s ON s.id = t.stock_id WHERE s.market = 'US'),
           date('now')),
       CAST(a.value AS REAL),
       datetime('now')
FROM app_meta a
WHERE a.key = 'ibkr.transferred_hkd' AND CAST(a.value AS REAL) != 0;

-- The imported invested adjustment folded the transfers in (the sheet's cell
-- was `港股!C ± manual + 美股!B1`); split them back out so the adjustment
-- holds only the manual part.
UPDATE year_review
SET invested_adjustment =
        COALESCE(invested_adjustment, 0)
        - (SELECT SUM(t.amount_hkd) FROM ibkr_transfers t
            WHERE strftime('%Y', t.transfer_date) = CAST(year_review.year AS TEXT))
WHERE EXISTS (
    SELECT 1 FROM ibkr_transfers t
    WHERE strftime('%Y', t.transfer_date) = CAST(year_review.year AS TEXT));

DELETE FROM app_meta WHERE key = 'ibkr.transferred_hkd';
