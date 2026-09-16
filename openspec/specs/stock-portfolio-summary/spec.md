# Stock Portfolio Summary Specification

## Purpose

Turns the recorded trade history into the per-stock and per-market position view the owner previously maintained by hand on the `港股` and `美股` sheets, using manually entered current prices instead of spreadsheet market-data functions.

## Requirements

### Requirement: Per-stock holdings and cost
For each stock the system SHALL derive, from stored trades only, the shares held as `Σ BUY 股數 − Σ SELL 股數` and the total buy cost as `Σ BUY total` (fees included). These figures SHALL never be stored denormalized; they SHALL be recomputed whenever trades change.

#### Scenario: Holdings after multiple buys
- **WHEN** `中國銀行` has BUY trades of 30000, 18000 and 20000 shares with totals 96523.15, 53731.5 and 58491.99
- **THEN** the summary reports 68000 shares held and 總買入成本 208746.64

#### Scenario: Holdings after a sell
- **WHEN** a stock has 1000 shares bought and a SELL of 400 shares is recorded
- **THEN** the summary reports 600 shares held, and 總買入成本 remains the sum of BUY totals

#### Scenario: Trade edited
- **WHEN** a trade's 股數 is edited
- **THEN** the summary reflects the new holdings without any separate recalculation step by the user

### Requirement: Weighted average buy price
The system SHALL report 加權平均買入單價 for each stock as `總買入成本 ÷ Σ BUY 股數` when shares are held, and 0 when no shares are held. This intentionally divides by shares bought rather than shares held, preserving the behavior of the spreadsheet being replaced, and the system SHALL make that definition visible to the user.

#### Scenario: Average across several buys
- **WHEN** `中國銀行` has 68000 shares bought for a total of 208746.64
- **THEN** 加權平均買入單價 is reported as 3.069804 (rounded for display)

#### Scenario: Position fully closed
- **WHEN** all shares of a stock have been sold so holdings are 0
- **THEN** 加權平均買入單價 is reported as 0

#### Scenario: Definition disclosed in the UI
- **WHEN** the user views the summary
- **THEN** the 加權平均買入單價 column states that it divides total buy cost by total shares bought, not by shares held

### Requirement: Manually entered market data
The system SHALL let the user store a 現價 per stock, together with an optional PE, EPS, high52 and low52, and SHALL record when 現價 was last updated. The system SHALL NOT call any external market-data service.

#### Scenario: Updating a current price
- **WHEN** the user sets 現價 for `匯豐` to 166.1
- **THEN** the value is stored, the last-updated timestamp is set, and the summary uses it immediately

#### Scenario: No price entered
- **WHEN** a stock has no 現價
- **THEN** its 當前總市值, 未實現金額 and 未實現報酬率 are reported as empty rather than zero, and market totals state that they exclude that stock

### Requirement: Unrealized position value
For each stock holding shares the system SHALL report 當前總市值 as `現價 × shares held`, 未實現金額 as `當前總市值 − 總買入成本`, and 未實現報酬率 as `未實現金額 ÷ 總買入成本`. When holdings are 0 or 總買入成本 is 0, the derived figures SHALL be reported as empty.

#### Scenario: Profitable holding
- **WHEN** a stock holds 68000 shares at 現價 5.91 with 總買入成本 208746.64
- **THEN** 當前總市值 is 401880, 未實現金額 is 193133.36, and 未實現報酬率 is 0.925205 (rounded for display)

#### Scenario: Zero cost basis
- **WHEN** a stock's 總買入成本 is 0
- **THEN** 未實現報酬率 is reported as empty instead of a division error

### Requirement: Market totals and sector rollup
For each market the system SHALL report totals across stocks — total buy cost, total current market value, net unrealized amount, and net as a percentage of total buy cost — and SHALL group stocks by sector, reporting each sector's buy cost, market value, share of total market value, and percentage change.

#### Scenario: Market totals
- **WHEN** the HK summary is viewed
- **THEN** the total buy cost, total market value, net amount and net percentage across all HK stocks are shown

#### Scenario: Sector grouping
- **WHEN** several stocks share the sector `Utilities`
- **THEN** a `Utilities` row aggregates their buy cost and market value, and shows its share of total market value

#### Scenario: Stock without a sector
- **WHEN** a stock has no sector recorded
- **THEN** it is grouped under an explicit "未分類" row rather than being omitted from the rollup

### Requirement: Manual stock display order
For each market the system SHALL persist a user-controlled stock display order. Imported stocks SHALL initially use their row order on the workbook's `港股` or `美股` summary sheet. The summary SHALL let the user drag a stock row to a new position and persist that order without changing any trade or calculation data.

#### Scenario: Initial order after import
- **WHEN** the workbook is imported
- **THEN** each market's stock list initially follows the order of the corresponding summary sheet

#### Scenario: Reordering a stock
- **WHEN** the user drags a stock above or below another stock in 持倉總覽
- **THEN** the new order is saved, survives a reload, and applies only to that market

#### Scenario: Incomplete reorder request
- **WHEN** a reorder request omits, duplicates, or includes an unknown stock id for the selected market
- **THEN** the system rejects the request and leaves the existing order unchanged

### Requirement: Separation of markets
The system SHALL keep HK and US summaries separate and SHALL NOT mix their currencies in any total.

#### Scenario: Requesting a summary
- **WHEN** the user requests the summary for market `US`
- **THEN** only US stocks and USD-denominated figures are included

#### Scenario: No cross-market total
- **WHEN** the user views either summary
- **THEN** no combined HK+US money total is presented

### Requirement: Bulk 現價 update from a price file
The system SHALL accept a user-supplied JSON price file of the form `{"stocks": [{"symbol": "0388.HK", "price": 395.4}, ...]}` and update 現價 and its last-updated timestamp for every entry that resolves to a stored stock, applying all matched updates in one transaction. The same update SHALL be available from the web UI (file picker) and from a CLI command that reads a local file path. The system SHALL NOT call any external market-data service; the file is user-supplied input equivalent to manual 現價 entry.

Symbols ending in `.HK` SHALL resolve to HK stocks by `ticker` (e.g. `0388.HK` → ticker `0388`). All other symbols SHALL resolve to US stocks by `code` or `ticker`, treating `-` and `.` as equivalent (e.g. `BRK-B` resolves to `BRK.B`).

The system SHALL report, after each upload: which stocks were updated, which symbols did not match any stock, which entries were invalid (missing symbol or non-positive price), and which stored stocks received no price in the file. Unmatched or invalid entries SHALL NOT prevent the matched entries from being applied.

#### Scenario: Uploading a price file
- **WHEN** the user uploads a file containing `{"stocks": [{"symbol": "0388.HK", "price": 395.4}, {"symbol": "VOO", "price": 699.3}]}`
- **THEN** the stock with ticker `0388` in market HK and stock `VOO` in market US each get their 現價 and last-updated timestamp set, and the summary uses the new prices on the next load

#### Scenario: US share-class symbol
- **WHEN** the file contains `{"symbol": "BRK-B", "price": 514.95}` and a US stock `BRK.B` exists
- **THEN** `BRK.B` is updated to 514.95

#### Scenario: Unmatched symbol
- **WHEN** the file contains a symbol such as `1310.HK` that matches no stored stock
- **THEN** that symbol is listed as unmatched in the report and every other matched entry is still applied

#### Scenario: Invalid entry
- **WHEN** the file contains an entry with a missing symbol or a non-positive price
- **THEN** that entry is listed as invalid in the report and is not applied

#### Scenario: Stock absent from the file
- **WHEN** a stored stock has no entry in the uploaded file
- **THEN** its 現價 is left unchanged and it is listed in the report as not updated

#### Scenario: Malformed file
- **WHEN** the uploaded content is not valid JSON or lacks a `stocks` array
- **THEN** the request is rejected with a validation error and no prices are changed

### Requirement: Dividend-adjusted net position
For each stock the system SHALL report 累計派息 as the sum of its **received** dividend amounts only; pending estimates SHALL NOT count toward it. From this the system SHALL derive 累計派息% as `累計派息 ÷ 總買入成本`, 淨投入總本金 as `總買入成本 − 累計派息`, 淨攤薄單價 as `淨投入總本金 ÷ 股數 held`, and 實質動態總回報% as `(當前總市值 − 淨投入總本金) ÷ 淨投入總本金`. 累計派息% SHALL be empty when 總買入成本 is 0, 淨攤薄單價 SHALL be empty when holdings are 0, and 實質動態總回報% SHALL be empty when the stock has no 現價/市值 or when 淨投入總本金 is 0. These figures apply to both HK and US markets and are derived on each read, never stored.

#### Scenario: Net position after received dividends
- **WHEN** `中國銀行` has 總買入成本 208746.64, 股數 held 68000, 現價 5.91, and received dividends totalling 54023.87
- **THEN** the summary reports 累計派息 54023.87, 累計派息% 0.258801, 淨投入總本金 154722.77, 淨攤薄單價 2.275335, and 實質動態總回報% 1.597420 (rounded for display)

#### Scenario: Pending estimate excluded
- **WHEN** a stock has a pending dividend with only an estimated amount
- **THEN** that estimate does not change 累計派息, 淨投入總本金, 淨攤薄單價 or 實質動態總回報%

#### Scenario: No dividends recorded
- **WHEN** a stock has no dividend records
- **THEN** 累計派息 is 0 and 淨投入總本金 equals 總買入成本

#### Scenario: No holdings
- **WHEN** a stock's holdings are 0
- **THEN** 淨攤薄單價 is reported as empty rather than a division error

#### Scenario: No current price
- **WHEN** a stock has no 現價
- **THEN** 實質動態總回報% is reported as empty while the other four figures are still reported

#### Scenario: US market
- **WHEN** the user views the US summary
- **THEN** US stocks show the same five figures computed from their own trades and received dividends

### Requirement: Dividend-adjusted market totals
For each market the system SHALL report totals for 累計派息 and 淨投入總本金 across all stocks, an aggregate 累計派息% as `Σ 累計派息 ÷ Σ 總買入成本` (empty when total buy cost is 0), and an aggregate 實質動態總回報% as `(total market value − 淨投入總本金 of priced stocks) ÷ 淨投入總本金 of priced stocks`, reported as empty when no stock is priced or that denominator is 0.

#### Scenario: Market totals include dividends
- **WHEN** the HK summary is viewed
- **THEN** the totals show the summed 累計派息, the aggregate 累計派息%, the summed 淨投入總本金 across all HK stocks and the aggregate 實質動態總回報% over the priced subset

#### Scenario: Aggregate return excludes unpriced stocks
- **WHEN** a stock has no 現價
- **THEN** its 淨投入總本金 does not enter the denominator of the aggregate 實質動態總回報%, matching how `buy_cost_priced` already excludes it

### Requirement: Hidden stocks excluded from the summary table
The 持倉總覽 table SHALL omit stocks marked hidden (inactive), while the summary response still includes them and they still count toward 總買入成本, 累計派息, 淨投入總本金, market totals and sector rollups — hiding is display-only. The view SHALL indicate how many stocks are hidden so the user knows rows are filtered. Manual stock order SHALL continue to cover hidden stocks, which keep their saved position until unhidden.

#### Scenario: Hidden stock not shown
- **WHEN** `香港寬頻` is hidden and the user opens 持倉總覽
- **THEN** its row is not shown, and a note reports that 1 stock is hidden

#### Scenario: Hidden stock still counted
- **WHEN** a hidden stock has 總買入成本 or received 派息
- **THEN** those figures still contribute to the market totals and its sector's rollup

#### Scenario: Reordering with a hidden stock
- **WHEN** the user drags a visible row while a hidden stock exists
- **THEN** the saved order still covers every stock in the market and the hidden stock keeps its position
