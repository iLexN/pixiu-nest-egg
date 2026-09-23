# Spec Delta

## MODIFIED Requirements

### Requirement: Yearly rollup rows
For each market (HK, US) the system SHALL report one row per calendar year, from the earliest year that has any trade or dividend in that market through the current year. HK and US yearly tables SHALL remain separate and SHALL NOT mix currencies. Each row SHALL carry: `year`, `invested`, `cost` (年末總成本), `market_value` (年末總市值), `dividends` (當年派息), `sold_pl` (賣出損益 — the year's realized P/L), and the derived columns defined below.

#### Scenario: Rows span earliest data to current year
- **WHEN** HK trades exist in 2023–2026 and the current year is 2026
- **THEN** the HK yearly table returns rows for 2023, 2024, 2025, and 2026

#### Scenario: Markets stay separate
- **WHEN** the user requests the US yearly table
- **THEN** only US trades, US dividends, and US snapshots contribute to its rows

### Requirement: Year-end snapshots
The system SHALL persist per `(market, year)` snapshots holding an optional frozen `market_value` (年末總市值), an optional frozen `cost` override, an optional `invested` override, an optional manual `sold_pl` (賣出損益 — the workbook never recorded SELL trades, so the figure is entered by hand), and the time the snapshot was last written. The user SHALL be able to set or clear these values explicitly (manual paste of sheet figures) and to freeze a year, which stores the currently computed year-end `cost` and current total market value as that year's snapshot. A stored snapshot value SHALL take precedence over the computed figure for the same column; the current year's row SHALL fall back to live computed totals when it has no snapshot. `sold_pl` is reported absent while none is stored and is never computed from trades.

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

#### Scenario: Manual sold P/L
- **WHEN** the user stores `sold_pl` `-14991.49` on the HK 2024 snapshot
- **THEN** the HK 2024 row reports `sold_pl` `-14991.49`, and a year with no stored value reports it absent
