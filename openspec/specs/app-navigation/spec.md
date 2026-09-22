# app-navigation Specification

## Purpose

Defines the app's top-level navigation: which pages are grouped under 股票, 定期, MPF, 債券, and AIA, the labels and order of each group's sub-tabs, when the 港股/美股 market toggle is visible, and which page the app opens on.

## Requirements

### Requirement: Grouped top navigation

The app SHALL present a two-level top navigation. The first level SHALL show one button per group: 總覽, 股票, 定期, MPF, 債券, AIA, and 月結, in that order. The second level SHALL show the sub-tabs of the currently active group only — 總覽 SHALL contain 總覽; 股票 SHALL contain 總覽, 交易記錄, 派息, 管理 (in that order); 定期 SHALL contain 總覽, 記錄 (in that order); MPF SHALL contain 總覽; 債券 SHALL contain 總覽; AIA SHALL contain 總覽; 月結 SHALL contain 總覽. The active group and active sub-tab SHALL each be visually marked.

#### Scenario: App loads

- **WHEN** the app loads
- **THEN** the nav shows the 總覽, 股票, 定期, MPF, 債券, AIA, and 月結 group buttons, the 股票 group is active, and its sub-tabs 總覽, 交易記錄, 派息, 管理 are shown

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

#### Scenario: Only active group's sub-tabs shown

- **WHEN** the 股票 group is active
- **THEN** the 總覽, 定期, MPF, 債券, AIA, and 月結 groups' sub-tabs are not shown in the nav

### Requirement: Default page is 股票 → 總覽

The app SHALL open on the 股票 group's 總覽 page (the stock portfolio summary) instead of 交易記錄.

#### Scenario: Initial landing page

- **WHEN** the app loads with no prior navigation
- **THEN** the stock portfolio summary for the default market is displayed, with 總覽 marked as the active sub-tab

### Requirement: Group selection opens the group's first sub-tab

Clicking a group button SHALL switch to that group and open its first sub-tab (its 總覽). The app SHALL NOT remember a previously visited sub-tab within a group.

#### Scenario: Return to a group

- **WHEN** the user is on 股票 → 管理, clicks 定期, then clicks 股票 again
- **THEN** the 股票 → 總覽 page is shown, not 管理

### Requirement: Market toggle is scoped to the 股票 group

The 港股/美股 market toggle SHALL be shown while the 股票 group is active and SHALL control the market for all of its sub-pages (總覽, 交易記錄, 派息, 管理). It SHALL be hidden while the 總覽, 定期, MPF, 債券, AIA, or 月結 group is active. The selected market SHALL persist when switching between stock sub-tabs.

#### Scenario: Market toggle on stock pages

- **WHEN** the 股票 group is active on any of its sub-tabs
- **THEN** the 港股/美股 toggle is shown and switching it changes the market of the displayed page

#### Scenario: Market toggle hidden for 總覽

- **WHEN** the user switches to the 總覽 group
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

#### Scenario: Market toggle hidden for 月結

- **WHEN** the user switches to the 月結 group
- **THEN** the 港股/美股 toggle is not shown

#### Scenario: Market persists across stock sub-tabs

- **WHEN** the user selects 美股 on 總覽 and then opens 交易記錄
- **THEN** 交易記錄 shows US trades
