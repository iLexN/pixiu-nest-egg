# Spec Delta

## ADDED Requirements

### Requirement: Deposit creation month item

`POST /api/deposits` SHALL record the `dep-start:<id>` `adjustment` item of −`principal` in the `start_date` month, in the same write as the deposit itself — as the dep-start suggestion would have. The item is skipped when the deposit carries no `start_date`, when the start month already stores its `dep-start:<id>` item, or when the start month has no month row; a skipped item still surfaces as the month's `dep-start` suggestion once the month exists.

#### Scenario: Create records the start month item

- **WHEN** the user creates a deposit with principal `140000` and `start_date` `2026-10-13`, and the `2026-10` month row exists
- **THEN** the deposit is stored and the `2026-10` month holds a `dep-start:<id>` adjustment item of `-140000`

#### Scenario: No start_date, no item

- **WHEN** the user creates a deposit with no `start_date`
- **THEN** no `dep-start` item is recorded for it

#### Scenario: dep-start item skipped when already stored or monthless

- **WHEN** the start month already stores the `dep-start:<id>` item or has no month row
- **THEN** the deposit is stored but no item is recorded, and the `dep-start:<id>` suggestion still appears for that month
