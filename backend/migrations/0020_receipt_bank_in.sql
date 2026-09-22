-- Auto bank-in on coupon/dividend 收訖: when received_amount is set, the
-- amount is credited to the HS cash row (coupon, HK dividend) or the IBKR
-- USD cash meta value (US dividend). credited_amount records what was
-- actually credited so clearing received_amount reverses exactly; NULL means
-- no credit happened (opted out, or the target row is missing).

ALTER TABLE bond_coupons ADD COLUMN credited_amount REAL;
ALTER TABLE dividends ADD COLUMN credited_amount REAL;
