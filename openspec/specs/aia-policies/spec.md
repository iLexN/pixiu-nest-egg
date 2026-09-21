# aia-policies Specification

## Purpose

Tracks the user's AIA insurance and annuity policies — premium paid in USD, current value, cumulative withdrawals, remaining payment years and remarks — deriving per-policy and overall returns plus HKD conversions, so the app replaces the workbook's `AIA` sheet.

## Requirements

### Requirement: AIA policy registry

The system SHALL store one record per policy line with `label` (plan or group name, e.g. `年金 - 2024 - 2029`), optional `policy_no` (e.g. `B632611401`), optional `next_pay_date` (the next premium-due date), `premium_usd` (the sheet's `buy usd`), `value_usd` (the sheet's `now usd`) with `value_updated_at`, optional `remaining_years`, `withdrew_usd` (the sheet's `Withdrew`, cumulative), optional `note`, optional `link`, and `sort_order`. The API SHALL expose `GET /api/aia/policies`, `POST /api/aia/policies`, `PATCH /api/aia/policies/:id`, and `DELETE /api/aia/policies/:id`. A policy SHALL be deletable regardless of its figures; deleting it SHALL also delete its payment/withdrawal events.

#### Scenario: Create a policy

- **WHEN** the user submits a policy with label `年金 - 2024 - 2029`, policy number `B632611401`, premium `24960` and value `14284.35`
- **THEN** the policy is stored and returned with its stored fields

#### Scenario: Validation

- **WHEN** a policy is submitted without a label or with a negative premium, value or withdrew amount, or an invalid `next_pay_date`
- **THEN** the request is rejected with field-level errors and nothing is stored

#### Scenario: Delete a policy

- **WHEN** a policy is deleted
- **THEN** it no longer appears in the policy list or the summary totals

### Requirement: Updating the current value

`PATCH /api/aia/policies/:id` SHALL update `value_usd` and refresh `value_updated_at` whenever the value changes, the same convention as a stock's manual price. Other fields SHALL be editable through the same endpoint. There is no value history — the sheet stores a single current figure per policy.

#### Scenario: Value update records its time

- **WHEN** the user patches a policy's `value_usd` to `15000`
- **THEN** the policy reports `value_usd` `15000` and a `value_updated_at` of the update time

### Requirement: Derived per-policy return

Each policy's `balance_pct` SHALL be derived on read as `(value_usd + withdrew_usd − premium_usd) ÷ premium_usd`, reproducing the sheet's `balance %%` column; it SHALL be absent when `premium_usd` is zero. The figure SHALL NOT be stored.

#### Scenario: Return includes withdrawals

- **WHEN** a policy has premium `6000`, value `7845.67` and withdrew `127.86`
- **THEN** its derived `balance_pct` is approximately `0.3289`

#### Scenario: Zero premium yields no return figure

- **WHEN** a policy has `premium_usd` `0`
- **THEN** no `balance_pct` is reported for it

### Requirement: Excluded and in-account flags

Each policy SHALL carry two boolean flags: `excluded` (default false — the row is not counted in the portfolio totals, reproducing the sheet's `irene 20%` share row that the totals subtract out) and `in_account` (default true — the row is counted in the `display_value` figure, reproducing the sheet's `AIA display value` cell, which covers only the policies shown in the user's AIA account). Both SHALL be editable.

#### Scenario: Excluded row stays out of totals

- **WHEN** a policy is flagged `excluded`
- **THEN** its premium, value and withdrew amounts are absent from the summary totals but the row is still listed

#### Scenario: Out-of-account row stays out of display value

- **WHEN** a policy is flagged `in_account` false
- **THEN** its value is absent from `display_value` while still counting toward the portfolio totals when not excluded

### Requirement: Manual USD→HKD rate

The system SHALL keep a manual USD→HKD exchange rate (`aia.usd_hkd_rate`) used to derive the HKD totals. The rate SHALL be seeded from the workbook's cached `Overview!N3` at import and SHALL be updatable through `PATCH /api/aia/rate`. When no rate is stored, HKD figures SHALL be absent from the summary.

#### Scenario: Update the rate

- **WHEN** the user patches the rate to `7.8`
- **THEN** the summary's HKD figures are recomputed with `7.8` on the next request

#### Scenario: No rate stored

- **WHEN** no rate has been seeded or set
- **THEN** the summary reports USD figures only

### Requirement: AIA summary

`GET /api/aia/summary` SHALL return every policy with its derived `balance_pct`, the stored rate, and totals derived on read: `premium`, `value`, `withdrew` and overall `balance_pct` — `(value + withdrew − premium) ÷ premium` — over non-excluded policies (the sheet's `buy usd` / `now usd` / `drew` / `balance %%` figures), `display_value` over `in_account` policies (the sheet's `AIA display value`), and each USD total converted to HKD via the stored rate. The summary SHALL also report `next_premium_due` — the earliest `next_pay_date` on or after today across all policies — since the date marks a premium the user still owes. All figures SHALL be recomputed from stored rows on each request.

#### Scenario: Totals mirror the sheet

- **WHEN** policies hold premiums totalling `124781.98` USD, values totalling `89260.78` USD (one excluded row and one out-of-account row handled per their flags) and withdrawals of `379.07` USD
- **THEN** the summary reports those totals, a `display_value` of `87274.32` USD, and the HKD conversions of premium, value and withdrew

#### Scenario: Overall return includes withdrawals

- **WHEN** the totals are premium `124781.98`, value `89260.78`, withdrew `379.07`
- **THEN** the overall `balance_pct` is approximately `−0.2816`

#### Scenario: Next premium due

- **WHEN** two policies carry `next_pay_date` `2027-07-01` and `2028-01-15` and today is before both
- **THEN** the summary's `next_premium_due` is `2027-07-01`

### Requirement: Recording a premium payment

The system SHALL expose `POST /api/aia/events` accepting a policy id, `kind` `payment`, a date, a USD amount, an optional note, and an optional `next_pay_date`. Recording a payment SHALL atomically insert the event row, add the amount to the policy's `premium_usd`, decrement `remaining_years` by one when it is set, and advance the policy's `next_pay_date` — to the submitted date when given, otherwise one year after the current `next_pay_date` — collapsing the sheet's three manual edits into one action. The stored event SHALL also keep the policy's prior `next_pay_date` and `remaining_years` so the change can be reversed.

#### Scenario: Payment updates everything at once

- **WHEN** the user records a `8320` payment dated `2026-07-01` on a policy with premium `16640`, `remaining_years` `3` and `next_pay_date` `2026-07-01`
- **THEN** the policy reports premium `24960`, `remaining_years` `2` and `next_pay_date` `2027-07-01`, and the event row is stored

#### Scenario: Payment with explicit next date

- **WHEN** the user records a payment submitting `next_pay_date` `2027-01-15`
- **THEN** the policy's `next_pay_date` becomes `2027-01-15` instead of the default year-plus-one

#### Scenario: Payment on a fully-paid policy

- **WHEN** a payment is recorded on a policy whose `remaining_years` is unset
- **THEN** only `premium_usd` and `next_pay_date` change

### Requirement: Recording a withdrawal

`POST /api/aia/events` with `kind` `withdrawal` SHALL insert the event row and add the amount to the policy's `withdrew_usd`, so receiving an annuity payout or withdrawal updates the cumulative figure and is logged in one step.

#### Scenario: Withdrawal accumulates

- **WHEN** the user records a `7032` withdrawal on a policy with `withdrew_usd` `127.86`
- **THEN** the policy reports `withdrew_usd` `7159.86` and the event row is stored

### Requirement: Event history and undo

The system SHALL store one event row per recorded payment/withdrawal and expose `GET /api/aia/events` (filterable by `policy_id`) and `DELETE /api/aia/events/:id`. Deleting an event SHALL reverse its effect — subtracting the amount from `premium_usd` or `withdrew_usd` and restoring the `next_pay_date`/`remaining_years` the event recorded — so a misrecorded entry can be undone cleanly.

#### Scenario: Delete undoes a payment

- **WHEN** the user deletes a payment event that added `8320` and recorded prior `next_pay_date` `2026-07-01` and `remaining_years` `3`
- **THEN** the policy's premium drops by `8320` and its `next_pay_date`/`remaining_years` return to `2026-07-01`/`3`

#### Scenario: Event list per policy

- **WHEN** a policy has three recorded payments and one withdrawal
- **THEN** `GET /api/aia/events?policy_id=<id>` returns all four in date order

### Requirement: AIA overview page

The frontend SHALL show an AIA view with a totals strip (USD premium / value / withdrew / overall return / display value plus their HKD conversions, and the next premium due date), an editable exchange-rate field, the policy table (label, policy number, next premium due, premium, value, withdrew, remaining years, per-row return, flags, note) with add/edit/delete actions, a 繳費 action recording a premium payment and a 提取 action recording a withdrawal on each row, and a per-policy payment/withdrawal history. The view SHALL reload after every mutation.

#### Scenario: Table shows derived returns

- **WHEN** a policy has premium `38215`, value `33927.28` and no withdrawals
- **THEN** its row shows a return of approximately `−11.2%`

#### Scenario: Rate edit refreshes HKD figures

- **WHEN** the user saves a new exchange rate
- **THEN** the view reloads and the totals strip shows HKD figures computed with the new rate

#### Scenario: Record a payment from the table

- **WHEN** the user submits the 繳費 form on a policy with an amount and date
- **THEN** the view reloads, the row shows the increased premium, decremented remaining years and advanced next date, and the payment appears in the policy's history
