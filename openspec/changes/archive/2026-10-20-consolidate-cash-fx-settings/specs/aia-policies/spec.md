# Spec Delta

## MODIFIED Requirements

### Requirement: AIA summary

`GET /api/aia/summary` SHALL return every policy with its derived `balance_pct`, the stored USD rate from the fx-rates capability (`null` while unset), and totals derived on read: `premium`, `value`, `withdrew` and overall `balance_pct` — `(value + withdrew − premium) ÷ premium` — over non-excluded policies (the sheet's `buy usd` / `now usd` / `drew` / `balance %%` figures), `display_value` over `in_account` policies (the sheet's `AIA display value`), and each USD total converted to HKD via the stored USD rate, absent while no USD rate is stored. The summary SHALL also report `next_premium_due` — the earliest `next_pay_date` on or after today across all policies — since the date marks a premium the user still owes. All figures SHALL be recomputed from stored rows on each request.

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

### Requirement: AIA overview page

The frontend SHALL show an AIA view with a totals strip (USD premium / value / withdrew / overall return / display value plus their HKD conversions, and the next premium due date), an editable USD exchange-rate field saving through `PATCH /api/fx-rates/USD`, the policy table (label, policy number, next premium due, premium, value, withdrew, remaining years, per-row return, flags, note) with add/edit/delete actions, a 繳費 action recording a premium payment and a 提取 action recording a withdrawal on each row, and a per-policy payment/withdrawal history. The view SHALL reload after every mutation.

#### Scenario: Table shows derived returns

- **WHEN** a policy has premium `38215`, value `33927.28` and no withdrawals
- **THEN** its row shows a return of approximately `−11.2%`

#### Scenario: Rate edit refreshes HKD figures

- **WHEN** the user saves a new exchange rate
- **THEN** the view reloads and the totals strip shows HKD figures computed with the new rate

#### Scenario: Record a payment from the table

- **WHEN** the user submits the 繳費 form on a policy with an amount and date
- **THEN** the view reloads, the row shows the increased premium, decremented remaining years and advanced next date, and the payment appears in the policy's history

## REMOVED Requirements

### Requirement: Manual USD→HKD rate

**Reason**: The rate is global, not AIA-specific: IBKR, overview totals, month suggestions and the year review all use it. It now lives in the `fx-rates` capability.

**Migration**: The stored `aia.usd_hkd_rate` value is migrated to the USD fx rate. Use `PATCH /api/fx-rates/USD` with `{ "rate": … }` instead of `PATCH /api/aia/rate`.
