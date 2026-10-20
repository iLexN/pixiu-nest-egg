-- fx_rates holds the current HKD conversion rate per foreign currency,
-- replacing the app_meta key aia.usd_hkd_rate. A CHECK on currency keeps the
-- table to supported codes; rate_to_hkd > 0 keeps a stored rate usable.
CREATE TABLE fx_rates (
    currency    TEXT PRIMARY KEY CHECK (currency = 'USD'),
    rate_to_hkd REAL NOT NULL CHECK (rate_to_hkd > 0),
    updated_at  TEXT NOT NULL
);

-- Seed from the stored AIA rate only when it parses as a positive number.
INSERT INTO fx_rates (currency, rate_to_hkd, updated_at)
SELECT 'USD', CAST(value AS REAL), strftime('%Y-%m-%dT%H:%M:%fZ','now')
FROM app_meta
WHERE key = 'aia.usd_hkd_rate' AND CAST(value AS REAL) > 0;

DELETE FROM app_meta WHERE key = 'aia.usd_hkd_rate';
