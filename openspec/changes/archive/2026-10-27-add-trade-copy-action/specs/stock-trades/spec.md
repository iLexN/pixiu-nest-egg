# Spec Delta

## ADDED Requirements

### Requirement: Copying a trade summary
Each row in the trade history SHALL offer a 複製 action, listed before 編輯, that copies a four-line plain-text summary of that trade to the clipboard. Each line SHALL be a label ending in `：`, a tab, then the value: the stock code, 股數, `$` followed by the entered 單價, and 日期. Values SHALL be the stored values unrounded and without thousands separators. BUY rows SHALL use 購買/買入 labels and SELL rows 賣出 labels.

#### Scenario: Copying a HK buy
- **WHEN** the user chooses 複製 on the BUY trade of 1100 shares of `新華保險` at 單價 47.02 dated 2026-06-04 (平均單價 含 fee 47.216)
- **THEN** the clipboard holds exactly these four lines, each label followed by a tab:
  `資產名稱/代號：	新華保險`
  `購買股數：	1100`
  `買入價格：	$47.02`
  `買入日期：	2026-06-04`

#### Scenario: Copying a sell
- **WHEN** the user chooses 複製 on a SELL trade of 0.5 shares of `VOO` at 單價 696.04 dated 2026-06-02
- **THEN** the clipboard holds `資產名稱/代號：	VOO`, `賣出股數：	0.5`, `賣出價格：	$696.04`, and `賣出日期：	2026-06-02` on four lines

#### Scenario: Price is not padded or rounded
- **WHEN** the user copies a trade whose stored 單價 is 3.2
- **THEN** the price line reads `$3.2`

#### Scenario: Copy feedback
- **WHEN** the clipboard write succeeds
- **THEN** the row briefly shows that the trade was copied, and if the write fails a visible 複製失敗 message is shown instead
