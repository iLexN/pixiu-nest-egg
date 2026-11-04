-- A placement records the cash account it debited the principal from, so a
-- delete can add back exactly what was stored. Pre-existing rows stay NULL
-- (no backfill), and a deleted account clears its column via
-- ON DELETE SET NULL (store::cash_accounts::delete clears it explicitly too,
-- matching how credited_account_id is handled).
ALTER TABLE deposits ADD COLUMN debited_account_id INTEGER
    REFERENCES cash_accounts(id) ON DELETE SET NULL;
ALTER TABLE deposits ADD COLUMN debited_amount REAL;
