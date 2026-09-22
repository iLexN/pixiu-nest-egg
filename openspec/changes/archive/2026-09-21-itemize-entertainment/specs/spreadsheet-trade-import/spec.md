# Spec Delta

## MODIFIED Requirements

### Requirement: Importing Month Stat rows from the workbook

The import command SHALL also read the `Month Stat` sheet's monthly rows (one row per month under the header row carrying 月初(出糧後)) and create one month row per dated row, storing `start_cash` (F), `interest` (N), `pool_input` (P) and `note`, plus the cached `total_assets` (B) and `liquid_assets` (D) as frozen values. Cells containing formulas SHALL be read as their computed values; a B/D cell holding a live `Overview` link SHALL import with no stored value so the row derives live. Each month's `salary` SHALL be recovered from the 月尾 formula's salary literal where present, else the latest known earlier salary, else the workbook's `Overview!E1`. An H cell that is not the `=F<next row> − …` chain formula SHALL store its cached value as `end_cash_override` (blank H stores none), and re-imports SHALL fill the override on existing rows only while it is NULL. The sheet's 調整 (G), extra-spend, 存-extra and 娛樂支出 (O) formula sums SHALL each import as a single `month_items` entry — `adjustment`, `extra_spend`, `income` and `entertainment` respectively — carrying the cached total and keeping the formula text as its `note`; imported `entertainment` items SHALL NOT carry `exclude_from_living` (the O↔J overlap is not re-linked on import). Months with only a date SHALL be skipped. Rows SHALL be imported in workbook order.

#### Scenario: Importing the current workbook

- **WHEN** the import command is run against `財富分析報告.xlsx`
- **THEN** every month row from 2023-12 onward is stored — history rows carrying their frozen 總數/流動資產 — and the report lists how many month rows and items were imported and skipped

#### Scenario: Live-linked current month

- **WHEN** the current month's B/D cells hold `=Overview!$B$1`/`=Overview!$H$1`
- **THEN** the imported row stores no `total_assets`/`liquid_assets` and derives them live

#### Scenario: Formula sum kept as a note

- **WHEN** a month's 調整 cell holds `=28780.19 + 15775.79 - 35844.03 + 2743 - 10000 - 25000 - 39282.6`
- **THEN** one adjustment item is stored with amount `-62827.65` and the formula text as its note

#### Scenario: 娛樂支出 imports as an item

- **WHEN** a month's O cell holds `=500 + 4700 + 75`
- **THEN** one `entertainment` item is stored with amount `5275` and the formula text as its note, while the separate `=I − 4700` exclusion still imports as an `extra_spend` item of `4700`

### Requirement: Month Stat parity report

The parity command SHALL also compare the app's Month Stat figures against the workbook's cached values — each stored month's `start_cash`, `interest` and `pool_input`, the derived `entertainment` sum against O, the frozen `total_assets`/`liquid_assets`, the derived `end_cash`/`month_spend`/`living_spend`/`living_yoy`/`saved` where the sheet caches them, the yearly aggregate block, and the seeded settings — and report any difference beyond a small floating-point tolerance. Figures for the sheet's live-linked current row and rows the user has edited since import SHALL be reported as informational differences rather than failures.

#### Scenario: Figures match

- **WHEN** the parity command is run after a successful import against a freshly recalculated workbook
- **THEN** the Month Stat section reports zero differences beyond the expected live/current-row drift

#### Scenario: Figures differ

- **WHEN** an imported month's stored or derived figure differs from the cached value beyond tolerance
- **THEN** the command lists the differing month and field with both values and exits with a non-zero status
