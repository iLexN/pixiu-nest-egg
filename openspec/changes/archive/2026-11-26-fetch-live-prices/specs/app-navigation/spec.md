# Spec Delta

## ADDED Requirements

### Requirement: 更新現價 action in the 股票 nav row

The 股票 group's second-level nav SHALL show a `更新現價` action button immediately after the 管理 sub-tab (before 轉入記錄 when the market is 美股). It is an action, not a sub-tab: clicking it SHALL NOT change the active sub-tab. Clicking it SHALL start a price fetch for the currently selected market, display a blocking overlay that prevents all other interaction until the fetch completes, then show the fetch outcome inline and refresh the displayed data.

#### Scenario: Button placement for 港股

- **WHEN** the 股票 group is active with the 港股 market selected
- **THEN** the second-level row shows 總覽, 交易記錄, 派息, 管理, then the 更新現價 button, in that order

#### Scenario: Button placement for 美股

- **WHEN** the 股票 group is active with the 美股 market selected
- **THEN** the second-level row shows 總覽, 交易記錄, 派息, 管理, 更新現價, 轉入記錄 in that order

#### Scenario: Clicking fetches for the selected market

- **WHEN** the user clicks 更新現價 while the 美股 market is selected
- **THEN** a blocking overlay is shown, a price fetch runs for market US only, and when it completes the overlay is dismissed, the outcome is displayed, and the page shows the refreshed prices
