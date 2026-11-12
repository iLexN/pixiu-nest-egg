# Spec Delta

## MODIFIED Requirements

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

### Requirement: Bond receipt finality and matured display

Re-receiving SHALL return 409. Receipt SHALL NOT be reversible. The matured-bonds section SHALL list only 收訖'd bonds, each showing its 收訖日.

#### Scenario: Matured section flags the unreceived

- **WHEN** one bond is 收訖'd and another matured bond is not
- **THEN** the matured-bonds section lists only the 收訖'd one with its 收訖日; the unreceived one is not in it — it sits in the active section flagged 已到期未收

#### Scenario: Re-receive conflicts

- **WHEN** a second receive call targets a 收訖'd bond
- **THEN** it returns 409 and nothing changes
