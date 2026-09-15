-- Stock registry: one row per stock per market.
CREATE TABLE stocks (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    market           TEXT    NOT NULL CHECK (market IN ('HK', 'US')),
    code             TEXT    NOT NULL,
    ticker           TEXT,
    exchange         TEXT,
    sector           TEXT,
    manual_price     REAL,
    price_updated_at TEXT,
    pe               REAL,
    eps              REAL,
    high52           REAL,
    low52            REAL,
    note             TEXT,
    is_active        INTEGER NOT NULL DEFAULT 1,
    UNIQUE (market, code)
);

-- Trade history. fee/total are both stored; input_mode records which pair the
-- user typed so an edit re-derives the same way the original entry did.
CREATE TABLE trades (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    stock_id   INTEGER NOT NULL REFERENCES stocks (id),
    trade_type TEXT    NOT NULL CHECK (trade_type IN ('BUY', 'SELL')),
    trade_date TEXT    NOT NULL,
    shares     REAL    NOT NULL,
    unit_price REAL    NOT NULL,
    fee        REAL    NOT NULL,
    total      REAL    NOT NULL,
    input_mode TEXT    NOT NULL CHECK (input_mode IN ('HK_TOTAL', 'US_FEE')),
    note       TEXT,
    created_at TEXT    NOT NULL,
    updated_at TEXT    NOT NULL
);

CREATE INDEX idx_trades_stock_date ON trades (stock_id, trade_date);
