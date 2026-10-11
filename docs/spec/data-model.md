# Core data model

Implemented contracts live in [types.rs](../../libs/lib-core/src/types.rs), [traits.rs](../../libs/lib-core/src/traits.rs), and [history.rs](../../libs/lib-core/src/history.rs). Provider clients and the CLI depend on `lib-core`; core does not depend on those clients or the binary. Provider conversions belong in adapters. Existing legacy migration recognition of Nexo credit IDs is described in [persistence](persistence.md).

The [proposed evolution](data-model-proposal.md) maps confirmed features to future entities, exact-amount handling, event evidence, reconciliation, and phased migrations. It is a design review, not implemented behavior.

## Identity and metadata

`AssetId`, `ProviderId`, `PositionId`, `ProductId`, and `TransactionId` wrap strings. Asset IDs are currently often tickers, not chain/contract identities. `Asset` contains ID, decimal scale (`u8`), ticker, and symbol. `ExternalAssetId` associates an external identifier with its issuer; a persistent cross-provider mapping catalogue is not implemented. Its equality/hash consistency needs correction in [the master todo list](../todo/master-todo.md).

`UserPosition` references a product and optional start/end dates. `Product` references one asset and an `apy: f64`. Position amounts are derived separately from effects or independent observations, not embedded in `UserPosition`. Full provider/account/product metadata and these relationships are not yet stored in DuckDB.

`IsProvider` covers provider identity and aggregate transaction-data fetching, with position/transaction convenience methods. Adapters may expose additional lookup/product capabilities; unsupported capabilities must be reported rather than fabricated. Issuer traits identify external ID sources. Price history contains timestamped asset/quote/price observations. The current `PositionHistory` merely contains `UserPosition` records; independent dated amount observations live in storage.

## Financial amounts and movements

`TxnEffect` holds position ID, unsigned `u64` smallest-unit amount, and UTC timestamp. `Transaction` holds inputs, outputs, and its UTC timestamp; persistent transaction identity is supplied separately. Inputs reduce balances; outputs increase balances. Effects persist as signed `BIGINT` values, so storage must reject values outside signed 64-bit range. In-memory cumulative balances use `i128`, but persisted exact-balance queries cast sums to `BIGINT` and read `i64` before widening; aggregate overflow remains possible. The core does not require a Decimal amount dependency; exact decimal parsing at ingestion remains essential, and float rounding in Nexo is a known gap.

Asset scale determines the conversion from integer units. Scale should remain stable across imports, but current scale registrations use replacement upserts rather than rejecting conflicting scales; this is a gap addressed by the proposal. Reject amount overflow. A same numeric sum across different currencies is not a meaningful balancing rule. Binance uses per-asset counterparties and mapping validation; the generic transaction type does not enforce double entry by construction.

## Economic roles

Persist `PositionRole::Asset`, `Liability`, or `ExternalCounterparty` explicitly. Asset balance queries select assets; credit-line queries select liabilities. External market, fee, and reward positions represent counterparties and must never become user debt merely because they are not owned.

Liability balances are signed: borrowing/interest decrease them and repayment increases them toward zero. Net worth sums signed asset and liability values. Holding USD 100 and owing USD 20 gives USD 80; a USD 5 repayment moves debt to USD 15, while its asset funding effects are counted separately. Moving assets into collateral or a locked wallet does not create another liability or remove their economic ownership.

See [historical valuation](historical-net-worth.md) for price selection and [Airtable findings](../findings/airtable-model.md) for missing account, yield, subscription, composite-position, and claim relationships.
