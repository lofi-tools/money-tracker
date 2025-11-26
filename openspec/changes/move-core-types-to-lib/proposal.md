# Change: Move Core Domain Types to Core Library

## Why

Core domain types (Asset, Position, Product, Transaction, Provider traits) are currently defined in the binary crate (`bin/crypto-tracker/src/models.rs`) but represent shared domain concepts that should be accessible to all provider adapters and libraries. This creates unnecessary coupling and limits code reuse, as client libraries cannot reference these types directly.

## What Changes

- Create new `libs/core` library crate containing all domain types and traits
- Move domain models (Asset, Position, Product, Transaction, etc.) from `bin/crypto-tracker/src/models.rs` to `libs/core/src/types.rs`
- Move provider traits (IsProvider, Issuer3, etc.) to `libs/core/src/traits.rs`
- Update workspace dependencies to include the new core library
- Update all existing crates to import types from the core library instead of the binary
- Preserve all existing functionality and type implementations

## Impact

- **Affected specs**: `core-domain-types` (new capability)
- **Affected code**:
  - `bin/crypto-tracker/src/models.rs` - Types will be moved from here
  - `bin/crypto-tracker/src/adapters/binance.rs` - Import path updates
  - `bin/crypto-tracker/src/adapters/nexo.rs` - Import path updates
  - `bin/crypto-tracker/src/adapters/coingecko.rs` - Import path updates
  - `bin/crypto-tracker/src/main.rs` - Import path updates
  - `Cargo.toml` - Add core library to workspace dependencies
  - `libs/binance-client` - Can now use core types directly (future enhancement)
  - `libs/coingecko-client` - Can now use core types directly (future enhancement)
  - `libs/nexo-csv` - Can now use core types directly (future enhancement)
- **Breaking changes**: None - This is a refactoring that preserves all existing APIs
