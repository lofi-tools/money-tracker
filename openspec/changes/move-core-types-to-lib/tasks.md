## 1. Create Core Library Structure

- [ ] 1.1 Create `libs/core/` directory structure
- [ ] 1.2 Create `libs/core/Cargo.toml` with minimal dependencies (serde, chrono, async-trait)
- [ ] 1.3 Create `libs/core/src/lib.rs` as library entry point

## 2. Move Domain Types

- [ ] 2.1 Create `libs/core/src/types.rs` for domain types
- [ ] 2.2 Move `AssetId`, `Asset`, `ExternalAssetId` from models.rs to types.rs
- [ ] 2.3 Move `ProviderId` from models.rs to types.rs
- [ ] 2.4 Move `PositionId`, `Position`, `AllPositions` from models.rs to types.rs
- [ ] 2.5 Move `ProductId`, `Product`, `AllProducts` from models.rs to types.rs
- [ ] 2.6 Move `TransactionId`, `Transaction`, `TxInputOutput` from models.rs to types.rs
- [ ] 2.7 Move `Db` struct from models.rs to types.rs
- [ ] 2.8 Create `libs/core/src/history.rs` and move history module (AssetPricePoint, AssetPriceHistory, PositionHistory)

## 3. Move Provider Traits

- [ ] 3.1 Create `libs/core/src/traits.rs`
- [ ] 3.2 Move `IsProvider`, `Issuer2`, `Issuer3` traits from models.rs to traits.rs
- [ ] 3.3 Ensure all trait bounds and imports are preserved

## 4. Update Workspace Configuration

- [ ] 4.1 Add `core = { path = "./libs/core" }` to workspace.dependencies in root Cargo.toml
- [ ] 4.2 Add `lib-core.workspace = true` to bin/crypto-tracker/Cargo.toml dependencies
- [ ] 4.3 Verify workspace structure with `cargo metadata`

## 5. Update Import Paths in Binary

- [ ] 5.1 Update `bin/crypto-tracker/src/main.rs` to use `use core::types::*` and `use core::traits::*`
- [ ] 5.2 Update `bin/crypto-tracker/src/adapters/binance.rs` import paths
- [ ] 5.3 Update `bin/crypto-tracker/src/adapters/nexo.rs` import paths
- [ ] 5.4 Update `bin/crypto-tracker/src/adapters/coingecko.rs` import paths
- [ ] 5.5 Remove or minimize `bin/crypto-tracker/src/models.rs` (keep only binary-specific extensions like `impl AssetId::from_binance()`)

## 6. Preserve Type Extensions

- [ ] 6.1 Keep adapter-specific AssetId extensions (from_binance, from_nexo_asset, from_coingecko) in their respective adapter files as `impl AssetId` blocks
- [ ] 6.2 Verify all extension methods are still accessible after the move

## 7. Testing and Validation

- [ ] 7.1 Run `cargo check --workspace` to verify compilation
- [ ] 7.2 Run `cargo build --workspace` to ensure all crates build
- [ ] 7.3 Run unit tests: `scripts/utest` (if available) or `cargo test --workspace`
- [ ] 7.4 Verify no functional changes - all existing behavior preserved
- [ ] 7.5 Ensure import paths are minimal and idiomatic

## 8. Documentation

- [ ] 8.1 Add `libs/core/README.md` documenting the purpose and contents of the core library
- [ ] 8.2 Add rustdoc comments to public types and traits in core library
- [ ] 8.3 Update root README.md if it mentions the architecture to reflect new structure
