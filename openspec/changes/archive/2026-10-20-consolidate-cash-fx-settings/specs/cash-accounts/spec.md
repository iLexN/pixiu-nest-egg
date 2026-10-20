# Spec Delta

## Purpose

Tracks the user's cash balances as currency-aware accounts (bank 活期 such as HS and 渣打, and the IBKR broker's HKD/USD cash). Every 收訖 credits money through one rule: a default account per currency, an optional per-receipt override, and exact reversal.

## ADDED Requirements

### Requirement: Cash account registry

The system SHALL store cash accounts, each with:

- `label` (required, unique)
- `currency` (`HKD` or `USD`)
- `role` (`bank` = 活期 or `broker` = IBKR cash)
- `amount` (a user-entered amount MUST be finite and non-negative; receipt reversals adjust it without that check)
- `is_default_receipt`
- `sort_order` and `updated_at`

A `bank` account MUST be `HKD`. At most one `broker` account per currency SHALL exist.

#### Scenario: Create a bank account

- **WHEN** the user creates account `HS` with currency `HKD`, role `bank`, amount `30538.78`
- **THEN** the account is stored and listed with those fields

#### Scenario: USD bank account rejected

- **WHEN** an account with role `bank` and currency `USD` is submitted
- **THEN** the request is rejected with a field error and nothing is stored

#### Scenario: Second broker account per currency rejected

- **WHEN** a `broker` `USD` account already exists and another is submitted
- **THEN** the request is rejected and nothing is stored

### Requirement: Cash account API

The system SHALL expose `GET /api/cash-accounts` and `POST /api/cash-accounts`, plus `PATCH /api/cash-accounts/{id}` and `DELETE /api/cash-accounts/{id}`. PATCH SHALL change any field and refresh `updated_at`. A `broker` account SHALL NOT be deletable (409), because the IBKR block depends on it. Deleting a bank account SHALL leave past receipts readable, with their credited account cleared.

#### Scenario: Edit a balance

- **WHEN** the user patches HS's amount to `31000`
- **THEN** HS reports `31000` and live 活期 includes `31000` on the next read

#### Scenario: Broker account protected

- **WHEN** the user deletes the IBKR USD account
- **THEN** the request fails with 409 and the account remains

### Requirement: Default receipt account per currency

Each currency SHALL have at most one account flagged `is_default_receipt`. Setting the flag on an account SHALL clear it from the other accounts of the same currency, in the same write. A currency MAY have no default.

#### Scenario: Default moves

- **WHEN** HS is the HKD default and the user flags 渣打 as default
- **THEN** 渣打 is the only HKD default and HS is no longer flagged

### Requirement: Receipt crediting rule

Every 收訖 SHALL credit, in the same transaction as the receipt, the account chosen as follows:

1. If the request sets `bank_in` false, nothing is credited.
2. Otherwise the request's `account_id` is credited, if given.
3. Otherwise the default account of the receipt's currency is credited.
4. With no default, nothing is credited.

`account_id` MUST name an account in the receipt's currency, else 400.

#### Scenario: Default used

- **WHEN** an HKD receipt of `1000` arrives with no `account_id` and HS is the HKD default
- **THEN** HS gains `1000`, and the receipt records HS and `1000` as its credit

#### Scenario: Override used

- **WHEN** an HKD receipt of `1000` arrives with `account_id` = 渣打
- **THEN** 渣打 gains `1000` and HS is unchanged

#### Scenario: Currency mismatch rejected

- **WHEN** a USD receipt names an HKD account
- **THEN** the request fails with 400 and nothing changes

#### Scenario: No default, no credit

- **WHEN** a USD receipt arrives with no `account_id` and no USD default exists
- **THEN** the receipt is recorded with no credited account or amount

### Requirement: Recorded credit and exact reversal

A receipt SHALL store the account it credited and the amount, both empty when nothing was credited. Reversing a receipt SHALL subtract the stored amount from the stored account, whatever the defaults are at that time, and then clear both. When the credited account has since been deleted, the reversal SHALL change no account.

#### Scenario: Reversal follows the recorded account

- **WHEN** a receipt credited HS `1000`, the HKD default then moves to 渣打, and the receipt is reversed
- **THEN** HS loses `1000` and 渣打 is unchanged

#### Scenario: Reversal after the account was deleted

- **WHEN** a receipt credited 渣打 `500`, 渣打 is then deleted, and the receipt is reversed
- **THEN** the receipt's credit is cleared and no remaining account changes
