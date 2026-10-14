# Spec Delta

## MODIFIED Requirements

### Requirement: Isolation from the user's own figures

Family deposits SHALL NOT be counted in any figure derived from the user's own deposits: the 定期 summary (active principal, month/bank/year rollups), Overview 已定期 / 半流動資金 / 總數 / 流動資產, Month Stat 利息, `interest_auto`, and deposit suggestions, and the 年結 回顧 定期 figures.

#### Scenario: User figures unchanged

- **WHEN** a family deposit of 300000 with interest 920.29 is created and later 收訖'd
- **THEN** the 定期 summary active principal, the Overview 總數 and 流動資產, and the month's `interest_auto` are identical to their values before the family deposit existed
