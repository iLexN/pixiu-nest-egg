-- Forecast items (預測): the manual plan cells of the Overview sheet's
-- A20:H36 forecast grid — one row per planned cash line. `kind` picks the
-- sheet row: hs_deposit / sc_deposit are planned lockups (negative amounts;
-- they schedule a derived deposit_return at `return_month`, else +3 / +4
-- months), `bill` overrides the quarterly 差餉 line, and interest / tax /
-- stock / other are plain signed lines. Everything else in the grid —
-- start, salary, spend, deposit finish, returns, the locked chain — is
-- derived on read and never stored.
CREATE TABLE forecast_items (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    month        TEXT    NOT NULL,          -- YYYY-MM-01
    kind         TEXT    NOT NULL,
    amount       REAL    NOT NULL,
    return_month TEXT,                      -- YYYY-MM-01, deposit kinds only
    note         TEXT,
    sort_order   INTEGER NOT NULL,
    created_at   TEXT    NOT NULL,
    updated_at   TEXT    NOT NULL
);

CREATE INDEX idx_forecast_items_month ON forecast_items (month);
