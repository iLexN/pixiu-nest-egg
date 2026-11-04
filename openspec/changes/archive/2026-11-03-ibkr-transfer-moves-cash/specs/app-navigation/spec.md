# Spec Delta

## MODIFIED Requirements

### Requirement: Sub-tab membership per group

The second level SHALL show the sub-tabs of the currently active group only — 總覽 SHALL contain 總覽, 月結, 年結, HappyPool (in that order); 股票 SHALL contain 總覽, 交易記錄, 派息, 管理 (in that order), followed by 轉入記錄 while the market is 美股; 定期 SHALL contain 總覽, 記錄 (in that order); MPF SHALL contain 總覽; 債券 SHALL contain 總覽; AIA SHALL contain 總覽; 家人 SHALL contain 定期.

#### Scenario: Switching groups

- **WHEN** the user activates the 股票 group with the 港股 market selected
- **THEN** the second level shows 總覽, 交易記錄, 派息, 管理 in that order and no other group's tabs

#### Scenario: 美股 adds 轉入記錄

- **WHEN** the 股票 group is active and the user selects 美股
- **THEN** the second level shows 總覽, 交易記錄, 派息, 管理, 轉入記錄 in that order

### Requirement: Market toggle is hidden outside the 股票 group

The 港股/美股 market toggle SHALL be shown while the 股票 group is active and SHALL control the market for all of its sub-pages (總覽, 交易記錄, 派息, 管理, and the 美股-only 轉入記錄). It SHALL be hidden while the 總覽 (including its 月結 and 年結 sub-tabs), 定期, MPF, 債券, AIA, or 家人 group is active. The selected market SHALL persist when switching between stock sub-tabs. Switching to 港股 while 轉入記錄 is open SHALL open 股票 → 總覽.

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

#### Scenario: Switching to 港股 leaves 轉入記錄

- **WHEN** the user is on 股票 → 轉入記錄 and selects 港股
- **THEN** 股票 → 總覽 opens for 港股 and the 轉入記錄 tab is no longer shown
