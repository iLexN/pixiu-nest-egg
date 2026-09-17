# Stock Yearly Summary Specification

## Purpose

Replaces the hand-maintained per-year table in the owner's spreadsheet/Google Sheet: for each market it rolls up yearly invested, cumulative 成本, year-end 總市值, and 派息 into derived yields and year-over-year changes, storing year-end snapshots so past years stay frozen once prices move on.

## Requirements

### Requirement: Yearly rollup rows
For each market (HK, US) the system SHALL report one row per calendar year, from the earliest year that has any trade or dividend in that market through the current year. HK and US yearly tables SHALL remain separate and SHALL NOT mix currencies. Each row SHALL carry: `year`, `invested`, `cost` (年末總成本), `market_value` (年末總市值), `dividends` (當年派息), and the derived columns defined below. A `sold_pl` column SHALL be reserved in the row and reported empty until a later change defines it.

#### Scenario: Rows span earliest data to current year
- **WHEN** HK trades exist in 2023–2026 and the current year is 2026
- **THEN** the HK yearly table returns rows for 2023, 2024, 2025, and 2026

#### Scenario: Markets stay separate
- **WHEN** the user requests the US yearly table
- **THEN** only US trades, US dividends, and US snapshots contribute to its rows

### Requirement: Computed yearly figures
For each year row the system SHALL compute, from stored trades and dividends in that market only: `invested` as `Σ BUY total − Σ SELL total` for trades with `trade_date` in the year; `cost` as `Σ BUY total` for trades with `trade_date` on or before December 31 of the year (consistent with 總買入成本 — SELL rows do not reduce it); and `dividends` as `Σ received_amount` for dividends with `pay_date` in the year, counting received amounts only and excluding pending estimates. These figures SHALL be derived on each read, never stored, except where a stored snapshot overrides them.

#### Scenario: Per-year invested
- **WHEN** a market has BUY trades totaling 161935.39 and no SELL trades in 2026
- **THEN** the 2026 row reports invested 161935.39

#### Scenario: Cumulative year-end cost
- **WHEN** BUY totals are 96523.15 in 2023, 242135.49 in 2024, and 496006.18 in 2025
- **THEN** the 2025 row reports cost 834664.82

#### Scenario: Dividends count received amounts only
- **WHEN** a market has received dividends totaling 49330.19 with pay_date in 2026 plus a pending dividend with only an estimate
- **THEN** the 2026 row reports dividends 49330.19

### Requirement: Year-end snapshots
The system SHALL persist per `(market, year)` snapshots holding an optional frozen `market_value` (年末總市值), an optional frozen `cost` override, an optional `invested` override, and the time the snapshot was last written. The user SHALL be able to set or clear these values explicitly (manual paste of sheet figures) and to freeze a year, which stores the currently computed year-end `cost` and current total market value as that year's snapshot. A stored snapshot value SHALL take precedence over the computed figure for the same column; the current year's row SHALL fall back to live computed totals when it has no snapshot.

#### Scenario: Freeze the current year
- **WHEN** the user freezes the current year for HK
- **THEN** the snapshot stores the computed cumulative 成本 and current 總市值, and later price changes do not alter the row

#### Scenario: Manual entry for a past year
- **WHEN** the user saves market_value 1022027 for the HK 2025 row
- **THEN** the 2025 row reports that stored value as its 總市值

#### Scenario: Snapshot overrides computed cost
- **WHEN** a snapshot stores cost 760055.41 for 2025 while trades recompute to a different value
- **THEN** the row reports the stored 760055.41

#### Scenario: Past year without snapshot
- **WHEN** a past year has trades and dividends but no snapshot
- **THEN** its market_value is reported empty and derived columns needing it are empty, while invested, cost, and dividends still compute

### Requirement: Derived yearly columns
For each year row the system SHALL derive: `yield_on_cost` (報酬率 1) = dividends ÷ cost when cost is positive; `yield_on_value` (報酬率 2) = dividends ÷ market_value when market_value is present and positive; `monthly_dividend` (月均派息) = dividends ÷ 12; `dividend_yoy` = (dividends − prior year's dividends) ÷ prior year's dividends when the prior-year value is positive; and `invested_yoy` = (cost − prior year's cost) ÷ prior year's cost when the prior-year value is positive. Derived columns SHALL be reported empty when their denominator is missing or zero.

#### Scenario: First year has no YoY
- **WHEN** the earliest year row is rendered
- **THEN** dividend_yoy and invested_yoy are empty

#### Scenario: YoY of cumulative cost
- **WHEN** cost is 338658.64 in 2024 and 834664.82 in 2025
- **THEN** the 2025 row reports invested_yoy ≈ 1.4647

### Requirement: Workbook seeding
The workbook importer SHALL seed `year_snapshots` from year-end figures found in the workbook (the `港股`/`美股` year blocks and the `YearInReview` 股票 cost and "now value" rows) when it can attribute a value to a single market and year. Importing SHALL NOT overwrite an existing snapshot row; a repeated import skips seeded values like other imported data.

#### Scenario: Seed from workbook
- **WHEN** the workbook's YearInReview records a 2024 year-end stock value attributable to HK and the database has no HK 2024 snapshot
- **THEN** import creates the HK 2024 snapshot with that market value

#### Scenario: Re-import preserves snapshots
- **WHEN** the workbook is imported a second time after the user edited a snapshot
- **THEN** the edited snapshot is unchanged and no duplicate rows exist
