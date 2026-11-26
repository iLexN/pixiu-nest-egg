# Spec Delta

## ADDED Requirements

### Requirement: USD rate refreshed by a US price fetch

When the user triggers a stock price fetch for market US, the system SHALL also fetch the USD→HKD rate (`USDHKD=X`) and store it as the USD rate, unrounded, with a fresh timestamp. A failed rate fetch SHALL be reported and SHALL leave the stored rate unchanged.

#### Scenario: US fetch updates the stored rate

- **WHEN** a US market price fetch runs and the `USDHKD=X` response carries a rate of `7.843045`
- **THEN** the stored USD rate is exactly `7.843045` with an updated timestamp, and the fetch report includes the applied rate

#### Scenario: Rate fetch fails

- **WHEN** a US market price fetch runs but the `USDHKD=X` request fails
- **THEN** the stored USD rate is unchanged and the failure is reported; the stock prices that succeeded are still applied
