# Spec Delta

## REMOVED Requirements

### Requirement: No external market-data service

**Reason**: The constraint existed so manually entered prices could be verified figure-for-figure against the workbook during migration. Migration is complete and the workbook is retired, so the app may now fetch prices from Yahoo Finance.

**Migration**: Fetching becomes an additional 現價 source alongside the price-file upload and manual entry; both existing paths are unchanged.

## MODIFIED Requirements

### Requirement: Manually entered market data

The system SHALL let the user store a 現價 per stock, together with an optional PE, EPS, high52 and low52, and SHALL record when 現價 was last updated. A stored 現價 may originate from manual entry, a price-file upload, or a user-triggered Yahoo Finance fetch.

#### Scenario: Updating a current price

- **WHEN** the user sets 現價 for `匯豐` to 166.1
- **THEN** the value is stored, the last-updated timestamp is set, and the summary uses it immediately

#### Scenario: No price entered

- **WHEN** a stock has no 現價
- **THEN** its 當前總市值, 未實現金額 and 未實現報酬率 are reported as empty rather than zero, and market totals state that they exclude that stock

## ADDED Requirements

### Requirement: Yahoo Finance price fetch

The system SHALL provide a user-triggered fetch that retrieves the latest price for every stored stock in one market from Yahoo Finance and applies it as 現價 through the same pipeline as a price-file upload. Fetches SHALL be initiated only by an explicit user action; the system SHALL NOT refresh prices automatically. When a live price is unavailable, the previous close SHALL be used.

#### Scenario: Fetching HK prices

- **WHEN** the user triggers a price fetch for market HK
- **THEN** each HK stock is queried as `{ticker}.HK` and every resolved price is stored as its 現價 with a fresh timestamp, rounded to 3 decimal places

#### Scenario: Fetching US prices

- **WHEN** the user triggers a price fetch for market US
- **THEN** each US stock is queried with `.` converted to `-` (e.g. `BRK.B` → `BRK-B`) and every resolved price is stored as its 現價, rounded to 2 decimal places

#### Scenario: Previous close fallback

- **WHEN** a fetch response carries no current price but a previous close
- **THEN** the previous close is applied as the stock's 現價

#### Scenario: Stock without a fetchable symbol

- **WHEN** a stored stock in the market cannot be mapped to a query symbol (e.g. an HK stock with no ticker)
- **THEN** it is reported as unupdated and does not block the other stocks

### Requirement: Price-fetch report

A fetch SHALL report which stocks were updated, which stocks could not be fetched or resolved, and — for a US fetch — the USD rate it applied. Failures for individual symbols SHALL NOT prevent the successful prices from being applied.

#### Scenario: Partial failure

- **WHEN** a fetch for market US succeeds for most symbols but one symbol's request fails
- **THEN** the successful prices are applied and the report lists the failed symbol separately from the updated ones
