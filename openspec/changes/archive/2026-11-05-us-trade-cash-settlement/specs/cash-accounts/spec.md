# Spec Delta

## ADDED Requirements

### Requirement: Broker account adjustments by trades

A US trade's create, edit, and delete SHALL adjust the USD `broker` account per the stock-trades settlement rules — BUY subtracts the total, SELL adds it, and reversal follows the amount recorded on the trade. These adjustments SHALL run inside the same write as the trade operation and MAY push the broker amount negative, exempt from the non-negative rule on user-entered amounts. HK trades SHALL make no broker adjustment.

#### Scenario: Trade settles broker cash atomically

- **WHEN** a US `BUY` with total `2089.12` is recorded while the USD broker account holds `5000`
- **THEN** the trade and the account's new amount `2910.88` are both visible — a failure stores neither

#### Scenario: Adjustment may go negative

- **WHEN** a US `BUY` exceeds the USD broker balance
- **THEN** the account amount goes negative rather than rejecting the trade
