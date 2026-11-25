# Spec Delta

## MODIFIED Requirements

### Requirement: Cash account API

The system SHALL expose `GET /api/cash-accounts` and `POST /api/cash-accounts`, plus `PATCH /api/cash-accounts/{id}` and `DELETE /api/cash-accounts/{id}`, and `POST /api/cash-accounts/moves` for 活期調撥. PATCH SHALL change any field and refresh `updated_at`. A `broker` account SHALL NOT be deletable (409), because the IBKR block depends on it. Deleting a bank account SHALL leave past receipts and deposit placements readable: their credited account and their debited account are cleared.

#### Scenario: Edit a balance

- **WHEN** the user patches HS's amount to `31000`
- **THEN** HS reports `31000` and live 活期 includes `31000` on the next read

#### Scenario: Broker account protected

- **WHEN** the user deletes the IBKR USD account
- **THEN** the request fails with 409 and the account remains

#### Scenario: Deleting a debited account

- **WHEN** a deposit placement debited 渣打 and the user deletes 渣打
- **THEN** the deposit remains with its debited account cleared and its debited amount kept

## ADDED Requirements

### Requirement: 活期調撥 between bank accounts

`POST /api/cash-accounts/moves` with `from_account_id`, `to_account_id`, and `amount` SHALL, in one transaction, subtract `amount` from the source account and add exactly `amount` to the destination, refreshing both `updated_at`. It SHALL respond `200` with the two updated accounts, source first. The 活期 total SHALL be unchanged by a move.

#### Scenario: Move HS to 渣打

- **WHEN** HS holds `30000`, 渣打 holds `1549.01`, and the user moves `10000` from HS to 渣打
- **THEN** the response lists HS at `20000` then 渣打 at `11549.01`, and live 活期 is unchanged

#### Scenario: Move back undoes a move

- **WHEN** the user moves `10000` from HS to 渣打 and then `10000` from 渣打 to HS
- **THEN** both accounts are back at their original amounts

### Requirement: 活期調撥 validation

A move SHALL be rejected with `400` and change nothing when `amount` is not a finite number greater than `0`, when either account is not an HKD `bank` account, when the source and destination are the same account, or when the source holds less than `amount`. An unknown account id SHALL respond `404` and change nothing.

#### Scenario: Overdraft rejected

- **WHEN** HS holds `5000` and the user moves `8000` from HS to 渣打
- **THEN** the request fails with 400 naming `amount`, and neither account changes

#### Scenario: Broker account rejected

- **WHEN** the user moves `1000` from HS to the IBKR HKD broker account
- **THEN** the request fails with 400 naming `to_account_id`, and no account changes

#### Scenario: Same account rejected

- **WHEN** the user moves `1000` from HS to HS
- **THEN** the request fails with 400 naming `to_account_id`, and HS is unchanged

#### Scenario: Non-positive amount rejected

- **WHEN** the user moves `0` from HS to 渣打
- **THEN** the request fails with 400 naming `amount`, and neither account changes

#### Scenario: Unknown account

- **WHEN** the user moves `1000` from HS to an id that names no account
- **THEN** the request fails with 404 and HS is unchanged

### Requirement: 活期調撥 records nothing

A move SHALL NOT store any log row, month item, candidate, or forecast line. It SHALL NOT change any month's 月初, 總數, or 流動資產, and SHALL NOT be listed or undoable on its own; the reverse move is the undo.

#### Scenario: No month item

- **WHEN** the user moves `10000` from HS to 渣打 in a month whose row exists
- **THEN** that month's items and candidates are unchanged
