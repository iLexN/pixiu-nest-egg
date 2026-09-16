-- Split the bank out of the deposit label: labels like "SC-9632" encode the
-- bank as a prefix (SC = 渣打, HS = 恒生), which is fragile to parse.
-- `bank` stores the short code so rollups group by a real column.
ALTER TABLE deposits ADD COLUMN bank TEXT;

UPDATE deposits
   SET bank = TRIM(substr(label, 1, instr(label, '-') - 1))
 WHERE instr(label, '-') > 1;

CREATE INDEX idx_deposits_bank ON deposits (bank);
