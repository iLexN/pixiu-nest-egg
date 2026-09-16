## MODIFIED Requirements

### Requirement: Browsing trade history
The system SHALL list stored trades for a chosen market, allowing filtering by stock and by date range, and SHALL present them ordered by 日期 with the derived fee, total, per-trade 平均單價, and the stock's stored 現價 for each row. For BUY rows the 現價 cell SHALL be shown in the positive (green) style when 現價 is higher than that row's 平均單價（含 fee）and in the negative (red) style when lower; the styles SHALL match those used for 未實現金額 in 持倉總覽.

#### Scenario: Filter by stock
- **WHEN** the user filters HK trades by stock `中移動`
- **THEN** only trades of `中移動` are listed, newest or oldest first according to the chosen sort

#### Scenario: Filter by date range
- **WHEN** the user filters trades to 日期 between 2025-01-01 and 2025-12-31
- **THEN** only trades dated inside that inclusive range are listed

#### Scenario: BUY row below 現價
- **WHEN** a BUY trade's stock has 現價 170 and the trade's 平均單價（含 fee）is 165
- **THEN** the row's 現價 cell shows 170 in the positive (green) style

#### Scenario: BUY row above 現價
- **WHEN** a BUY trade's stock has 現價 160 and the trade's 平均單價（含 fee）is 165
- **THEN** the row's 現價 cell shows 160 in the negative (red) style

#### Scenario: SELL row is not colored
- **WHEN** a SELL trade's stock has a stored 現價
- **THEN** the row shows the 現價 value without the positive or negative style

#### Scenario: Missing data is not colored
- **WHEN** a trade's stock has no 現價, or the trade's 平均單價 is empty (zero-share row), or 現價 equals 平均單價
- **THEN** the 現價 cell is shown without the positive or negative style (empty when no 現價 is stored)
