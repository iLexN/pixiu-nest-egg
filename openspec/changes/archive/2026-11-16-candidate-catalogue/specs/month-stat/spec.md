# Spec Delta

## ADDED Requirements

### Requirement: Bond maturity candidate

For the current month and later, the candidates SHALL also include each bond maturing in the month: an `adjustment` of + its principal, or + its credited amount once 收訖 credited one, labelled with the bond's label, with key `bond-end:<id>` and source `bond`.

#### Scenario: Bond maturing suggests an adjustment

- **WHEN** an unreceived bond with principal `100000` matures in the current month and the month stores no `bond-end:<id>` item
- **THEN** the month's response suggests an `adjustment` of `+100000` labelled with the bond's label, with `auto_key` `bond-end:<id>`

#### Scenario: Bond received before its month row existed

- **WHEN** a bond maturing in the current month was 收訖 with a credited amount of `99500` before the month row existed, and the month row is then created
- **THEN** the month's response suggests a `bond-end:<id>` `adjustment` of `+99500`

### Requirement: Deposit candidate label

A `dep-start` or `dep-end` candidate SHALL be labelled with the deposit's label, or `定期` when the deposit has none.

#### Scenario: Unlabeled deposit candidate is labelled 定期

- **WHEN** a 定期 without a label starts in the current month
- **THEN** its `dep-start:<id>` candidate carries the label `定期`, and accepting it stores an item labelled `定期`

### Requirement: Recorded candidates match their suggestion

When a write (deposit placement, deposit/bond/coupon/HK dividend 收訖, or IBKR transfer) records a keyed month item, that item SHALL carry the same `auto_key`, month, category and label the candidate would have shown, and an amount equal to the amount received at 收訖 (or the event's own amount for placements and transfers).

#### Scenario: Pending suggestion and recorded item agree

- **WHEN** a pending HK dividend paying in the current month is suggested as `div:<id>` and is then 收訖 at `909.83`
- **THEN** the stored item has `auto_key` `div:<id>`, the same month, category and label as the suggestion, and amount `909.83`

#### Scenario: Unlabeled deposit records 定期

- **WHEN** a deposit without a label is created with a `start_date` whose month row exists
- **THEN** its stored `dep-start:<id>` item is labelled `定期`

### Requirement: Retiring a candidate item

Whenever a write removes a keyed month item (deleting a deposit or IBKR transfer, or clearing a coupon's or dividend's `received_amount`), it SHALL remove that key's item in whichever month it sits, plus every dismissal recorded for that key.

#### Scenario: Item removed after the source's date moved

- **WHEN** a coupon was 收訖 with its `coupon:<id>` item stored in `2026-10`, and one update moves its `pay_date` to `2026-11` and clears `received_amount`
- **THEN** no `coupon:<id>` item remains in any month

#### Scenario: Dismissals go with the item

- **WHEN** a key has a dismissal in one month and its source is deleted or its receipt cleared
- **THEN** neither the item nor any dismissal for that key remains
