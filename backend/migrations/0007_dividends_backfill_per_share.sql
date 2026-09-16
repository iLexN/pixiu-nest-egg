-- Backfill 每股 for dividends recorded before per_share was derived from the
-- amount. Received amount wins over the estimate, matching dividend_amount.
UPDATE dividends
SET per_share = COALESCE(received_amount, estimated_amount) / shares_held
WHERE per_share IS NULL
  AND shares_held > 0
  AND COALESCE(received_amount, estimated_amount) IS NOT NULL;
