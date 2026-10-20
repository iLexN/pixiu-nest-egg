# Spec Delta

## MODIFIED Requirements

### Requirement: Coupon receipt lifecycle

A coupon's `status` SHALL be derived on read: `received` once `received_amount` is set; `待定` while `annual_rate`/`per_10k` are unset; otherwise `pending`. `variance` SHALL be derived as `received_amount − expected` when both exist.

Entering the announced rate SHALL be a `PATCH` that sets `annual_rate`/`per_10k`. Recording receipt SHALL be a `PATCH` that sets `received_amount`.

Setting `received_amount` where it was previously NULL SHALL do two things, atomically with the update:

- credit the amount under the cash-accounts receipt crediting rule, with receipt currency `HKD`. The request MAY carry `bank_in` (default true) and an `account_id` override.
- record the `coupon:<id>` `adjustment` item for the pay_date month. The item is skipped when already stored or when the month has no row.

Clearing `received_amount` SHALL reverse the recorded credit on the account it went to and delete the item. A coupon SHALL be editable and deletable regardless of status.

#### Scenario: Rate fixing moves coupon to pending

- **WHEN** the user patches a 待定 coupon with `annual_rate` `0.04` and `per_10k` `200.55`
- **THEN** the coupon's status becomes `pending` and its `expected` is `1002.75` for a `50000` principal

#### Scenario: Receipt recorded

- **WHEN** the user patches a pending coupon with `received_amount` `1002.75`
- **THEN** its status becomes `received` and `variance` is `0`

#### Scenario: Received amount differing from expected

- **WHEN** a coupon with `expected` `997.25` is marked received with `received_amount` `1000`
- **THEN** its `variance` reports `2.75`

#### Scenario: Receipt banks into HS and records the item

- **WHEN** HS is the HKD default and a pending coupon paying `2026-10-23` is 收訖 with `received_amount` `1000`
- **THEN** HS gains `1000`, the `2026-10` month gains a `coupon:<id>` adjustment item of `1000`, and October's derived 利息 includes `1000` on the next read

#### Scenario: Clearing the receipt reverses

- **WHEN** the user clears `received_amount` on a coupon that had credited HS `1000`
- **THEN** HS loses `1000` and the `coupon:<id>` item is deleted

### Requirement: Bond principal 收訖

`POST /api/bonds/:id/receive` SHALL mark the bond's principal received at a date (default today) and record the maturity month's `bond-end:<id>` `adjustment` item.

The receipt SHALL credit an amount, defaulting to `principal`, under the cash-accounts receipt crediting rule, with receipt currency `HKD`. The request carries `bank_in` (default true), an optional `account_id` override, and an optional `credit_amount`.

Re-receiving SHALL return 409. Receipt SHALL NOT be reversible. The matured-bonds section SHALL flag an unreceived bond 本金未收 and offer 收訖; a received one shows its 收訖日.

#### Scenario: One-click principal return

- **WHEN** the user 收訖's a matured bond (principal `100000`, maturity `2026-10-25`) choosing cash account `HS`
- **THEN** the bond is stored received, `HS` gains `100000`, and the `2026-10` month gains a `bond-end` adjustment item of `100000`

#### Scenario: Principal receipt without a bank-in

- **WHEN** the user 收訖's a matured bond with `不存入` selected (`bank_in` false)
- **THEN** only `received_at` and the `bond-end` item are recorded — no cash account changes

#### Scenario: bond-end item skipped when already stored or monthless

- **WHEN** the maturity month already stores the `bond-end:<id>` item or has no month row
- **THEN** the credit and flag apply but no item is recorded
