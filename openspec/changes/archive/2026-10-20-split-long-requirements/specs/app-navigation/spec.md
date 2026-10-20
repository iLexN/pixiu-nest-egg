# Spec Delta

## MODIFIED Requirements

### Requirement: Grouped top navigation with roll-ups under 總覽

The app SHALL present a two-level top navigation. The first level SHALL show one button per group: 總覽, 股票, 定期, MPF, 債券, AIA, and 家人, in that order. There SHALL be no top-level 月結 or 年結 group. The active group and active sub-tab SHALL each be visually marked.

#### Scenario: App loads

- **WHEN** the app loads
- **THEN** the nav shows the 總覽, 股票, 定期, MPF, 債券, AIA, and 家人 group buttons, the 總覽 group is active, and its sub-tabs 總覽, 月結, 年結 are shown

#### Scenario: Switch to the 總覽 group

- **WHEN** the user clicks the 總覽 group button
- **THEN** the 總覽 group becomes active, its sub-tabs 總覽, 月結, 年結 are shown with 總覽 active, and the 總覽 page (headline totals, asset table, 半流動資金, and the IBKR block) is shown

#### Scenario: Open 總覽 → 月結

- **WHEN** the 總覽 group is active and the user clicks the 月結 sub-tab
- **THEN** the 月結 sub-tab becomes active and the Month Stat page (monthly ledger, pool balance, and settings) is shown

#### Scenario: Open 總覽 → 年結

- **WHEN** the 總覽 group is active and the user clicks the 年結 sub-tab
- **THEN** the 年結 sub-tab becomes active and the year review page (the per-year review blocks) is shown

#### Scenario: Switch to the 定期 group

- **WHEN** the user clicks the 定期 group button
- **THEN** the 定期 group becomes active, its sub-tabs 總覽 and 記錄 replace the previous sub-tabs, and the 定期 → 總覽 page (upcoming deposits and rollups) is shown

#### Scenario: Switch to the MPF group

- **WHEN** the user clicks the MPF group button
- **THEN** the MPF group becomes active, its 總覽 sub-tab replaces the previous sub-tabs, and the MPF → 總覽 page (account totals and balances) is shown

#### Scenario: Switch to the 債券 group

- **WHEN** the user clicks the 債券 group button
- **THEN** the 債券 group becomes active, its 總覽 sub-tab replaces the previous sub-tabs, and the 債券 → 總覽 page (active bonds, coupon schedules, and matured history) is shown

#### Scenario: Switch to the AIA group

- **WHEN** the user clicks the AIA group button
- **THEN** the AIA group becomes active, its 總覽 sub-tab replaces the previous sub-tabs, and the AIA → 總覽 page (policy table and USD/HKD totals) is shown

#### Scenario: Switch to the 家人 group

- **WHEN** the user clicks the 家人 group button
- **THEN** the 家人 group becomes active, its 定期 sub-tab replaces the previous sub-tabs, and the 家人 → 定期 page (per-holder family deposits, notes, and history) is shown

#### Scenario: Only active group's sub-tabs shown

- **WHEN** the 股票 group is active
- **THEN** the 總覽, 定期, MPF, 債券, AIA, and 家人 groups' sub-tabs are not shown in the nav

#### Scenario: No 月結 or 年結 group button

- **WHEN** the app loads
- **THEN** the first-level nav contains no 月結 and no 年結 button; those pages are reachable only as sub-tabs of 總覽

## ADDED Requirements

### Requirement: Sub-tab membership per group

The second level SHALL show the sub-tabs of the currently active group only — 總覽 SHALL contain 總覽, 月結, 年結 (in that order); 股票 SHALL contain 總覽, 交易記錄, 派息, 管理 (in that order); 定期 SHALL contain 總覽, 記錄 (in that order); MPF SHALL contain 總覽; 債券 SHALL contain 總覽; AIA SHALL contain 總覽; 家人 SHALL contain 定期.

#### Scenario: Switching groups

- **WHEN** the user activates the 股票 group
- **THEN** the second level shows 總覽, 交易記錄, 派息, 管理 in that order and no other group's tabs

