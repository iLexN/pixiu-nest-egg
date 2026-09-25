# Spec Delta

## ADDED Requirements

### Requirement: Grouped top navigation with roll-ups under 總覽

The app SHALL present a two-level top navigation. The first level SHALL show one button per group: 總覽, 股票, 定期, MPF, 債券, AIA, and 家人, in that order. The second level SHALL show the sub-tabs of the currently active group only — 總覽 SHALL contain 總覽, 月結, 年結 (in that order); 股票 SHALL contain 總覽, 交易記錄, 派息, 管理 (in that order); 定期 SHALL contain 總覽, 記錄 (in that order); MPF SHALL contain 總覽; 債券 SHALL contain 總覽; AIA SHALL contain 總覽; 家人 SHALL contain 定期. There SHALL be no top-level 月結 or 年結 group. The active group and active sub-tab SHALL each be visually marked.

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

### Requirement: Default page is 總覽 → 總覽

The app SHALL open on the 總覽 group's 總覽 page (the headline totals, asset table, 半流動資金, and IBKR block).

#### Scenario: Initial landing page

- **WHEN** the app loads with no prior navigation
- **THEN** the 總覽 page is displayed, the 總覽 group button is marked active, and 總覽 is marked as the active sub-tab

### Requirement: Market toggle is hidden outside the 股票 group

The 港股/美股 market toggle SHALL be shown while the 股票 group is active and SHALL control the market for all of its sub-pages (總覽, 交易記錄, 派息, 管理). It SHALL be hidden while the 總覽 (including its 月結 and 年結 sub-tabs), 定期, MPF, 債券, AIA, or 家人 group is active. The selected market SHALL persist when switching between stock sub-tabs.

#### Scenario: Market toggle on stock pages

- **WHEN** the 股票 group is active on any of its sub-tabs
- **THEN** the 港股/美股 toggle is shown and switching it changes the market of the displayed page

#### Scenario: Market toggle hidden for 總覽

- **WHEN** the user switches to the 總覽 group
- **THEN** the 港股/美股 toggle is not shown

#### Scenario: Market toggle hidden for 總覽 → 月結

- **WHEN** the user opens the 月結 sub-tab of 總覽
- **THEN** the 港股/美股 toggle is not shown

#### Scenario: Market toggle hidden for 總覽 → 年結

- **WHEN** the user opens the 年結 sub-tab of 總覽
- **THEN** the 港股/美股 toggle is not shown

#### Scenario: Market toggle hidden for 定期

- **WHEN** the user switches to the 定期 group
- **THEN** the 港股/美股 toggle is not shown

#### Scenario: Market toggle hidden for MPF

- **WHEN** the user switches to the MPF group
- **THEN** the 港股/美股 toggle is not shown

#### Scenario: Market toggle hidden for 債券

- **WHEN** the user switches to the 債券 group
- **THEN** the 港股/美股 toggle is not shown

#### Scenario: Market toggle hidden for AIA

- **WHEN** the user switches to the AIA group
- **THEN** the 港股/美股 toggle is not shown

#### Scenario: Market toggle hidden for 家人

- **WHEN** the user switches to the 家人 group
- **THEN** the 港股/美股 toggle is not shown

#### Scenario: Market persists across stock sub-tabs

- **WHEN** the user selects 美股 on 總覽 and then opens 交易記錄
- **THEN** 交易記錄 shows US trades

## MODIFIED Requirements

### Requirement: Group selection opens the group's first sub-tab

Clicking a group button SHALL switch to that group and open its first sub-tab. The app SHALL NOT remember a previously visited sub-tab within a group.

#### Scenario: Return to a group

- **WHEN** the user is on 股票 → 管理, clicks 定期, then clicks 股票 again
- **THEN** the 股票 → 總覽 page is shown, not 管理

#### Scenario: Return to 總覽 after visiting 年結

- **WHEN** the user is on 總覽 → 年結, clicks 定期, then clicks 總覽 again
- **THEN** the 總覽 → 總覽 page is shown, not 年結

## REMOVED Requirements

### Requirement: Grouped top navigation

**Reason**: The 月結 and 年結 groups no longer exist, so the "Switch to the 月結 group" / "Switch to the 年結 group" scenarios cannot survive; superseded by "Grouped top navigation with roll-ups under 總覽" above.
**Migration**: None — 月結 and 年結 are reached as sub-tabs of 總覽.

### Requirement: Market toggle is scoped to the 股票 group

**Reason**: Its "hidden for 月結" / "hidden for 年結" scenarios refer to groups that no longer exist; superseded by "Market toggle is hidden outside the 股票 group" above, which covers the same pages as 總覽 sub-tabs.
**Migration**: None — behavior of the toggle is unchanged.

### Requirement: Default page is 股票 → 總覽

**Reason**: The app has opened on the 總覽 group for some time and that is the intended landing page; the requirement is replaced by "Default page is 總覽 → 總覽" above.
**Migration**: None — no user action; the stock portfolio summary remains one click away at 股票 → 總覽.
