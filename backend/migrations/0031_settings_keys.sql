-- Settings keys move to domain names owned by store::settings: salary belongs
-- to the month ledger, the semi-liquid target to the 策略 block, the goal
-- figures to the goals block, and the IBKR mark-to-market is a reported value.
UPDATE app_meta SET key = 'ledger.salary'                WHERE key = 'overview.salary';
UPDATE app_meta SET key = 'strategy.semi_liquid_target'  WHERE key = 'overview.semi_liquid_target';
UPDATE app_meta SET key = 'goals.goal1_amount'           WHERE key = 'overview.goal1_amount';
UPDATE app_meta SET key = 'goals.goal2_amount'           WHERE key = 'overview.goal2_amount';
UPDATE app_meta SET key = 'goals.goal3_inflation'        WHERE key = 'overview.goal3_inflation';
UPDATE app_meta SET key = 'goals.goal3_years'            WHERE key = 'overview.goal3_years';
UPDATE app_meta SET key = 'goals.final_replacement'      WHERE key = 'overview.final_replacement';
UPDATE app_meta SET key = 'ibkr.reported_value'          WHERE key = 'ibkr.now_value';

-- The 開心Pool yearly rate becomes a table of its own so a stored rate does
-- not have to fabricate a year_review row.
CREATE TABLE pool_rates (
    year INTEGER PRIMARY KEY,
    rate REAL NOT NULL CHECK (rate >= 0 AND rate < 1)
);

INSERT INTO pool_rates (year, rate)
SELECT CAST(substr(key, 20) AS INTEGER), CAST(value AS REAL)
FROM app_meta
WHERE key LIKE 'overview.pool_rate.%';

DELETE FROM app_meta WHERE key LIKE 'overview.pool_rate.%';
