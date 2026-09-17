# app-navigation Specification

## Purpose

Defines the app's top-level navigation: which pages are grouped under 股票, 定期, and MPF, the labels and order of each group's sub-tabs, when the 港股/美股 market toggle is visible, and which page the app opens on.

## Requirements

### Requirement: Grouped top navigation

The app SHALL present a two-level top navigation. The first level SHALL show one button per group: 股票, 定期, and MPF, in that order. The second level SHALL show the sub-tabs of the currently active group only — 股票 SHALL contain 總覽, 交易記錄, 派息, 管理 (in that order); 定期 SHALL contain 總覽, 記錄 (in that order); MPF SHALL contain 總覽. The active group and active sub-tab SHALL each be visually marked.

#### Scenario: App loads

- **WHEN** the app loads
- **THEN** the nav shows the 股票, 定期, and MPF group buttons, the 股票 group is active, and its sub-tabs 總覽, 交易記錄, 派息, 管理 are shown

#### Scenario: Switch to the 定期 group

- **WHEN** the user clicks the 定期 group button
- **THEN** the 定期 group becomes active, its sub-tabs 總覽 and 記錄 replace the stock sub-tabs, and the 定期 → 總覽 page (upcoming deposits and rollups) is shown

#### Scenario: Switch to the MPF group

- **WHEN** the user clicks the MPF group button
- **THEN** the MPF group becomes active, its 總覽 sub-tab replaces the previous sub-tabs, and the MPF → 總覽 page (account totals and balances) is shown

#### Scenario: Only active group's sub-tabs shown

- **WHEN** the 股票 group is active
- **THEN** the 定期 and MPF groups' sub-tabs are not shown in the nav

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

The 港股/美股 market toggle SHALL be shown while the 股票 group is active and SHALL control the market for all of its sub-pages (總覽, 交易記錄, 派息, 管理). It SHALL be hidden while the 定期 or MPF group is active. The selected market SHALL persist when switching between stock sub-tabs.

#### Scenario: Market toggle on stock pages

- **WHEN** the 股票 group is active on any of its sub-tabs
- **THEN** the 港股/美股 toggle is shown and switching it changes the market of the displayed page

#### Scenario: Market toggle hidden for 定期

- **WHEN** the user switches to the 定期 group
- **THEN** the 港股/美股 toggle is not shown

#### Scenario: Market toggle hidden for MPF

- **WHEN** the user switches to the MPF group
- **THEN** the 港股/美股 toggle is not shown

#### Scenario: Market persists across stock sub-tabs

- **WHEN** the user selects 美股 on 總覽 and then opens 交易記錄
- **THEN** 交易記錄 shows US trades
