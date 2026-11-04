-- Cash-moving transfers record the two accounts they moved money between so a
-- delete reverses exactly what was recorded; pre-existing rows stay NULL
-- (log-only), and a deleted account clears its column via ON DELETE SET NULL.
ALTER TABLE ibkr_transfers
    ADD COLUMN from_account_id INTEGER REFERENCES cash_accounts(id) ON DELETE SET NULL;
ALTER TABLE ibkr_transfers
    ADD COLUMN broker_account_id INTEGER REFERENCES cash_accounts(id) ON DELETE SET NULL;
