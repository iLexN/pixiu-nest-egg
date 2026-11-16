# Spec Delta

## MODIFIED Requirements

### Requirement: Coupon receipt reversal and editability

Clearing `received_amount` SHALL reverse the recorded credit on the account it went to and delete the `coupon:<id>` item in whichever month it sits, together with any dismissal of that key. A coupon SHALL be editable and deletable regardless of status.

#### Scenario: Clearing reverses the credit

- **WHEN** the user clears `received_amount` on a received coupon
- **THEN** the cash account that was credited loses the amount and the `coupon:<id>` item is deleted

#### Scenario: Clearing after the pay date moved

- **WHEN** a coupon received with its `coupon:<id>` item in `2026-10` is updated in one request to `pay_date` `2026-11-15` with `received_amount` cleared
- **THEN** the credit is reversed and no `coupon:<id>` item remains in `2026-10` or `2026-11`

### Requirement: Bond principal 收訖

`POST /api/bonds/:id/receive` SHALL mark the bond's principal received at a date (default today) and record the maturity month's `bond-end:<id>` `adjustment` item at the credited amount (`credit_amount`, defaulting to `principal`), or at `principal` when `bank_in` is false and nothing is credited. An already-stored keyed item is updated to the credited amount; the write is skipped only when the month has no row.

#### Scenario: bond-end item skipped when already stored or monthless

- **WHEN** the maturity month already stores the `bond-end:<id>` item or has no month row
- **THEN** the credit and flag apply; the stored item is updated to the credited amount, or none is recorded when the month has no row

#### Scenario: Credited amount differs from the principal

- **WHEN** the user 收訖's a bond with principal `100000` supplying `credit_amount` `99500`, and its maturity month row exists
- **THEN** the cash account gains `99500` and the `bond-end:<id>` item is `99500`
