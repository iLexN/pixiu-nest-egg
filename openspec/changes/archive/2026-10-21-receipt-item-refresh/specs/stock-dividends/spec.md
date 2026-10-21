# Spec Delta

## MODIFIED Requirements

### Requirement: Dividend receipt month item

For HK dividends, the same transaction SHALL record the `div:<id>` `adjustment` item for the pay_date month. An already-stored keyed item is updated to the `received_amount`; the write is skipped only when the month has no row. US dividends never touch 活期, so they record none.

#### Scenario: HK receipt records the month item

- **WHEN** the user marks a pending HK dividend received and the pay_date month row exists without the item
- **THEN** the month gains a `div:<id>` `adjustment` item, while a US dividend records none

#### Scenario: Receipt refreshes a stored item

- **WHEN** the pay month already stores the `div:<id>` item at the estimated amount and the dividend is 收訖 at a different `received_amount`
- **THEN** the stored item's amount becomes the `received_amount`
