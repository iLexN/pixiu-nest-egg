# Spec Delta

## ADDED Requirements

### Requirement: 開心Pool contribution totals

`GET /api/months/summary` SHALL return `pool_totals` over every stored year: `ilex_input` = Σ the years' pool income (Σ `interest` × that year's pool rate), `irene_input` = Σ the years' `pool_input`, and `irene_payable` = `ilex_input ÷ 2 − irene_input`. These values SHALL NOT be rounded. `irene_payable` MAY be negative.

#### Scenario: Totals across years

- **WHEN** the stored years have pool income `19386.2658` / `22057.77625` / `23532.23146` and pool_input `0` / `20722.025468` / `4597.611557` (2024–2026)
- **THEN** `pool_totals` reports `ilex_input` ≈ `64976.27`, `irene_input` ≈ `25319.64` and `irene_payable` ≈ `7168.50`

#### Scenario: No stored months

- **WHEN** no month rows are stored
- **THEN** `pool_totals` reports `0` for all three figures

### Requirement: 開心Pool yearly averages

`pool_totals` SHALL also report `avg_spend` = Σ 娛樂支出 ÷ the number of active years and `avg_input` = (`ilex_input` + `irene_input`) ÷ the number of active years. An active year is a stored year whose pool income, `pool_input` or 娛樂支出 is nonzero. The in-progress year counts as a whole year. With no active year, both values SHALL be `0`.

#### Scenario: Averages skip empty years

- **WHEN** 2023 is stored with all-zero pool figures, and 2024–2026 have 娛樂支出 `17719` / `38729.5` / `11170.9` plus the pool income and pool_input above
- **THEN** `avg_spend` ≈ `22539.80` and `avg_input` ≈ `30098.64` (both divided by 3)

#### Scenario: No active years

- **WHEN** no stored year has pool activity
- **THEN** `avg_spend` and `avg_input` are `0`

### Requirement: HappyPool page

The frontend SHALL show a HappyPool view with headline cards 開心 Pool (current pool balance), iLex (`ilex_input`), Irene (`irene_input`), Irene payable (`irene_payable`), 平均支出 (`avg_spend`) and 平均 input (`avg_input`), and a per-year table in ascending year order with columns year, 結餘 (chained pool balance), iLex input (pool income), Irene input (Σ `pool_input`) and 支出 (Σ `entertainment` items). Years with all three inputs zero SHALL be omitted.

#### Scenario: Cards and rows render

- **WHEN** the user opens the HappyPool page with the 2024–2026 data above
- **THEN** the cards show 開心 Pool `22,676.51`, iLex `64,976.27`, Irene `25,319.64` and Irene payable `7,168.50`, 平均支出 `22,539.80` and 平均 input `30,098.64`, and the 2026 row shows 結餘 `22,676.51`, iLex input `23,532.23`, Irene input `4,597.61` and 支出 `11,170.90`

#### Scenario: Copy a row

- **WHEN** the user clicks the row's `copy` button on the 2026 row
- **THEN** the clipboard holds the tab-separated text `2026<TAB>22676.51<TAB>23532.23<TAB>4597.61<TAB>11170.90` (two decimals, no thousands separators or currency symbol), so a paste into a spreadsheet fills five adjacent cells

#### Scenario: Empty year is hidden

- **WHEN** 2023 is stored but has no pool income, no pool_input and no 娛樂支出
- **THEN** the table has no 2023 row
