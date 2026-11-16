# Spec Delta

## MODIFIED Requirements

### Requirement: Dividend receipt reversal

**Reversal.** Clearing `received_amount` SHALL reverse the recorded credit on the account it went to, delete the `div:<id>` item in whichever month it sits together with any dismissal of that key, and clear `withholding_tax`.

#### Scenario: Clearing reverses the receipt

- **WHEN** the user clears `received_amount` on a received HK dividend
- **THEN** the credited cash account loses the amount, the `div:<id>` item is deleted, and `withholding_tax` is cleared

#### Scenario: Clearing after the pay date moved

- **WHEN** an HK dividend received with its `div:<id>` item in `2026-09` is updated in one request to a `pay_date` in `2026-10` with `received_amount` cleared
- **THEN** the credit is reversed and no `div:<id>` item remains in either month
