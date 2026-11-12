# bonds Specification

## Purpose

Tracks the user's bond holdings (債券) — each bond's principal and maturity — together with its coupon schedule, so the app replaces the workbook's 債券 sheet and, unlike the sheet, keeps matured bonds and their coupon records as history.

## Requirements

### Requirement: Bond registry

The system SHALL store one record per bond with `label`, `issue_no`, `principal`, `maturity_date`, optional `note`, and `sort_order`. The API SHALL expose `GET /api/bonds` (optional `status=active|matured` filter), `POST /api/bonds`, `PATCH /api/bonds/:id`, and `DELETE /api/bonds/:id`. A bond's `status` SHALL be derived on read: active while its principal is not 收訖 (`received_at` unset), matured once 收訖 — whatever `maturity_date` is, the same rule as deposits. `status` SHALL NOT be stored.

#### Scenario: Create a bond

- **WHEN** the user submits a bond with label `silver bond`, issue number `03GB2710R`, principal `50000` and maturity `2027-10-23`
- **THEN** the bond is stored and returned with status `active`

#### Scenario: Maturity alone keeps the bond active

- **WHEN** a bond's `maturity_date` passes and its principal is not 收訖
- **THEN** it is still reported as `active`, and the `status=active` filter returns it

#### Scenario: Maturity flips status

- **WHEN** a bond reaches the end of its life by having its principal 收訖
- **THEN** it is reported as `matured` without any other stored field changing, and the `status=matured` filter returns it

#### Scenario: Early 收訖 ends activity

- **WHEN** a bond whose `maturity_date` is still in the future is 收訖
- **THEN** it is reported as `matured`

#### Scenario: Validation

- **WHEN** a bond is submitted without a label or with a non-positive principal or an invalid maturity date
- **THEN** the request is rejected with field-level errors and nothing is stored

### Requirement: Deleting a bond removes its coupons

`DELETE /api/bonds/:id` SHALL remove the bond and all of its coupon records. There is no dependent data outside the bond's own coupons, so deletion is always permitted.

#### Scenario: Delete cascades

- **WHEN** a matured bond with six coupon records is deleted
- **THEN** the bond and all six coupons are gone and the API returns them no more

### Requirement: Coupon schedule per bond

The system SHALL store one record per scheduled coupon with `bond_id`, `pay_date`, optional `fixing_date`, `annual_rate`, `per_10k`, `received_amount`, and `note`. `annual_rate` and `per_10k` SHALL be nullable — a `NULL` rate represents the sheet's 待定 (not yet fixed). The API SHALL expose `GET/POST /api/coupons` (list filterable by `bond_id`) and `PATCH/DELETE /api/coupons/:id`.

#### Scenario: Fixed coupon derives expected amount

- **WHEN** a coupon of a `50000`-principal bond has `per_10k` `199.45`
- **THEN** its derived `expected` is `997.25`

#### Scenario: Unfixed coupon shows 待定

- **WHEN** a coupon's `annual_rate` and `per_10k` are unset
- **THEN** it is reported with status `待定` and no expected amount

### Requirement: Coupon expected amount

Coupon `expected` SHALL be derived on read as `per_10k × bond principal ÷ 10000`, reproducing the sheet's interest column; it SHALL be absent while `per_10k` is unset.

#### Scenario: Expected amount from per-10k

- **WHEN** a coupon of a `50000`-principal bond has `per_10k` `199.45`
- **THEN** its derived `expected` is `997.25`

### Requirement: Coupon receipt lifecycle

A coupon's `status` SHALL be derived on read: `received` once `received_amount` is set; `待定` while `annual_rate`/`per_10k` are unset; otherwise `pending`. `variance` SHALL be derived as `received_amount − expected` when both exist.

Entering the announced rate SHALL be a `PATCH` that sets `annual_rate`/`per_10k`. Recording receipt SHALL be a `PATCH` that sets `received_amount`.

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

### Requirement: Coupon receipt reversal and editability

Clearing `received_amount` SHALL reverse the recorded credit on the account it went to and delete the item. A coupon SHALL be editable and deletable regardless of status.

#### Scenario: Clearing reverses the credit

- **WHEN** the user clears `received_amount` on a received coupon
- **THEN** the cash account that was credited loses the amount and the `coupon:<id>` item is deleted

### Requirement: Bond summary view

`GET /api/bonds/summary` SHALL return the date used for derivation, the active bonds (each with its next unpaid coupon date), the matured bonds, the active principal total (the sheet's `Total` figure), and the upcoming coupons across active bonds ordered by `pay_date`. All figures SHALL be recomputed from stored rows on each request. A matured-but-unreceived bond SHALL be an active bond for every one of these figures.

#### Scenario: Active totals

- **WHEN** one active bond of principal `50000` and one 收訖'd bond of `30000` exist
- **THEN** the summary's active principal total is `50000` and the 收訖'd bond appears only in the matured list

#### Scenario: Unreceived matured bond keeps counting

- **WHEN** a bond of principal `50000` matured `2026-11-10`, today is `2026-11-12`, and its principal is not 收訖
- **THEN** it appears in the active list, the active principal total includes `50000`, and Overview 債券 and live 總數/流動資產 are unchanged by the maturity

#### Scenario: 收訖 removes the bond from the totals

- **WHEN** the user 收訖's that bond choosing cash account `HS`
- **THEN** the active principal total drops by `50000`, `HS` rises by `50000`, and live 流動資產 is unchanged by the receipt

#### Scenario: Upcoming coupons

- **WHEN** an active bond has coupons dated before and after today
- **THEN** only the unpaid coupons appear in upcoming coupons, earliest `pay_date` first

### Requirement: 債券 overview page

The frontend SHALL show a 債券 view with the active principal total, one section per active bond listing its coupons with their status (待定 / pending / received), inline entry of the announced rate, marking a coupon received, and a matured-bonds history section below. An active bond whose `maturity_date` is on or before the summary's derivation date SHALL be flagged 已到期未收 and offer its principal 收訖 in the active section. The view SHALL reload after every mutation.

#### Scenario: Coupon table states

- **WHEN** the silver bond has three fixed and three unfixed coupons
- **THEN** the table shows the fixed coupons' expected amounts and marks the unfixed ones 待定

#### Scenario: Mark received from the table

- **WHEN** the user marks a pending coupon received
- **THEN** the view reloads and the coupon displays as received with its variance

#### Scenario: Overdue bond flagged in the active section

- **WHEN** an active bond's `maturity_date` is today or past
- **THEN** its active section shows a 已到期未收 marker and a 收訖 action

### Requirement: Bond principal 收訖 flag

Each bond SHALL carry a `received_at` date (NULL while the principal is still on the books), separate from the per-coupon receipt lifecycle. Create SHALL record `received_at = maturity_date` when the entered maturity is already past. The active/matured split and `債券!B1` active principal SHALL follow `received_at`, not `maturity_date`. Editing `maturity_date` SHALL NOT change `received_at`.

#### Scenario: Past maturity auto-marks on create

- **WHEN** a bond is created with `maturity_date` already past
- **THEN** it is stored received with `received_at = maturity_date`

#### Scenario: Editing maturity into the past does not receive

- **WHEN** an unreceived bond's `maturity_date` is patched to a past date
- **THEN** `received_at` stays NULL and the bond stays active, flagged 已到期未收

### Requirement: Bond principal 收訖

`POST /api/bonds/:id/receive` SHALL mark the bond's principal received at a date (default today) and record the maturity month's `bond-end:<id>` `adjustment` item. An already-stored keyed item is updated to the credited amount; the write is skipped only when the month has no row.

#### Scenario: bond-end item skipped when already stored or monthless

- **WHEN** the maturity month already stores the `bond-end:<id>` item or has no month row
- **THEN** the credit and flag apply; the stored item is updated to the credited amount, or none is recorded when the month has no row

### Requirement: Bond principal receipt crediting

The receipt SHALL credit an amount, defaulting to `principal`, under the cash-accounts receipt crediting rule, with receipt currency `HKD`. The request carries `bank_in` (default true), an optional `account_id` override, and an optional `credit_amount`.

#### Scenario: One-click principal return

- **WHEN** the user 收訖's a matured bond (principal `100000`, maturity `2026-10-25`) choosing cash account `HS`
- **THEN** the bond is stored received, `HS` gains `100000`, and the `2026-10` month gains a `bond-end` adjustment item of `100000`

#### Scenario: Principal receipt without a bank-in

- **WHEN** the user 收訖's a matured bond with `不存入` selected (`bank_in` false)
- **THEN** only `received_at` and the `bond-end` item are recorded — no cash account changes

### Requirement: Bond receipt finality and matured display

Re-receiving SHALL return 409. Receipt SHALL NOT be reversible. The matured-bonds section SHALL list only 收訖'd bonds, each showing its 收訖日.

#### Scenario: Matured section flags the unreceived

- **WHEN** one bond is 收訖'd and another matured bond is not
- **THEN** the matured-bonds section lists only the 收訖'd one with its 收訖日; the unreceived one is not in it — it sits in the active section flagged 已到期未收

#### Scenario: Re-receive conflicts

- **WHEN** a second receive call targets a 收訖'd bond
- **THEN** it returns 409 and nothing changes
