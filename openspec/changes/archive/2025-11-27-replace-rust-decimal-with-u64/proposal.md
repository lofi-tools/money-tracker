# Change: Replace Rust Decimal with u64

## Why

The current implementation uses `f64` for financial amounts, which can lead to precision issues. While `rust_decimal` was added as a dependency, the user has requested to use a simpler approach using `u64` for amounts and storing the decimal precision in the `Asset` definition. This avoids the overhead of an external decimal library and allows for precise integer arithmetic.

## What Changes

- Remove `rust_decimal` dependency from `libs/lib-core`.
- Update `Asset` struct in `libs/lib-core/src/types.rs` to include a `decimals: u8` field.
- Update `Position` struct in `libs/lib-core/src/types.rs` to use `amount: u64` instead of `f64`.
- Update `TxInputOutput` struct in `libs/lib-core/src/types.rs` to use `amount: u64` instead of `f64`.
- Update all call sites and adapters to convert between `f64` (from APIs) and `u64` (internal representation) using the asset's decimal precision.

## Impact

- **Affected specs**: `financial-primitives` (new capability), `core-domain-types` (modified).
- **Breaking changes**: This is a breaking change for the internal data model. All adapters will need to be updated.
