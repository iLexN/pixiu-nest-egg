# Spec Delta

## MODIFIED Requirements

### Requirement: Trade field validation
The system SHALL validate trade input before storing it: 日期 MUST be a calendar date in `YYYY-MM-DD` form, 類別 MUST be `BUY` or `SELL`, 股數 and 單價 MUST NOT be negative, and the referenced stock MUST exist in the market the trade is recorded for. On edit, a replacement `stock_id` MUST belong to the trade's recorded market — a trade's market is fixed at creation; entering it under the wrong market is corrected by deleting and recreating it.

#### Scenario: Malformed date
- **WHEN** the user submits a trade with 日期 `2026/13/45`
- **THEN** the system rejects it with a validation error naming the date field, and stores nothing

#### Scenario: Unknown stock
- **WHEN** the user submits a trade referencing a stock that is not registered
- **THEN** the system rejects it with a validation error, and stores nothing

#### Scenario: Adjustment row with zero shares
- **WHEN** the user records a trade with 股數 0 and a non-zero total together with a note
- **THEN** the trade is accepted so 加權平均買入單價 can be corrected to match the broker's displayed value — it adjusts 總買入成本 without changing Σ BUY 股數 and moves no cash

#### Scenario: Repointing a trade to the other market
- **WHEN** the user edits a US trade's `stock_id` to an HK stock, or an HK trade's to a US stock
- **THEN** the request is rejected with a field error naming `stock_id`, and nothing changes

#### Scenario: Same-market repoint allowed
- **WHEN** the user edits a US trade's `stock_id` to another US stock
- **THEN** the trade is repointed and its figures are re-derived as usual

## ADDED Requirements

### Requirement: US trade cash settlement
Recording a US trade with 股數 > 0 SHALL adjust the USD `broker` cash account by the trade's signed total — BUY subtracts the total, SELL adds it — in the same write as the trade itself. US trades with 股數 0 (price-adjustment rows) and all HK trades SHALL move no broker cash. Settlement SHALL record the applied signed amount on the trade so it can be reversed exactly; a trade carrying no recorded amount moved no cash. US trades SHALL record no month item — the money never touches 活期.

#### Scenario: US buy debits USD cash
- **WHEN** the user records a US `BUY` of 3 shares of `VOO` with total `2089.120009` while USD 現金 is `5000`
- **THEN** the trade is stored and the USD broker account shows `2910.879991`

#### Scenario: US sell credits USD cash
- **WHEN** the user records a US `SELL` with total `1200` while USD 現金 is `500`
- **THEN** the trade is stored and the USD broker account shows `1700`

#### Scenario: HK trade moves no broker cash
- **WHEN** the user records a HK `BUY` with total `96523.15`
- **THEN** the trade is stored and both broker cash accounts are unchanged — the HK 活期 suggestion rules apply instead

#### Scenario: Zero-share adjustment row moves no cash
- **WHEN** the user records a US trade with 股數 `0`, total `50`, and a note
- **THEN** the trade is stored, carries no recorded settlement, and USD 現金 is unchanged

#### Scenario: Negative USD cash allowed
- **WHEN** the user records a US `BUY` with total `5000` while USD 現金 is `1200`
- **THEN** the trade is stored and the USD broker account shows `-3800` — margin positions are legitimate and the manual PATCH remains the reconcile path

### Requirement: Settlement reversal on edit and delete
Editing a settled US trade SHALL reverse its recorded signed amount from the USD broker account and apply the new signed amount, recording it in place of the old; a settled row edited to 股數 0 SHALL unwind its settlement and record none. Deleting a settled US trade SHALL reverse its recorded amount. A trade with no recorded settlement SHALL move no cash on edit or delete — including every trade recorded before settlement existed, whatever its market or shares.

#### Scenario: Edit re-applies the delta
- **WHEN** a settled US `BUY` recorded `-2089.12` is edited so its total becomes `2090.12`
- **THEN** the USD broker account moves by `-1.00` net and the trade records `-2090.12`

#### Scenario: Delete returns the cash
- **WHEN** a settled US `BUY` that recorded `-2089.12` is deleted
- **THEN** the USD broker account gains `2089.12`

#### Scenario: Pre-existing trade stays inert
- **WHEN** a US trade recorded before settlement existed (no recorded amount) is edited or deleted
- **THEN** no broker cash moves — the manually reconciled balance already reflected it

#### Scenario: Settled trade edited to zero shares unwinds
- **WHEN** a settled US trade is edited down to 股數 `0`
- **THEN** its recorded amount is reversed from the USD broker account and it carries no settlement afterwards
