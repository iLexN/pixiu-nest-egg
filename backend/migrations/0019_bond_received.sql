-- Bond 收訖 lifecycle for the principal return: a matured bond awaits the
-- user's confirmation that the principal came back, and 收訖 can credit a
-- cash manual_asset plus record the month's bond-end adjustment item.
-- Coupons keep their own received_amount lifecycle — this is only about the
-- bond's principal.
--
-- Every bond already past maturity was effectively received, so it is
-- backfilled received_at = maturity_date.

ALTER TABLE bonds ADD COLUMN received_at TEXT;
ALTER TABLE bonds ADD COLUMN credited_asset_id INTEGER;
ALTER TABLE bonds ADD COLUMN credited_amount REAL;

UPDATE bonds SET received_at = maturity_date WHERE maturity_date <= date('now', 'localtime');
