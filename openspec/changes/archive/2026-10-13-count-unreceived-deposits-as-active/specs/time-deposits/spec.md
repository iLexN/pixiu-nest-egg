# Spec Delta

## MODIFIED Requirements

### Requirement: Upcoming deposit view
The system SHALL present the **unreceived** deposits ordered by end date (a matured-but-unreceived deposit is included and flagged 已到期未收), each showing label, principal, rate, interest, total, end_date, and end_month, with a 收訖 action per row. The totals and rollups SHALL be receipt-based, covering exactly the same deposits as the list: a deposit counts while it is not 收訖, whatever its `end_date`, and stops counting once 收訖 — including a deposit 收訖'd before its `end_date`. They are: the total active principal (Σ principal of unreceived deposits); a month rollup grouping those deposits by end year and month with Σ total, Σ interest, and Σ principal; and a bank rollup by `bank` with the same three sums. Deposits without a bank SHALL be grouped under their own bucket rather than matched to a bank. The same active principal feeds Overview 已定期 and the live 總數/流動資產, still at principal only. The parity check SHALL keep comparing against the workbook with the sheet's end-date rule (`End` when today ≥ end date), since the workbook has no receipt concept.

#### Scenario: Active principal total
- **WHEN** the imported workbook data is live and every deposit whose end date had passed at import is recorded as received
- **THEN** the total active principal equals the sum of principal over the unreceived deposits (445,000 for the current workbook)

#### Scenario: Overdue deposit stays listed
- **WHEN** deposits of 110,000 and 35,000 ended `2026-10-13`, today is `2026-10-13`, and neither is 收訖
- **THEN** both appear in 未到期定期 with a 已到期未收 marker, the total active principal and Overview 已定期 still include 145,000, and live 流動資產 is unchanged by the maturity

#### Scenario: 收訖 removes the deposit from the totals
- **WHEN** the user 收訖's an overdue deposit of 110,000 with interest 993, choosing 存入活期 on the HS cash row
- **THEN** the total active principal drops by 110,000, the HS cash row rises by 110,993, and live 流動資產 rises by only the 993 interest

#### Scenario: Early receipt stops counting
- **WHEN** a deposit whose `end_date` is still in the future is 收訖'd
- **THEN** it leaves the list and no longer counts in the total active principal or either rollup

#### Scenario: Month rollup
- **WHEN** active deposits end in October 2026 and November 2026
- **THEN** the rollup lists those months with their Σ total, Σ interest, and Σ principal

#### Scenario: Bank rollup by bank
- **WHEN** active deposits carry banks `SC` (four deposits) and `HS` (one deposit)
- **THEN** the bank rollup reports an `SC` group (Σ principal 365,000) and an `HS` group (Σ principal 80,000)

#### Scenario: Parity keeps the sheet rule
- **WHEN** the parity check runs on a day when an unreceived deposit's end date has passed
- **THEN** it compares the workbook's cached 定期 active principal against a figure computed with the end-date rule, so the unreceived deposit causes no parity difference
