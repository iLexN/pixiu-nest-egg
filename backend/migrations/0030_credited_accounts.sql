-- credited_asset_id becomes credited_account_id now that receipt targets are
-- cash_accounts rows. The deposits/bonds columns rename in place (the values
-- already point at the ids the cash rows kept). Coupons and dividends gain
-- the column with a real FK; rows that recorded a credited_amount backfill to
-- the account today's reversal code would hit: HS for coupons and HK
-- dividends, the IBKR USD broker account for US dividends.
ALTER TABLE deposits RENAME COLUMN credited_asset_id TO credited_account_id;
ALTER TABLE bonds RENAME COLUMN credited_asset_id TO credited_account_id;

ALTER TABLE bond_coupons ADD COLUMN credited_account_id INTEGER
    REFERENCES cash_accounts(id) ON DELETE SET NULL;
ALTER TABLE dividends ADD COLUMN credited_account_id INTEGER
    REFERENCES cash_accounts(id) ON DELETE SET NULL;

UPDATE bond_coupons
SET credited_account_id = (SELECT id FROM cash_accounts WHERE label = 'HS' AND role = 'bank')
WHERE credited_amount IS NOT NULL;

UPDATE dividends
SET credited_account_id = (
    SELECT CASE stocks.market
        WHEN 'HK' THEN (SELECT id FROM cash_accounts WHERE label = 'HS' AND role = 'bank')
        ELSE (SELECT id FROM cash_accounts WHERE role = 'broker' AND currency = 'USD')
    END
    FROM stocks WHERE stocks.id = dividends.stock_id
)
WHERE credited_amount IS NOT NULL;
