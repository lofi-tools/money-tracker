## 1. Remove Rust Decimal

- [ ] 1.1 Remove `rust_decimal` from `libs/lib-core/Cargo.toml`
- [ ] 1.2 Remove any usage of `rust_decimal` in the codebase (if any)

## 2. Update Core Types

- [ ] 2.1 Update `Asset` struct in `libs/lib-core/src/types.rs` to add `pub decimals: u8`
- [ ] 2.2 Update `Position` struct in `libs/lib-core/src/types.rs` to change `amount` to `u64`
- [ ] 2.3 Update `TxInputOutput` struct in `libs/lib-core/src/types.rs` to change `amount` to `u64`
- [ ] 2.4 Update `Asset` constructors/builders to require `decimals` (default to 0 or 18 if unknown?)

## 3. Update Adapters

- [ ] 3.1 Update `bin/cli/src/adapters/binance.rs` to convert `f64` amounts to `u64`
- [ ] 3.2 Update `bin/cli/src/adapters/nexo.rs` to convert `f64` amounts to `u64`
- [ ] 3.3 Update `bin/cli/src/adapters/coingecko.rs` (if applicable)

## 4. Verification

- [ ] 4.1 Run `cargo build --workspace` to ensure all types match
- [ ] 4.2 Run tests to verify conversion logic
