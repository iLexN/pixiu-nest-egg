-- Deposit 收訖 lifecycle: a deposit stays unreceived until the user confirms
-- the money came back, and its interest only counts in Month Stat 利息 once
-- received. `credited_*` records the optional auto bank-in to a cash
-- manual_asset so `unreceive` can reverse it exactly.
--
-- Every deposit whose end date has already passed was treated as received by
-- the old date-based interest rule, so it is backfilled as received on its
-- end date. Future deposits stay unreceived until the user clicks 收訖.

ALTER TABLE deposits ADD COLUMN received_at TEXT;
ALTER TABLE deposits ADD COLUMN credited_asset_id INTEGER;
ALTER TABLE deposits ADD COLUMN credited_amount REAL;

UPDATE deposits SET received_at = end_date WHERE end_date <= date('now', 'localtime');
