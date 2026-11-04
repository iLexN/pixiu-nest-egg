# Spec Delta

## MODIFIED Requirements

### Requirement: Auto-suggested items

For the current month and later, `GET /api/months/:ym` SHALL also return candidate items derived from events dated in that month, including each 定期 starting (an `adjustment` of − its principal, only while the deposit carries a `start_date`), each 定期 ending (an `adjustment` of + its principal + interest), each HK BUY/SELL trade (an `adjustment` of −/+ its total), and each cash-moving IBKR transfer whose item is not yet stored (an `IBKR 轉入` `adjustment` of − its amount, key `ibkr:<id>`).

#### Scenario: Deposit starting suggests an adjustment

- **WHEN** a 定期 with principal `100000` and `start_date` in the current month exists
- **THEN** the month's response suggests an `adjustment` item of `−100000` for it

#### Scenario: Deposit ending suggests an adjustment

- **WHEN** a 定期 with principal `100000` and interest `1200` ends in the current month
- **THEN** the month's response suggests an `adjustment` item of `+101200` for it

#### Scenario: Accept then dismiss semantics

- **WHEN** the user accepts a suggested trade adjustment and dismisses a suggested coupon adjustment
- **THEN** the accepted item is stored and listed, and the dismissed suggestion is absent from later responses

#### Scenario: Manual-only items stay manual

- **WHEN** the user adds a yearly bonus as an item
- **THEN** it is stored as an ordinary item with no `auto_key` and behaves like any other item

#### Scenario: IBKR transfer suggested when its month row was missing

- **WHEN** a `50000` IBKR transfer was recorded for the current month before its month row existed, and the month row is then created
- **THEN** the month's response suggests an `adjustment` of `−50000` labelled `IBKR 轉入` with `auto_key` `ibkr:<id>`

#### Scenario: Log-only transfers never suggest

- **WHEN** an IBKR transfer row carries no source account
- **THEN** no candidate is produced for it

## ADDED Requirements

### Requirement: IBKR transfer month item

Recording an IBKR transfer SHALL store an `adjustment` item in the month of its `transfer_date`, labelled `IBKR 轉入`, of − its amount, with `auto_key` `ibkr:<id>`, while that month row exists; otherwise nothing is stored and the candidate rule applies. Deleting the transfer SHALL remove that item and any dismissal of its key, in whatever month they sit.

#### Scenario: Transfer records its 調整

- **WHEN** the `2026-11` month row exists and the user records a `50000` transfer dated `2026-11-03`
- **THEN** `2026-11` lists an `adjustment` item `IBKR 轉入` of `−50000` with `auto_key` `ibkr:<id>`, and 月支出 is not inflated by the transfer

#### Scenario: Deleting the transfer removes its item

- **WHEN** the user deletes that transfer
- **THEN** the `ibkr:<id>` item is gone from `2026-11`

#### Scenario: Existing manual transfer items untouched

- **WHEN** the change is deployed while `2026-10` holds the manual `to IBKR` item of `−90000`
- **THEN** that item is unchanged and no `ibkr:` item or candidate is added for the legacy transfer
