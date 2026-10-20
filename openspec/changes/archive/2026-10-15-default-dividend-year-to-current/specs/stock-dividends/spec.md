# Spec Delta

## ADDED Requirements

### Requirement: Received-records year filter default

The 已收記錄 card on each market's dividend view SHALL offer a year filter over the years that have received records, defaulting to the current year on load and on market switch; when the current year holds no received records it SHALL fall back to the latest year that does, and stay on the current year when no year has any. 全部年份 SHALL remain selectable to show every year.

#### Scenario: Default is the current year

- **WHEN** the user opens the 港股 dividend view and 2026 has received records
- **THEN** the 已收記錄 filter selects 2026 and lists only that year's records

#### Scenario: Current year has no records

- **WHEN** the latest year with received records is 2025
- **THEN** the filter selects 2025 instead of showing an empty 2026

#### Scenario: All years still reachable

- **WHEN** the user picks 全部年份
- **THEN** the card lists every received record regardless of year
