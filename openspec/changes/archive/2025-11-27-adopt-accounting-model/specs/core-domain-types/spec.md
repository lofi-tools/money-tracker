# core-domain-types Specification Delta

## MODIFIED Requirements

### Requirement: Transaction Types

The core library SHALL define transaction-related types (`Transaction`, `TransactionOutput`) to represent all financial movements using a double-entry accounting model.

#### Scenario: Transaction consists of outputs (effects)

- **GIVEN** a financial transaction
- **WHEN** it is represented
- **THEN** it SHALL have a `datetime` timestamp
- **AND** it SHALL have a list of `outputs` (TransactionOutput)
- **AND** it SHALL NOT have separate input/output lists

#### Scenario: TransactionOutput represents account balance change

- **GIVEN** a transaction output (effect)
- **WHEN** it is defined
- **THEN** it SHALL have an `account_id`
- **AND** it SHALL have an `amount` (Decimal) representing the change (negative for outflow, positive for inflow)
- **AND** it SHALL have a `datetime` timestamp
