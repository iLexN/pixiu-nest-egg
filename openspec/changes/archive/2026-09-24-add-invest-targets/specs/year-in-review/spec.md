# Spec Delta

## ADDED Requirements

### Requirement: 月薪增幅 derivation

Each year row SHALL carry `raise` (月薪增幅 — the monthly salary increment feeding the Overview 投資目標 formula): the year's stored `raise` override when set, otherwise derived live as `max(0, lastSalary(Y) − lastSalary(Y-1))`, where `lastSalary(y)` is the `salary` of the latest stored month row of year `y` that carries one. The derived figure SHALL be absent while either year has no stored month salary; the Overview target formula counts an absent `raise` as `0`.

#### Scenario: Derived from stored salaries

- **WHEN** the latest 2026 month salary is `52700`, the latest 2025 month salary is `50810`, and no override is stored
- **THEN** the 2026 row reports `raise` `1890`

#### Scenario: Override wins

- **WHEN** the 2026 record stores `raise` `2500`
- **THEN** the 2026 row reports `raise` `2500` regardless of the salary history

#### Scenario: Flat salary derives zero

- **WHEN** the latest stored salary is unchanged between two consecutive years
- **THEN** the later year's derived `raise` is `0`

#### Scenario: No salary history

- **WHEN** the prior year has no stored month salary
- **THEN** the year's derived `raise` is absent and the Overview target counts it as `0`

## MODIFIED Requirements

### Requirement: Stored manual figures and overrides

The system SHALL persist one record per year carrying `income` (收入, manual), `invested_adjustment` (manual add-on to the year's `invested`, on top of its 轉入), `raise` (manual override for the year's 月薪增幅; NULL derives live from month salaries), and the nullable overrides `bond_principal`, `bond_interest`, `deposit_principal`, `deposit_interest`. `PATCH /api/year-review/:year` SHALL upsert the record — absent fields unchanged, a value sets it, `null` clears an override back to live derivation — and SHALL also accept `sold_pl`, stored as the HK `year_snapshots` `sold_pl` for that year. Values SHALL be finite numbers; `income`, principals, and interests SHALL be non-negative; `raise` may be negative to deliberately lower the target.

#### Scenario: Clearing an override re-derives

- **WHEN** the 2025 record has `bond_principal` `160000` and the user patches the year with `bond_principal: null`
- **THEN** the override is cleared and the row reports the coupon-derived principal

#### Scenario: Editing income reprices derived cells

- **WHEN** the user patches 2026 with `income` `740000`
- **THEN** `income`, `income_avg`, `invested_pct`, `saved`, `saved_avg`, and `saved_pct` all reflect the new figure on the next read

#### Scenario: Setting raise reprices the target

- **WHEN** the user patches 2026 with `raise` `2500`
- **THEN** the 2026 row reports `raise` `2500` and the Overview 投資目標 `target`/`remain`/`growth` reflect it on the next read

### Requirement: Workbook seeding

The workbook importer SHALL seed `year_review` records from `YearInReview`'s year blocks: `income` from the 收入 cell and `invested_adjustment` = the sheet's `invested` figure minus the HK net invested computed from trades and the year's `ibkr_transfers` sum, for every block year; `sold_pl` from each block's 投資P/L cell into the HK `year_snapshots` row; and the four bond/deposit overrides only for years before the current year (the current year derives live). The importer SHALL also seed `invested_adjustment` by the same sheet-figure-minus-derived formula from the `Overview` `J23:K26` year/invested cells, covering years the `YearInReview` sheet has no block for. Seeding SHALL NOT overwrite existing stored values, and a repeated import SHALL create no duplicates — matching the `year_snapshots` convention.

#### Scenario: Import seeds past-year overrides

- **WHEN** the workbook's 2024 block carries bond principal `130000` and interest `5728.26`, and deposit principal `795095.92` and interest `10208.01`
- **THEN** import stores those four values on the 2024 record

#### Scenario: Current-year manual inputs still seed

- **WHEN** the workbook's current-year block carries `income` `737020` and `invested` `182935.39`, trades derive HK net invested `161935.39`, and the seeded transfer log sums `131000` for the year
- **THEN** import stores `income` `737020` and `invested_adjustment` `-110000` for the current year, while its bond/deposit cells stay NULL

#### Scenario: Overview-only year seeds its adjustment

- **WHEN** the `Overview` block lists 2023 with `invested` `206523.15`, the HK snapshot invests `96523.15`, and no 2023 transfers exist
- **THEN** import stores a 2023 `year_review` record with `invested_adjustment` `110000` and the 2023 year row reports `invested` `206523.15`

#### Scenario: Re-import keeps edits

- **WHEN** the user edited the seeded 2025 `income` and the workbook is imported again
- **THEN** the edited value is unchanged

### Requirement: Year review page

The frontend SHALL show a 年結 → 回顧 view listing one block per year reproducing the sheet's three groups — ledger aggregates, investment summary, and per-asset-class returns — with their YoY columns, the derived `IBKR 轉入` figure, the effective `raise` per year, inline editing for `income`, `invested_adjustment`, `raise`, `sold_pl`, and the four overrides (clearing restores derived figures), and a reload after every mutation.

#### Scenario: Edit a manual figure

- **WHEN** the user edits the 2026 收入 cell to `740000`
- **THEN** the view patches the year and reloads with the new derived cells

#### Scenario: Clearing raise restores the derived figure

- **WHEN** the 2026 record stores a `raise` override and the user clears it
- **THEN** the view patches `raise: null` and reloads showing the salary-derived `raise`
