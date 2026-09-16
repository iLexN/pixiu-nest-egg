-- The pay date is the receipt day in practice; the separate received_date
-- column carried no extra information.
ALTER TABLE dividends DROP COLUMN received_date;
