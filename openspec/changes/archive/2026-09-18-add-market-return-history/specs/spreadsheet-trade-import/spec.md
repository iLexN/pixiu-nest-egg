# Spec Delta

## ADDED Requirements

### Requirement: Seed market return history
After importing year-end snapshots, the import SHALL create one synthetic market history row per market per year that has both a 成本 and a 總市值 figure, dated at that year's December 31 and carrying the year's `cost` as `buy_cost_priced` and `market_value` as `market_value`. Seeding SHALL be idempotent — a second import adds no duplicate rows. A year missing either figure SHALL be skipped, and a market with no year-end figures gets no seeded rows.

#### Scenario: Year-end rows seeded
- **WHEN** the workbook yields HK year-end figures for 2023, 2024 and 2025 with 成本 and 總市值
- **THEN** synthetic HK history rows dated `2023-12-31`, `2024-12-31` and `2025-12-31` carry each year's figures

#### Scenario: Seeded history feeds the derived figures
- **WHEN** the workbook is imported in January 2027
- **THEN** the HK summary's 上月 resolves to the `2026-12-31` seeded row if present, and 最高 includes every seeded year-end

#### Scenario: Missing figure skipped
- **WHEN** a year-end snapshot has 總市值 but no 成本
- **THEN** no history row is seeded for that market and year

#### Scenario: Idempotent re-import
- **WHEN** the workbook is imported a second time
- **THEN** the seeded year-end history rows are unchanged and no duplicates are created

#### Scenario: Market with no year figures
- **WHEN** the workbook carries no year-end figures for US
- **THEN** no US history rows are seeded and the US 上月/最高 figures build up from recorded summaries only

### Requirement: Seed market last-month and max marks from cached cells
Each market sheet's cached `last month` rate SHALL seed one synthetic market history row dated at the previous calendar month's end, reconstructing `(buy_cost_priced, market_value)` as `(cost, cost × (1 + rate))` where `cost` is the market's imported Σ BUY total. The cached `max Balance %` and `max net` cells SHALL seed per-market maxima marks stored in `app_meta`, which the summary uses as floors for the derived 最高 — they are marks, not history rows, because a lone maximum cannot be decomposed into a `(buy_cost_priced, market_value)` pair. All seeding SHALL be idempotent: a second import never duplicates the last-month row and overwrites the maxima marks with the sheet's latest values. A market whose sheet lacks these cells gets no seeds.

#### Scenario: Last-month row seeded from the cached rate
- **WHEN** the 港股 sheet's `last month` cell holds `0.33` and HK's imported Σ BUY total is `900000`
- **THEN** a synthetic HK history row dated the previous month-end carries `buy_cost_priced` `900000` and `market_value` `1197000`, reproducing the sheet's rate exactly while its amount approximates the true last-month figure

#### Scenario: Maxima marks seeded
- **WHEN** the 港股 sheet caches `max Balance %` `0.3641` and `max net` `362869.79`
- **THEN** the HK summary's 最高 reports at least `0.3641` and `362869.79` until real records exceed them

#### Scenario: US sheet seeds its own figures
- **WHEN** the 美股 sheet caches `last month` `0.023`, `max Balance %` `0.0289` and `max net` `322.58`
- **THEN** US gets the same seeding in its own currency — the HKD-converted duplicate cells are ignored

#### Scenario: Idempotent seeding
- **WHEN** the workbook is imported a second time
- **THEN** no duplicate last-month row appears, the maxima marks reflect the sheet's latest cached values, and a real record already on the seed date is never overwritten
