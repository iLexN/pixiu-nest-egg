## ADDED Requirements

### Requirement: Hidden stocks excluded from the summary table
The 持倉總覽 table SHALL omit stocks marked hidden (inactive), while the summary response still includes them and they still count toward 總買入成本, 累計派息, 淨投入總本金, market totals and sector rollups — hiding is display-only. The view SHALL indicate how many stocks are hidden so the user knows rows are filtered. Manual stock order SHALL continue to cover hidden stocks, which keep their saved position until unhidden.

#### Scenario: Hidden stock not shown
- **WHEN** `香港寬頻` is hidden and the user opens 持倉總覽
- **THEN** its row is not shown, and a note reports that 1 stock is hidden

#### Scenario: Hidden stock still counted
- **WHEN** a hidden stock has 總買入成本 or received 派息
- **THEN** those figures still contribute to the market totals and its sector's rollup

#### Scenario: Reordering with a hidden stock
- **WHEN** the user drags a visible row while a hidden stock exists
- **THEN** the saved order still covers every stock in the market and the hidden stock keeps its position
