# Spec Delta

## MODIFIED Requirements

### Requirement: AIA policy registry

The system SHALL store one record per policy line with `label` (plan or group name, e.g. `年金 - 2024 - 2029`), optional `policy_no` (e.g. `B632611401`), optional `next_pay_date` (the next premium-due date), `premium_usd` (the sheet's `buy usd`), `value_usd` (the sheet's `now usd`) with `value_updated_at`, optional `remaining_years`, `withdrew_usd` (the sheet's `Withdrew`, cumulative), optional `note`, optional `link`, and `sort_order`.

#### Scenario: Create a policy

- **WHEN** the user submits a policy with label `年金 - 2024 - 2029`, policy number `B632611401`, premium `24960` and value `14284.35`
- **THEN** the policy is stored and returned with its stored fields

#### Scenario: Validation

- **WHEN** a policy is submitted without a label or with a negative premium, value or withdrew amount, or an invalid `next_pay_date`
- **THEN** the request is rejected with field-level errors and nothing is stored

#### Scenario: Delete a policy

- **WHEN** a policy is deleted
- **THEN** it no longer appears in the policy list or the summary totals

## ADDED Requirements

### Requirement: AIA policy endpoints

The API SHALL expose `GET /api/aia/policies`, `POST /api/aia/policies`, `PATCH /api/aia/policies/:id`, and `DELETE /api/aia/policies/:id`.

#### Scenario: Endpoints round-trip

- **WHEN** a policy is created, listed, patched, and deleted through the API
- **THEN** `GET /api/aia/policies`, `POST /api/aia/policies`, `PATCH /api/aia/policies/:id`, and `DELETE /api/aia/policies/:id` each respond

## ADDED Requirements

### Requirement: AIA policy deletion removes its events

A policy SHALL be deletable regardless of its figures; deleting it SHALL also delete its payment/withdrawal events.

#### Scenario: Delete removes events

- **WHEN** a policy carrying payment/withdrawal events is deleted through `DELETE /api/aia/policies/:id`
- **THEN** the policy and its events are gone

## MODIFIED Requirements

### Requirement: AIA summary

`GET /api/aia/summary` SHALL return every policy with its derived `balance_pct`, the stored USD rate from the fx-rates capability (`null` while unset). All figures SHALL be recomputed from stored rows on each request.

#### Scenario: Totals mirror the sheet

- **WHEN** policies hold premiums totalling `124781.98` USD, values totalling `89260.78` USD (one excluded row and one out-of-account row handled per their flags) and withdrawals of `379.07` USD
- **THEN** the summary reports those totals, a `display_value` of `87274.32` USD, and the HKD conversions of premium, value and withdrew

#### Scenario: Overall return includes withdrawals

- **WHEN** the totals are premium `124781.98`, value `89260.78`, withdrew `379.07`
- **THEN** the overall `balance_pct` is approximately `−0.2816`

#### Scenario: Next premium due

- **WHEN** two policies carry `next_pay_date` `2027-07-01` and `2028-01-15` and today is before both
- **THEN** the summary's `next_premium_due` is `2027-07-01`

#### Scenario: No rate stored

- **WHEN** no USD rate has been set
- **THEN** the summary reports USD figures only and its rate is `null`

## ADDED Requirements

### Requirement: AIA summary totals

The summary SHALL report totals derived on read: `premium`, `value`, `withdrew` and overall `balance_pct` — `(value + withdrew − premium) ÷ premium` — over non-excluded policies (the sheet's `buy usd` / `now usd` / `drew` / `balance %%` figures), `display_value` over `in_account` policies (the sheet's `AIA display value`), and each USD total converted to HKD via the stored USD rate, absent while no USD rate is stored.

#### Scenario: Summary totals derive on read

- **WHEN** policies hold premiums totalling `124781.98` USD, values totalling `89260.78` USD (one excluded row and one out-of-account row handled per their flags) and withdrawals of `379.07` USD
- **THEN** the summary reports those totals, a `display_value` of `87274.32` USD, and the HKD conversions of premium, value and withdrew

## ADDED Requirements

### Requirement: AIA next premium due

The summary SHALL also report `next_premium_due` — the earliest `next_pay_date` on or after today across all policies — since the date marks a premium the user still owes.

#### Scenario: Earliest upcoming pay date

- **WHEN** policies carry `next_pay_date` values on both sides of today
- **THEN** the summary's `next_premium_due` is the earliest one on or after today

## MODIFIED Requirements

### Requirement: Recording a premium payment

The system SHALL expose `POST /api/aia/events` accepting a policy id, `kind` `payment`, a date, a USD amount, an optional note, and an optional `next_pay_date`.

#### Scenario: Payment updates everything at once

- **WHEN** the user records a `8320` payment dated `2026-07-01` on a policy with premium `16640`, `remaining_years` `3` and `next_pay_date` `2026-07-01`
- **THEN** the policy reports premium `24960`, `remaining_years` `2` and `next_pay_date` `2027-07-01`, and the event row is stored

#### Scenario: Payment with explicit next date

- **WHEN** the user records a payment submitting `next_pay_date` `2027-01-15`
- **THEN** the policy's `next_pay_date` becomes `2027-01-15` instead of the default year-plus-one

#### Scenario: Payment on a fully-paid policy

- **WHEN** a payment is recorded on a policy whose `remaining_years` is unset
- **THEN** only `premium_usd` and `next_pay_date` change

## ADDED Requirements

### Requirement: Payment effects on the policy

Recording a payment SHALL atomically insert the event row, add the amount to the policy's `premium_usd`, decrement `remaining_years` by one when it is set, and advance the policy's `next_pay_date` — to the submitted date when given, otherwise one year after the current `next_pay_date` — collapsing the sheet's three manual edits into one action.

#### Scenario: Payment updates premium and due date

- **WHEN** the user records a `8320` payment dated `2026-07-01` on a policy with premium `16640`, `remaining_years` `3` and `next_pay_date` `2026-07-01`
- **THEN** the policy reports premium `24960`, `remaining_years` `2` and `next_pay_date` `2027-07-01`, and the event row is stored

## ADDED Requirements

### Requirement: Payment event keeps the prior values

The stored event SHALL also keep the policy's prior `next_pay_date` and `remaining_years` so the change can be reversed.

#### Scenario: Event snapshots the prior state

- **WHEN** a payment is recorded on a policy with `next_pay_date` `2026-07-01` and `remaining_years` `3`
- **THEN** the stored event keeps `2026-07-01` and `3` so the change can be reversed

## MODIFIED Requirements

### Requirement: AIA overview page

The frontend SHALL show an AIA view with a totals strip (USD premium / value / withdrew / overall return / display value plus their HKD conversions, and the next premium due date), an editable USD exchange-rate field saving through `PATCH /api/fx-rates/USD`. The view SHALL reload after every mutation.

#### Scenario: Table shows derived returns

- **WHEN** a policy has premium `38215`, value `33927.28` and no withdrawals
- **THEN** its row shows a return of approximately `−11.2%`

#### Scenario: Rate edit refreshes HKD figures

- **WHEN** the user saves a new exchange rate
- **THEN** the view reloads and the totals strip shows HKD figures computed with the new rate

#### Scenario: Record a payment from the table

- **WHEN** the user submits the 繳費 form on a policy with an amount and date
- **THEN** the view reloads, the row shows the increased premium, decremented remaining years and advanced next date, and the payment appears in the policy's history

## ADDED Requirements

### Requirement: AIA policy table and row actions

The AIA view SHALL show the policy table (label, policy number, next premium due, premium, value, withdrew, remaining years, per-row return, flags, note) with add/edit/delete actions, a 繳費 action recording a premium payment and a 提取 action recording a withdrawal on each row, and a per-policy payment/withdrawal history.

#### Scenario: Row shows the per-row return

- **WHEN** a policy has premium `38215`, value `33927.28` and no withdrawals
- **THEN** its row shows a return of approximately `−11.2%`

