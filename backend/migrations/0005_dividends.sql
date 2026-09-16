-- Stock dividends (派息): one row per dividend event per stock.
-- shares_held/buy_cost/received_price are point-in-time snapshots stored at
-- write time so later trades or price moves never rewrite a recorded yield.
-- status (pending until received_amount is set), the effective amount, and
-- both yields are derived on read and never stored.
CREATE TABLE dividends (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    stock_id         INTEGER NOT NULL REFERENCES stocks (id),
    pay_date         TEXT    NOT NULL,
    per_share        REAL,
    shares_held      REAL,
    buy_cost         REAL,
    estimated_amount REAL,
    received_amount  REAL,
    received_date    TEXT,
    received_price   REAL,
    note             TEXT,
    created_at       TEXT    NOT NULL,
    updated_at       TEXT    NOT NULL
);

CREATE INDEX idx_dividends_stock_date ON dividends (stock_id, pay_date);
