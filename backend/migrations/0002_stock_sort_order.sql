ALTER TABLE stocks ADD COLUMN sort_order INTEGER NOT NULL DEFAULT 0;

WITH ordered AS (
    SELECT id, ROW_NUMBER() OVER (PARTITION BY market ORDER BY id) AS position
    FROM stocks
)
UPDATE stocks
SET sort_order = (
    SELECT position FROM ordered WHERE ordered.id = stocks.id
);

CREATE INDEX idx_stocks_market_order ON stocks (market, sort_order, code);
