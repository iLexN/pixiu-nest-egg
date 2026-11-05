-- A US trade records the signed amount it applied to the USD broker
-- cash_accounts row (BUY -total, SELL +total), so an edit or delete can
-- reverse exactly what moved. NULL means the trade moved no cash: HK
-- trades, zero-share price-adjustment rows, and every row written before
-- settlement existed.
ALTER TABLE trades ADD COLUMN settled_amount REAL;
