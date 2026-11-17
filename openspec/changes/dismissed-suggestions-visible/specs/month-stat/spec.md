# Spec Delta

## MODIFIED Requirements

### Requirement: Auto-suggested items

For every month, `GET /api/months/:ym` SHALL also return candidate items derived from events dated in that month, including each 定期 starting (an `adjustment` of − its principal, only while the deposit carries a `start_date`), each 定期 ending (an `adjustment` of + its principal + interest), each HK BUY/SELL trade (an `adjustment` of −/+ its total), and each cash-moving IBKR transfer whose item is not yet stored (an `IBKR 轉入` `adjustment` of − its amount, key `ibkr:<id>`). The response SHALL flag whether the month is actionable — the previous month and later; candidates in earlier months are informational and offer no actions.

#### Scenario: Deposit starting suggests an adjustment

- **WHEN** a 定期 with principal `100000` and `start_date` in the current month exists
- **THEN** the month's response suggests an `adjustment` item of `−100000` for it

#### Scenario: Deposit ending suggests an adjustment

- **WHEN** a 定期 with principal `100000` and interest `1200` ends in the current month
- **THEN** the month's response suggests an `adjustment` item of `+101200` for it

#### Scenario: Accept then dismiss semantics

- **WHEN** the user accepts a suggested trade adjustment and dismisses a suggested coupon adjustment
- **THEN** the accepted item is stored and listed, and the dismissed suggestion stays in later responses flagged `dismissed`

#### Scenario: History month lists candidates read-only

- **WHEN** a stored month before the previous month has a pending or dismissed candidate
- **THEN** the month's response returns it and marks the month non-actionable

#### Scenario: Manual-only items stay manual

- **WHEN** the user adds a yearly bonus as an item
- **THEN** it is stored as an ordinary item with no `auto_key` and behaves like any other item

#### Scenario: IBKR transfer suggested when its month row was missing

- **WHEN** a `50000` IBKR transfer was recorded for the current month before its month row existed, and the month row is then created
- **THEN** the month's response suggests an `adjustment` of `−50000` labelled `IBKR 轉入` with `auto_key` `ibkr:<id>`

#### Scenario: Log-only transfers never suggest

- **WHEN** an IBKR transfer row carries no source account
- **THEN** no candidate is produced for it

### Requirement: Candidate accept and dismiss

Each candidate SHALL carry a stable `auto_key`; accepting a candidate stores it as an item carrying that key, and dismissing one SHALL record the key while the candidate stays in the month read flagged `dismissed` — an ignored, never-counted row. Undismissing SHALL remove the record so the candidate is pending again. Candidates SHALL NOT be stored until accepted.

#### Scenario: Dismissed candidate never returns

- **WHEN** the user dismisses a candidate
- **THEN** its `auto_key` is recorded and later responses return it flagged `dismissed`, never again as a pending suggestion

#### Scenario: Dismissed candidate stays listed

- **WHEN** the user dismisses a candidate
- **THEN** its `auto_key` is recorded and later responses return the candidate flagged `dismissed`

#### Scenario: Undismiss restores the pending candidate

- **WHEN** the user undismisses a recorded `auto_key`
- **THEN** the record is removed and the candidate returns pending again

#### Scenario: Accept clears the dismissal

- **WHEN** the user accepts a dismissed candidate
- **THEN** the item is stored carrying the key and the dismissal record is gone

### Requirement: Bond maturity candidate

For every month, the candidates SHALL also include each bond maturing in the month: an `adjustment` of + its principal, or + its credited amount once 收訖 credited one, labelled with the bond's label, with key `bond-end:<id>` and source `bond`.

#### Scenario: Bond maturing suggests an adjustment

- **WHEN** an unreceived bond with principal `100000` matures in the current month and the month stores no `bond-end:<id>` item
- **THEN** the month's response suggests an `adjustment` of `+100000` labelled with the bond's label, with `auto_key` `bond-end:<id>`

#### Scenario: Bond received before its month row existed

- **WHEN** a bond maturing in the current month was 收訖 with a credited amount of `99500` before the month row existed, and the month row is then created
- **THEN** the month's response suggests a `bond-end:<id>` `adjustment` of `+99500`

### Requirement: 月結 item editor and settings card

The view SHALL also show an item editor per month listing stored items and auto-suggestions — pending ones first with accept/dismiss actions, dismissed ones after them marked `忽略（不計）` with accept/undismiss actions — plus manual add (娛樂 items offering a 不計入生活支出 flag, 利息 among the categories), and a settings card for cash/asset balances, salary and the current pool rate. For months before the previous month the suggestion rows SHALL show no actions.

#### Scenario: Suggestions offer accept and dismiss

- **WHEN** an actionable month has a pending auto-suggestion
- **THEN** the item editor offers accept and dismiss actions on it

#### Scenario: Dismissed suggestion stays visible

- **WHEN** the user dismisses a suggestion
- **THEN** the row remains listed marked `忽略（不計）` and offers accept and undismiss actions

#### Scenario: Old month shows read-only suggestions

- **WHEN** the user opens a month before the previous month that has suggestions
- **THEN** the rows show with no actions
