# Spec Delta

## MODIFIED Requirements

### Requirement: Manual USD→HKD rate

The system SHALL keep a manual USD→HKD exchange rate (`aia.usd_hkd_rate`) used to derive the HKD totals. The rate SHALL be updatable through `PATCH /api/aia/rate`. When no rate is stored, HKD figures SHALL be absent from the summary.

#### Scenario: Update the rate

- **WHEN** the user patches the rate to `7.8`
- **THEN** the summary's HKD figures are recomputed with `7.8` on the next request

#### Scenario: No rate stored

- **WHEN** no rate has been set
- **THEN** the summary reports USD figures only
