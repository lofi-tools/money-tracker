## ADDED Requirements

### Requirement: Integer-based Asset Amounts

The system SHALL represent all asset amounts as unsigned 64-bit integers (`u64`) representing the smallest unit of the asset.

#### Scenario: Asset definition includes precision
- **GIVEN** an `Asset` definition
- **WHEN** it is instantiated
- **THEN** it SHALL include a `decimals` field of type `u8` indicating the number of decimal places

#### Scenario: Positions use integer amounts
- **GIVEN** a `Position`
- **WHEN** the amount is accessed
- **THEN** it SHALL be a `u64` integer

#### Scenario: Transaction inputs/outputs use integer amounts
- **GIVEN** a `TxInputOutput`
- **WHEN** the amount is accessed
- **THEN** it SHALL be a `u64` integer

### Requirement: No Decimal Dependency

The core library SHALL NOT depend on `rust_decimal`.

#### Scenario: Core library dependencies
- **GIVEN** the `libs/lib-core/Cargo.toml` file
- **WHEN** dependencies are checked
- **THEN** `rust_decimal` SHALL NOT be present
