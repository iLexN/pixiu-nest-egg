# Spec Delta

## REMOVED Requirements

### Requirement: Family deposit 收訖 lifecycle

**Reason**: The 取消收訖 action is removed — the owner never uses it. The surviving behavior is restated as "Family deposit 收訖".
**Migration**: Receipt becomes irreversible in-app. A mistaken 收訖 is corrected by deleting and re-creating the deposit (family receipts have no cash or month-item side effects).

## ADDED Requirements

### Requirement: Family deposit 收訖

Each family deposit SHALL carry a `received_at` date (NULL while unreceived). A receive action SHALL mark it received at a date (default today) and may correct `interest` to the amount paid; re-receiving SHALL return 409. Receipt SHALL NOT be reversible, and SHALL NOT credit any `manual_assets` row, create or delete a Month Stat item, or produce a suggestion. Create and edit SHALL NOT auto-mark past end dates received.

#### Scenario: Confirm receipt with corrected interest

- **WHEN** the user 收訖's 爸爸's SC-9024 with interest `920.29`
- **THEN** the deposit is stored received, its interest is `920.29`, it leaves 爸爸's upcoming list, and no cash row or month item changes

#### Scenario: Receiving twice

- **WHEN** the user 收訖's a family deposit that is already received
- **THEN** the system responds 409 and nothing changes
