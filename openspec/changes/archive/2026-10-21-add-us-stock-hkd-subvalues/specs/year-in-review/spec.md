# Spec Delta

## MODIFIED Requirements

### Requirement: US stocks and deposits year figures

The row SHALL carry:

- `us_stocks` (labeled 美股 — USD figures, not HKD-converted): `cost`, `dividends`, `rate`, `now_value`, `value_rate` derived exactly as `stocks` but from US trades, dividends, and snapshots. It SHALL also carry `cost_hkd` and `now_value_hkd` — `cost`/`now_value` × the stored USD→HKD rate — as display companions, each absent while its USD figure or the rate is missing.
- `deposits`: `principal`/`interest` = Σ over 定期 rows whose `end_date` falls in the year and is not after today (the sheet's `End` filter); a seeded override replaces either value when present.

#### Scenario: US figures stay in USD

- **WHEN** the year carries US trades and dividends
- **THEN** `us_stocks` reports USD figures, not HKD-converted

#### Scenario: HKD companions convert at the stored rate

- **WHEN** the year's `us_stocks` reports `cost` `1000` USD and `now_value` `5000` USD and the stored USD→HKD rate is `7.8`
- **THEN** the entry reports `cost_hkd` `7800` and `now_value_hkd` `39000`

#### Scenario: HKD companions absent without a rate

- **WHEN** the year's `us_stocks` reports a `cost` figure and no USD→HKD rate is stored
- **THEN** `cost_hkd` is absent while `cost` still reports

### Requirement: Year review page

The frontend SHALL show a 總覽 → 年結 view listing one block per year reproducing the sheet's three groups — ledger aggregates, investment summary, and per-asset-class returns (the asset rows labeled 債券/港股/美股/定期, 港股 figures being HK-only and 美股 figures USD) — with their YoY columns. The 美股 成本 and 美股 市值 cells SHALL also show the HKD-converted figure as a muted sub-line beneath the USD value.

#### Scenario: Edit a manual figure

- **WHEN** the user edits the 2026 收入 cell to `740000`
- **THEN** the view patches the year and reloads with the new derived cells

#### Scenario: Clearing raise restores the derived figure

- **WHEN** the 2026 record stores a `raise` override and the user clears it
- **THEN** the view patches `raise: null` and reloads showing the salary-derived `raise`

#### Scenario: HKD sub-line under the USD figure

- **WHEN** a year row carries `us_stocks.cost_hkd`
- **THEN** the 美股 成本 cell shows the HKD figure beneath the USD value
