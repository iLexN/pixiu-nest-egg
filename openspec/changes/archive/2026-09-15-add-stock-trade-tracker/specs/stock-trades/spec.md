## Purpose

Records the history of Hong Kong and US stock trades with the market-specific inputs the owner actually has at hand, deriving the missing money figures so each trade's unit price including fees is known without manual spreadsheet formulas.

## ADDED Requirements

### Requirement: Stock registry per market
The system SHALL maintain a list of stocks, each belonging to exactly one market (`HK` or `US`), identified within that market by a unique code (`股票代碼`: the Chinese name for HK stocks, the ticker for US stocks). A stock MAY additionally carry a ticker, an exchange label (e.g. `HKG`, `NYSE`, `NASDAQ`, `NYSEARCA`), a sector, and a free-text note.

#### Scenario: Creating a stock
- **WHEN** the user adds a stock with market `HK` and code `香港電訊`
- **THEN** the stock is stored and becomes selectable when recording HK trades

#### Scenario: Duplicate code in the same market
- **WHEN** the user adds a stock whose market and code match an existing stock
- **THEN** the system rejects the request with an error identifying the conflict, and no second stock is created

#### Scenario: Same code in different markets
- **WHEN** stocks with the same code exist for market `HK` and market `US`
- **THEN** both are accepted and kept separate

#### Scenario: Stock with no trades
- **WHEN** a stock has no trades recorded
- **THEN** it still appears in the stock list and in the summary with zero holdings and zero cost

### Requirement: Recording a Hong Kong trade
The system SHALL accept a HK trade described by 股票代碼, 股數, buy total (fee included), 單價, 日期, and 類別 (`BUY` or `SELL`), and SHALL derive the fee as `buy total − 股數 × 單價`.

#### Scenario: HK buy with fee included in total
- **WHEN** the user records a HK `BUY` of 30000 shares of `中國銀行` at 單價 3.2 with buy total 96523.15 on 2023-05-23
- **THEN** the trade is stored with fee 523.15 and total 96523.15

#### Scenario: Buy total below the gross amount
- **WHEN** the user records a HK trade whose buy total is less than 股數 × 單價, implying a negative fee
- **THEN** the system rejects the request unless the user supplies a note explaining the adjustment

### Requirement: Recording a US trade
The system SHALL accept a US trade described by 股票代碼, 股數, 單價, fee, 日期, and 類別 (`BUY` or `SELL`), and SHALL derive the buy total as `股數 × 單價 + fee`.

#### Scenario: US buy with explicit fee
- **WHEN** the user records a US `BUY` of 3 shares of `VOO` at 單價 696.04 with fee 1.000009 on 2026-06-02
- **THEN** the trade is stored with total 2089.120009

#### Scenario: Fractional shares
- **WHEN** the user records a US trade with a fractional share quantity such as 0.5
- **THEN** the trade is accepted and the total is derived from the fractional quantity

### Requirement: Trade field validation
The system SHALL validate trade input before storing it: 日期 MUST be a calendar date in `YYYY-MM-DD` form, 類別 MUST be `BUY` or `SELL`, 股數 and 單價 MUST NOT be negative, and the referenced stock MUST exist in the market the trade is recorded for.

#### Scenario: Malformed date
- **WHEN** the user submits a trade with 日期 `2026/13/45`
- **THEN** the system rejects it with a validation error naming the date field, and stores nothing

#### Scenario: Unknown stock
- **WHEN** the user submits a trade referencing a stock that is not registered
- **THEN** the system rejects it with a validation error, and stores nothing

#### Scenario: Adjustment row with zero shares
- **WHEN** the user records a trade with 股數 0 and a non-zero total together with a note
- **THEN** the trade is accepted so brokerage cash adjustments can be captured

### Requirement: Per-trade unit price including fee
For every stored trade the system SHALL report 平均單價 for that trade, computed as `total ÷ 股數`, so the entered 單價 and the true cost per share are both visible.

#### Scenario: HK trade unit price including fee
- **WHEN** a HK trade of 30000 shares with total 96523.15 is displayed
- **THEN** its 平均單價 is reported as 3.217438 (rounded for display, 單價 3.2 shown alongside)

#### Scenario: Zero-share trade
- **WHEN** a trade with 股數 0 is displayed
- **THEN** 平均單價 is reported as empty rather than as an error or infinity

### Requirement: Browsing trade history
The system SHALL list stored trades for a chosen market, allowing filtering by stock and by date range, and SHALL present them ordered by 日期 with the derived fee, total, and per-trade 平均單價 for each row.

#### Scenario: Filter by stock
- **WHEN** the user filters HK trades by stock `中移動`
- **THEN** only trades of `中移動` are listed, newest or oldest first according to the chosen sort

#### Scenario: Filter by date range
- **WHEN** the user filters trades to 日期 between 2025-01-01 and 2025-12-31
- **THEN** only trades dated inside that inclusive range are listed

### Requirement: Editing and deleting trades
The system SHALL allow any stored trade to be edited or deleted, re-deriving fee, total, and 平均單價 from the edited input using the rules of the trade's market.

#### Scenario: Correcting a buy total
- **WHEN** the user edits a HK trade's buy total
- **THEN** the stored fee and the reported 平均單價 are recomputed from the new total

#### Scenario: Deleting a trade
- **WHEN** the user deletes a trade
- **THEN** the trade disappears from the history and every figure derived from it is recomputed

#### Scenario: Deleting a stock that has trades
- **WHEN** the user tries to delete a stock that still has trades
- **THEN** the system refuses and explains that its trades must be removed first
