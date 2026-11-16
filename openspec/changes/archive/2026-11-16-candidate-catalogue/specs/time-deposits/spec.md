# Spec Delta

## MODIFIED Requirements

### Requirement: Deposit creation month item

`POST /api/deposits` SHALL record the `dep-start:<id>` `adjustment` item of −`principal` in the `start_date` month, in the same write as the deposit itself — as the dep-start suggestion would have, labelled with the deposit's label or `定期` when it has none. The item is skipped when the deposit carries no `start_date` or when the start month has no month row; a skipped item still surfaces as the month's `dep-start` suggestion once the month exists.

#### Scenario: Create records the start month item

- **WHEN** the user creates a deposit with principal `140000` and `start_date` `2026-10-13`, and the `2026-10` month row exists
- **THEN** the deposit is stored and the `2026-10` month holds a `dep-start:<id>` adjustment item of `-140000`

#### Scenario: No start_date, no item

- **WHEN** the user creates a deposit with no `start_date`
- **THEN** no `dep-start` item is recorded for it

#### Scenario: dep-start item skipped when already stored or monthless

- **WHEN** the start month has no month row
- **THEN** the deposit is stored but no item is recorded, and the `dep-start:<id>` suggestion still appears for that month

#### Scenario: Debit and item in one write

- **WHEN** the user creates a deposit with principal `140000`, `start_date` `2026-10-13`, and `debit_account_id` naming HS, and the `2026-10` month row exists
- **THEN** the deposit, the HS debit of `140000`, and the `dep-start:<id>` item of `-140000` are all stored together

### Requirement: Deposit receipt month item and finality

The receive endpoint also records the month's `dep-end:<id>` `adjustment` item for that amount, labelled with the deposit's label or `定期` when it has none. Re-receiving SHALL return 409. Receipt SHALL NOT be reversible: there is no unreceive endpoint or action.

#### Scenario: Receive is final

- **WHEN** a deposit has been received
- **THEN** a second receive call returns 409 and no unreceive action exists
