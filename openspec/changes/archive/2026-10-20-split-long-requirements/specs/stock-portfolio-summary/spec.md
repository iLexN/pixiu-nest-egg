# Spec Delta

## MODIFIED Requirements

### Requirement: Bulk 現價 update from a price file

The system SHALL accept a user-supplied JSON price file of the form `{"stocks": [{"symbol": "0388.HK", "price": 395.4}, ...]}` and update 現價 and its last-updated timestamp for every entry that resolves to a stored stock, applying all matched updates in one transaction. The same update SHALL be available from the web UI (file picker) and from a CLI command that reads a local file path.

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

## ADDED Requirements

### Requirement: No external market-data service

The system SHALL NOT call any external market-data service; the file is user-supplied input equivalent to manual 現價 entry.

#### Scenario: Prices come only from the file

- **WHEN** a 現價 is updated in the app
- **THEN** it comes from a user-supplied price file or manual entry, never an external market-data service

## ADDED Requirements

### Requirement: Price-file symbol resolution

Symbols ending in `.HK` SHALL resolve to HK stocks by `ticker` (e.g. `0388.HK` → ticker `0388`). All other symbols SHALL resolve to US stocks by `code` or `ticker`, treating `-` and `.` as equivalent (e.g. `BRK-B` resolves to `BRK.B`).

#### Scenario: Symbol resolution

- **WHEN** a price file carries `0388.HK` and `BRK-B`
- **THEN** the first updates the HK stock with ticker `0388` and the second the US stock `BRK.B`

## ADDED Requirements

### Requirement: Price-file upload report

The system SHALL report, after each upload: which stocks were updated, which symbols did not match any stock, which entries were invalid (missing symbol or non-positive price), and which stored stocks received no price in the file. Unmatched or invalid entries SHALL NOT prevent the matched entries from being applied.

#### Scenario: Partial upload still applies

- **WHEN** an upload contains one matched symbol, one unmatched symbol, and one entry with a non-positive price
- **THEN** the report lists each outcome and the matched stock's 現價 is updated anyway

## MODIFIED Requirements

### Requirement: Dividend-adjusted net position

For each stock the system SHALL report 累計派息 as the sum of its **received** dividend amounts only; pending estimates SHALL NOT count toward it. These figures apply to both HK and US markets and are derived on each read, never stored.

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

## ADDED Requirements

### Requirement: Net-position derived figures

From this the system SHALL derive 累計派息% as `累計派息 ÷ 總買入成本`, 淨投入總本金 as `總買入成本 − 累計派息`, 淨攤薄單價 as `淨投入總本金 ÷ 股數 held`, and 實質動態總回報% as `(當前總市值 − 淨投入總本金) ÷ 淨投入總本金`. 累計派息% SHALL be empty when 總買入成本 is 0, 淨攤薄單價 SHALL be empty when holdings are 0, and 實質動態總回報% SHALL be empty when the stock has no 現價/市值 or when 淨投入總本金 is 0.

#### Scenario: Net position for 中國銀行

- **WHEN** `中國銀行` has 總買入成本 208746.64, 股數 held 68000, 現價 5.91, and received dividends totalling 54023.87
- **THEN** the summary reports 累計派息 54023.87, 累計派息% 0.258801, 淨投入總本金 154722.77, 淨攤薄單價 2.275335, and 實質動態總回報% 1.597420 (rounded for display)

## MODIFIED Requirements

### Requirement: Market totals history

When a market summary is computed and at least one stock is priced, the system SHALL record a history row for that market dated with the day of computation, carrying that day's `buy_cost_priced` and `market_value`. Computing the same market's summary again on the same calendar day SHALL replace that day's row rather than append a second one, so repeated builds do not pollute the record.

#### Scenario: Summary records a history row
- **WHEN** the HK summary is computed on `2026-09-18` with at least one priced stock
- **THEN** a history row exists for market HK dated `2026-09-18` carrying that day's `buy_cost_priced` and `market_value`

#### Scenario: Same-day replace
- **WHEN** a market's summary is computed twice on the same calendar day and the totals changed between builds
- **THEN** exactly one history row exists for that market and day, carrying the latest values

#### Scenario: Month-rollover backfill
- **WHEN** a market's latest history row is dated in August 2026 and the next record happens on `2026-10-05`
- **THEN** a synthetic history row dated in September 2026 is created carrying the August values, and `2026-10-05` gets the current values' row

#### Scenario: Unpriced market records nothing
- **WHEN** no stock in a market has 現價
- **THEN** computing that market's summary records no history row

#### Scenario: Bulk price update captured for every touched market
- **WHEN** a price file updates 現價 for stocks in both HK and US
- **THEN** both markets have a history row for that day, even if only one market's summary is viewed afterward

## ADDED Requirements

### Requirement: Month-end history backfill

When a summary is computed and one or more calendar months have fully elapsed with no history rows since the market's previous record, the system SHALL insert a synthetic month-end history row for each elapsed empty month, carrying the last-recorded values forward. No history row SHALL be recorded for a market while none of its stocks is priced.

#### Scenario: Gap months backfill

- **WHEN** a market's last history row is two months old and a summary is computed
- **THEN** a synthetic month-end row carrying the last-recorded values exists for each elapsed empty month

## MODIFIED Requirements

### Requirement: Last-month and max unrealized figures

For each market the system SHALL derive, on read: the 上月 figures from the latest history row in the previous calendar month (empty when no such row exists), and the 最高 figures as the maxima over the market's stored seed marks (per market in `app_meta`, absent when none were ever stored), every history row, plus the market's current values.

#### Scenario: Last month from history
- **WHEN** HK's latest history row in August 2026 carries `buy_cost_priced` 500000 and `market_value` 600000, viewed in September 2026
- **THEN** the HK summary's 上月 shows 未實現報酬率 `0.2` and 未實現金額 `100000`

#### Scenario: No previous-month rows
- **WHEN** a market has history rows only in the current calendar month
- **THEN** its 上月 figures are empty

#### Scenario: Max rises automatically
- **WHEN** a market's reported 最高 未實現報酬率 is `0.47` and a new record makes the current 未實現報酬率 `0.50`
- **THEN** the reported 最高 未實現報酬率 becomes `0.50` with no manual step

#### Scenario: Independent maxima
- **WHEN** a market's highest 未實現報酬率 occurred in March and its highest 未實現金額 occurred in August
- **THEN** 最高 reports the March rate and the August amount

#### Scenario: Current counts toward max
- **WHEN** the current 未實現金額 exceeds every recorded history row
- **THEN** 最高 未實現金額 reflects the current value even before a history row is written for it

#### Scenario: Seed mark floors the max
- **WHEN** HK's stored seed marks are `0.3641` / `362869.79` and no history row or current value exceeds them
- **THEN** 最高 reports `0.3641` and `362869.79`, and a new record above either mark raises that figure automatically

## ADDED Requirements

### Requirement: Unrealized figure pairs and max tracking

Each figure pair carries 未實現報酬率 as `(market_value − buy_cost_priced) ÷ buy_cost_priced`, reported empty when `buy_cost_priced` is 0, and 未實現金額 as `market_value − buy_cost_priced`. The max 未實現報酬率 and max 未實現金額 SHALL be tracked independently — they may come from different moments. When a new record sets a new high, the reported 最高 SHALL rise automatically without any manual copy step.

#### Scenario: Last month read from history

- **WHEN** HK's latest history row in August 2026 carries `buy_cost_priced` 500000 and `market_value` 600000, viewed in September 2026
- **THEN** the HK summary's 上月 shows 未實現報酬率 `0.2` and 未實現金額 `100000`

