# Stock Trades Specification

## Purpose

Records the history of Hong Kong and US stock trades with the market-specific inputs the owner actually has at hand, deriving the missing money figures so each trade's unit price including fees is known without manual spreadsheet formulas.

## Requirements

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

### Requirement: Date entry in YYYY-MM-DD
Every 日期 entry field in the trade UI — the 新增交易 form, inline trade editing, and the trade-history 由/至 date-range filters — SHALL present and accept dates in `YYYY-MM-DD` form regardless of the OS or browser locale, and SHALL offer a calendar picker for choosing a date. An entry that is not in `YYYY-MM-DD` form SHALL be flagged or rejected before submission; the existing server-side validation remains the final check.

#### Scenario: Entry field shows ISO format
- **WHEN** the user opens the 新增交易 form or edits a trade dated 2026-09-16
- **THEN** the 日期 field displays `2026-09-16` in year-month-day order, not the OS locale order

#### Scenario: Picking a date from the calendar
- **WHEN** the user chooses 2026-09-16 from the calendar picker
- **THEN** the field shows `2026-09-16` and that value is submitted

#### Scenario: Typing a non-ISO date
- **WHEN** the user types `16/09/2026` into a 日期 field and submits
- **THEN** the input is flagged or rejected before or at submission and no trade is stored with a misread date

#### Scenario: Partial date-range filter
- **WHEN** the user has typed an incomplete value such as `2026-0` into a 由/至 filter
- **THEN** the filter is not applied until it holds a complete `YYYY-MM-DD` date

### Requirement: Hiding a stock
The system SHALL let the user mark a stock as hidden (inactive) and unhide it from the stock registry, regardless of whether it has trades or 派息 records. A hidden stock SHALL remain in the registry list — visually marked — so it can be unhidden, SHALL keep all of its trades, dividends and prices, and SHALL remain selectable when recording trades or dividends. Hiding SHALL NOT bypass the rule that a stock with trades or dividend records cannot be deleted.

#### Scenario: Hiding a fully-sold stock
- **WHEN** the user hides `香港寬頻`, which has trades and 派息 records but zero holdings
- **THEN** the stock is marked inactive and disappears from 持倉總覽 while staying in the registry, visually marked as hidden

#### Scenario: Unhiding a stock
- **WHEN** the user unhides a previously hidden stock
- **THEN** it becomes active again and reappears in 持倉總覽 in its saved order position

#### Scenario: Hidden stock keeps its history
- **WHEN** a stock is hidden
- **THEN** its trades, dividends and prices are unchanged, and it can still be selected in the trade and dividend forms
