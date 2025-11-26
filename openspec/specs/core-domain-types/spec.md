# core-domain-types Specification

## Purpose
TBD - created by archiving change move-core-types-to-lib. Update Purpose after archive.
## Requirements
### Requirement: Core Domain Types Library

The system SHALL provide a core library (`libs/core`) that contains all shared domain types and traits used across the application. This library SHALL be independent of any specific provider implementation and SHALL serve as the foundation for domain modeling.

#### Scenario: Core library exists and is accessible

- **GIVEN** the workspace is properly configured
- **WHEN** a crate adds `lib-core.workspace = true` to its dependencies
- **THEN** it SHALL have access to all public types and traits from the core library

#### Scenario: Domain types are defined in core

- **GIVEN** the core library is available
- **WHEN** code needs to reference Asset, Position, Product, or Transaction types
- **THEN** these types SHALL be importable from `core::types`

### Requirement: Asset Domain Types

The core library SHALL define comprehensive asset-related types including `AssetId`, `Asset`, and `ExternalAssetId`. These types SHALL support multiple asset representations across different providers.

#### Scenario: AssetId enum represents known and unknown assets

- **GIVEN** an asset needs to be referenced
- **WHEN** the asset is a known type (e.g., Ethereum)
- **THEN** `AssetId::Eth` SHALL be used
- **WHEN** the asset is unknown or provider-specific
- **THEN** `AssetId::Unknown(String)` SHALL be used

#### Scenario: External asset IDs map to providers

- **GIVEN** an asset exists on multiple provider platforms
- **WHEN** an `ExternalAssetId` is created with a provider's Issuer trait
- **THEN** it SHALL store the provider's asset identifier
- **AND** it SHALL be associated with that specific provider

#### Scenario: Assets can have multiple external IDs

- **GIVEN** an `Asset` instance
- **WHEN** external IDs from different providers are added
- **THEN** the Asset SHALL maintain a HashMap of ProviderId to ExternalAssetId
- **AND** external IDs from different providers SHALL coexist without conflict

### Requirement: Position and Product Types

The core library SHALL define `Position`, `Product`, and related types to model financial positions and investment products across all providers.

#### Scenario: Position represents staked or invested amounts

- **GIVEN** a user has an investment position
- **WHEN** the position is created
- **THEN** it SHALL have a unique PositionId
- **AND** it SHALL reference a ProductId
- **AND** it SHALL track the amount, start_date, and end_date

#### Scenario: Product describes investment offerings

- **GIVEN** a provider offers an investment product
- **WHEN** the product is modeled
- **THEN** it SHALL have a unique ProductId
- **AND** it SHALL reference an AssetId
- **AND** it SHALL include the APY (annual percentage yield)

### Requirement: Transaction Types

The core library SHALL define transaction-related types (`Transaction`, `TransactionId`, `TxInputOutput`) to represent all financial movements and operations.

#### Scenario: Transaction captures inputs and outputs

- **GIVEN** a financial transaction occurs
- **WHEN** the transaction is represented
- **THEN** it SHALL have a unique TransactionId
- **AND** it SHALL have a datetime timestamp
- **AND** it SHALL have a list of inputs (TxInputOutput)
- **AND** it SHALL have a list of outputs (TxInputOutput)

#### Scenario: Transaction input/output specifies asset and amount

- **GIVEN** a transaction input or output
- **WHEN** it is created
- **THEN** it SHALL specify an AssetId
- **AND** it SHALL specify a numeric amount

### Requirement: Provider Traits

The core library SHALL define traits (`IsProvider`, `Issuer2`, `Issuer3`) that establish contracts for provider implementations.

#### Scenario: IsProvider trait defines provider interface

- **GIVEN** a provider service implementation
- **WHEN** it implements the IsProvider trait
- **THEN** it SHALL provide a provider_id() method
- **AND** it SHALL implement async fetch_positions() returning Vec&lt;Position&gt;
- **AND** it SHALL implement async fetch_transactions() returning Vec&lt;Transaction&gt;

#### Scenario: Issuer trait identifies external ID sources

- **GIVEN** a provider needs to be identified for external asset IDs
- **WHEN** the Issuer3 trait is implemented
- **THEN** it SHALL provide a static name() method returning the provider's identifier

### Requirement: Historical Data Types

The core library SHALL define types for tracking historical price and position data (`AssetPricePoint`, `AssetPriceHistory`, `PositionHistory`).

#### Scenario: AssetPricePoint captures price at a moment

- **GIVEN** an asset's price needs to be recorded
- **WHEN** an AssetPricePoint is created
- **THEN** it SHALL have a datetime timestamp
- **AND** it SHALL reference the priced AssetId
- **AND** it SHALL reference the denominating AssetId (vs_asset_id)
- **AND** it SHALL include the numeric price value

#### Scenario: Price history is a collection of price points

- **GIVEN** historical prices for an asset
- **WHEN** represented as AssetPriceHistory
- **THEN** it SHALL contain a vector of AssetPricePoint instances

### Requirement: Core Library Independence

The core library SHALL have minimal dependencies and SHALL NOT depend on provider-specific implementations.

#### Scenario: Core library has minimal dependencies

- **GIVEN** the core library Cargo.toml
- **WHEN** dependencies are reviewed
- **THEN** it SHALL only include essential crates (serde, chrono, async-trait, serde_json, derive_more)
- **AND** it SHALL NOT depend on any provider client libraries (binance-client, coingecko-client, nexo-csv)
- **AND** it SHALL NOT depend on the binary crate

#### Scenario: Provider-specific conversions remain in adapters

- **GIVEN** a provider needs to convert its API types to core types
- **WHEN** the conversion logic is implemented
- **THEN** provider-specific extension methods (e.g., `AssetId::from_binance()`) SHALL remain in the adapter layer
- **AND** the core library SHALL NOT contain provider-specific mapping logic

