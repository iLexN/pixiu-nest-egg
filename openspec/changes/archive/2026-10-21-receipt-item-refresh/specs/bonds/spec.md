# Spec Delta

## MODIFIED Requirements

### Requirement: Coupon receipt side effects

Setting `received_amount` where it was previously NULL SHALL do two things, atomically with the update:

- credit the amount under the cash-accounts receipt crediting rule, with receipt currency `HKD`. The request MAY carry `bank_in` (default true) and an `account_id` override.
- record the `coupon:<id>` `adjustment` item for the pay_date month. An already-stored keyed item is updated to the receipt amount; the write is skipped only when the month has no row.

#### Scenario: Receipt credits and records

- **WHEN** the user marks a pending coupon received with `received_amount` `1002.75` and no `account_id`
- **THEN** the HKD default receipt cash account gains `1002.75` and the pay_date month gains a `coupon:<id>` `adjustment` item

#### Scenario: Receipt refreshes a stored item

- **WHEN** the pay month already stores the `coupon:<id>` item at the expected `1002.75` and the coupon is 收訖 at `received_amount` `1000`
- **THEN** the cash account gains `1000` and the stored item's amount becomes `1000`

### Requirement: Bond principal 收訖

`POST /api/bonds/:id/receive` SHALL mark the bond's principal received at a date (default today) and record the maturity month's `bond-end:<id>` `adjustment` item. An already-stored keyed item is updated to the credited amount; the write is skipped only when the month has no row.

The receipt SHALL credit an amount, defaulting to `principal`, under the cash-accounts receipt crediting rule, with receipt currency `HKD`. The request carries `bank_in` (default true), an optional `account_id` override, and an optional `credit_amount`.

#### Scenario: One-click principal return

- **WHEN** the user 收訖's a matured bond (principal `100000`, maturity `2026-10-25`) choosing cash account `HS`
- **THEN** the bond is stored received, `HS` gains `100000`, and the `2026-10` month gains a `bond-end` adjustment item of `100000`

#### Scenario: Principal receipt without a bank-in

- **WHEN** the user 收訖's a matured bond with `不存入` selected (`bank_in` false)
- **THEN** only `received_at` and the `bond-end` item are recorded — no cash account changes

#### Scenario: bond-end item skipped when already stored or monthless

- **WHEN** the maturity month already stores the `bond-end:<id>` item or has no month row
- **THEN** the credit and flag apply; the stored item is updated to the credited amount, or none is recorded when the month has no row
