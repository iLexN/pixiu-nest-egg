# Spec Delta

## MODIFIED Requirements

### Requirement: Convert plan to deposit

`POST /api/forecast-items/:id/convert` SHALL accept the deposit fields (`principal` prefilled with `−amount` by the frontend, `end_date` required, `rate`/`interest`/`start_date`/`label`/`note`/`debit_account_id` optional, `bank` defaulting from the item kind). In one transaction it SHALL create the deposit under `POST /api/deposits`'s placement rules (optional 活期 debit, start month `dep-start:<id>` item) and delete the forecast item. A rejected placement SHALL leave the forecast item in place.

#### Scenario: Conversion replaces the plan

- **WHEN** `sc_deposit` item `-180000` in `2026-10` is converted with `start_date` `2026-10-12` and `end_date` `2027-02-10`
- **THEN** a deposit row exists with `principal` `180000` and `bank` `SC`, the forecast item is gone, `2026-10` lists the deposit in `deposit_placements` in place of the plan, and `2027-02` gains the deposit's real `deposit_finish`/`interest` instead of the plan's derived return

#### Scenario: Conversion keeps the grid totals

- **WHEN** the current month `2026-09` holds an `sc_deposit` plan `-55000` and it is converted with `start_date` `2026-09-30`, `end_date` `2027-01-05`, and `interest` `435`
- **THEN** every month's `cash` and `semi_liquid` are the same as before the conversion, except that from `2027-01` onward they are `435` higher, and every month's `locked` is unchanged

#### Scenario: Conversion records the start month item

- **WHEN** an `hs_deposit` plan `-130000` is converted with `start_date` `2026-11-03`, and the `2026-11` month row exists
- **THEN** the `2026-11` month holds a `dep-start:<id>` adjustment item of `-130000`, shown as 自動 rather than as a 建議

#### Scenario: Conversion debits the chosen account

- **WHEN** an `hs_deposit` plan `-130000` is converted with `debit_account_id` naming HS
- **THEN** HS loses `130000` and the deposit records HS as its debited account

#### Scenario: Rejected debit keeps the plan

- **WHEN** a conversion names a `debit_account_id` holding less than the principal
- **THEN** the request returns 400, no deposit is created, and the forecast item still exists
