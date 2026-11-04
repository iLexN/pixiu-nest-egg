# Spec Delta

## MODIFIED Requirements

### Requirement: Cash account API

The system SHALL expose `GET /api/cash-accounts` and `POST /api/cash-accounts`, plus `PATCH /api/cash-accounts/{id}` and `DELETE /api/cash-accounts/{id}`. PATCH SHALL change any field and refresh `updated_at`. A `broker` account SHALL NOT be deletable (409), because the IBKR block depends on it. Deleting a bank account SHALL leave past receipts and deposit placements readable: their credited account and their debited account are cleared.

#### Scenario: Edit a balance

- **WHEN** the user patches HS's amount to `31000`
- **THEN** HS reports `31000` and live 活期 includes `31000` on the next read

#### Scenario: Broker account protected

- **WHEN** the user deletes the IBKR USD account
- **THEN** the request fails with 409 and the account remains

#### Scenario: Deleting a debited account

- **WHEN** a deposit placement debited 渣打 and the user deletes 渣打
- **THEN** the deposit remains with its debited account cleared and its debited amount kept
