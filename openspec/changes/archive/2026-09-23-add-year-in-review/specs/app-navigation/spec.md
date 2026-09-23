# Spec Delta

## MODIFIED Requirements

### Requirement: Grouped top navigation

The app SHALL present a two-level top navigation. The first level SHALL show one button per group: 總覽, 股票, 定期, MPF, 債券, AIA, 月結, and 年結, in that order. The second level SHALL show the sub-tabs of the currently active group only — 總覽 SHALL contain 總覽; 股票 SHALL contain 總覽, 交易記錄, 派息, 管理 (in that order); 定期 SHALL contain 總覽, 記錄 (in that order); MPF SHALL contain 總覽; 債券 SHALL contain 總覽; AIA SHALL contain 總覽; 月結 SHALL contain 總覽; 年結 SHALL contain 回顧. The active group and active sub-tab SHALL each be visually marked.

#### Scenario: App loads

- **WHEN** the app loads
- **THEN** the nav shows the 總覽, 股票, 定期, MPF, 債券, AIA, 月結, and 年結 group buttons, the 股票 group is active, and its sub-tabs 總覽, 交易記錄, 派息, 管理 are shown

#### Scenario: Switch to the 總覽 group

- **WHEN** the user clicks the 總覽 group button
- **THEN** the 總覽 group becomes active, its 總覽 sub-tab is shown, and the 總覽 page (headline totals, asset table, 半流動資金, and the IBKR block) is shown

#### Scenario: Switch to the 定期 group

- **WHEN** the user clicks the 定期 group button
- **THEN** the 定期 group becomes active, its sub-tabs 總覽 and 記錄 replace the stock sub-tabs, and the 定期 → 總覽 page (upcoming deposits and rollups) is shown

#### Scenario: Switch to the MPF group

- **WHEN** the user clicks the MPF group button
- **THEN** the MPF group becomes active, its 總覽 sub-tab replaces the previous sub-tabs, and the MPF → 總覽 page (account totals and balances) is shown

#### Scenario: Switch to the 債券 group

- **WHEN** the user clicks the 債券 group button
- **THEN** the 債券 group becomes active, its 總覽 sub-tab replaces the previous sub-tabs, and the 債券 → 總覽 page (active bonds, coupon schedules, and matured history) is shown

#### Scenario: Switch to the AIA group

- **WHEN** the user clicks the AIA group button
- **THEN** the AIA group becomes active, its 總覽 sub-tab replaces the previous sub-tabs, and the AIA → 總覽 page (policy table and USD/HKD totals) is shown

#### Scenario: Switch to the 月結 group

- **WHEN** the user clicks the 月結 group button
- **THEN** the 月結 group becomes active, its 總覽 sub-tab replaces the previous sub-tabs, and the 月結 → 總覽 page (monthly ledger, pool balance, and settings) is shown

#### Scenario: Switch to the 年結 group

- **WHEN** the user clicks the 年結 group button
- **THEN** the 年結 group becomes active, its 回顧 sub-tab replaces the previous sub-tabs, and the 年結 → 回顧 page (the per-year review blocks) is shown

#### Scenario: Only active group's sub-tabs shown

- **WHEN** the 股票 group is active
- **THEN** the 總覽, 定期, MPF, 債券, AIA, 月結, and 年結 groups' sub-tabs are not shown in the nav
