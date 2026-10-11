# Core data model

Implemented contracts live in [types.rs](../../libs/lib-core/src/types.rs), [traits.rs](../../libs/lib-core/src/traits.rs), and [history.rs](../../libs/lib-core/src/history.rs). Provider clients and the CLI depend on `lib-core`; core does not depend on those clients or the binary. Provider conversions belong in adapters. Existing legacy migration recognition of Nexo credit IDs is described in [persistence](persistence.md).

The [proposed evolution](data-model-proposal.md) maps confirmed features to future entities, exact-amount handling, event evidence, reconciliation, and phased migrations. Its A1 portfolio/account and exact-quantity foundation is now implemented; [stage-3 UI metadata](ui-metadata.md) now persists hierarchy/customization, product/subscription and dated evidence/action foundations. Event provenance, reviewed economic matching and later calculations remain proposed.

## Identity and metadata

`PortfolioId`, `AccountId`, `AssetId`, `ProviderId`, `PositionId`, `ProductId`, and `TransactionId` wrap strings. Legacy adapter asset IDs are often tickers. The [EVM adapter](evm-wallets.md) uses explicit chain/native or chain/contract identities; persistent cross-provider equivalence is still future work. `Asset` contains ID, decimal scale (`u8`), ticker, and symbol. `ExternalAssetId` associates an external identifier with its issuer; [stage-3 alias candidates](ui-metadata.md) persist issuer/namespace/external IDs without automatic ledger matching. Its equality and hash both include issuer and external identifier; namespace-aware persistent matching remains future work.

`UserPosition` references a product and optional start/end dates. `Product` references one asset and an `apy: f64`. Position amounts are derived separately from effects or independent observations, not embedded in `UserPosition`. Portfolio/account metadata and position-account membership persist; minimal provider display, product templates, held subscriptions and unique position relationships now persist separately in [UI metadata](ui-metadata.md). Full connection/catalogue definitions and economic exposure relationships remain future work.

`IsProvider` covers provider identity and aggregate transaction-data fetching, with position/transaction convenience methods. Adapters may expose additional lookup/product capabilities; unsupported capabilities must be reported rather than fabricated. Issuer traits identify external ID sources. Price history contains timestamped asset/quote/price observations. The current `PositionHistory` merely contains `UserPosition` records; independent dated amount observations live in storage.

## Portfolio and account identity

[identity.rs](../../libs/lib-core/src/identity.rs) defines portfolios (ID, label, reporting asset) and accounts (ID, portfolio ID, provider ID, label, optional source reference). One account belongs to one economic portfolio. Labels can change; changing an existing account's portfolio/provider through registration is rejected. Shared access, connection credentials and ownership percentages remain later work.

`Store` provides portfolio/account registration and retrieval, account balance queries, portfolio-scoped balances/exact asset or liability sums/net-worth history, and position-account lookup. The application-service boundary also uses `account_position_metadata` to retain account isolation when constructing exact DTOs. Existing unscoped queries retain their whole-database behavior. `balances_at`/`portfolio_balances_at` select asset positions; `account_balances_at` includes signed assets and liabilities and excludes counterparties. No account-selection CLI/UI has been added by this core change.

`save_account_import` scopes position IDs, transaction IDs and source/hash import markers using stable account identity and a collision-safe serialized namespace tuple. `account_position_id`, `has_account_imported_file` and `account_balance_observations` expose the same identity convention. Identical raw IDs/hashes from different accounts remain independent. Adapter-supplied transaction references must already distinguish endpoint/symbol namespaces and intentionally share an identity only for the same economic event across acquisition sources; account scoping does not discover API/CSV equivalence. Registrations, movements, observations and completion markers commit atomically; unknown accounts, positions, conflicting position identities or scales abort the import.

Legacy Nexo/Binance adapter entry points retain their old provider-wide position/transaction IDs and file hashes. Startup assigns recognized user positions to a default `local:default` portfolio (USD initially), a `legacy:nexo` aggregate account and `legacy:binance:<bucket>` accounts. Unknown legacy namespaces use a clearly labelled unresolved account. External counterparties have no owned account. Existing labels, reporting currency, membership and explicit roles are preserved. Previously merged real accounts cannot be separated without source replay; this migration does not invent that distinction.

## Financial amounts and movements

[Units](../../libs/lib-core/src/amount.rs) wraps an arbitrary-precision signed integer. `TxnEffect.amount` is a nonnegative magnitude checked by storage; inputs reduce balances and outputs increase them. Stored effects are canonical signed integer text (`VARCHAR`), and exact balance aggregation occurs in Rust without narrowing to `BIGINT`/`i128`. Independent observations use the same exact type. JSON quantities serialize as strings. Integer/decimal parsing has a 4,096-character input limit to bound resource use.

`Units::from_decimal` converts plain decimal strings at a chosen scale without rounding; `decimal_string` renders exact human units and `rescale` explicitly converts only when precision is preserved. Nexo and Binance source adapters use bounded `rust_decimal::Decimal` with exact parsing: unsupported precision/range fails instead of being silently rounded. The core ledger is not bounded by that provider-parser type. Nexo keeps its existing eight-place ledger scale; Binance retains its per-asset source precision policy.

Asset scale registrations are immutable: repeated identical registrations succeed, conflicting scales fail. File imports enforce this inside their transaction. Deliberate scale changes require a separately implemented full-history rescaling/replay operation; the pure `rescale` helper does not modify a database. Position asset/role/account registration also rejects conflicting identities rather than reinterpreting existing effects.

Legacy signed `BIGINT` amounts migrate losslessly to integer strings. Old Nexo float-derived results are preserved; widening storage does not repair their original precision. Completed import hashes still skip old sources. New Nexo imports avoid float conversion; reviewed legacy correction/replay remains separate work.

Exact quantities remain exact through balance accumulation, including historical valuation. Prices and returned reporting-currency values still use `f64`; conversion occurs only at the approximate valuation boundary and rejects nonfinite results. Exact-price/proceeds storage remains a proposed later change. A same numeric sum across different currencies is not a meaningful balancing rule. Binance validates movements per asset; the generic transaction type does not enforce double entry by construction.

## Economic roles

Persist `PositionRole::Asset`, `Liability`, or `ExternalCounterparty` explicitly. Asset balance queries select assets; credit-line queries select liabilities. External market, fee, and reward positions represent counterparties and must never become user debt merely because they are not owned.

Liability balances are signed: borrowing/interest decrease them and repayment increases them toward zero. Net worth sums signed asset and liability values. Holding USD 100 and owing USD 20 gives USD 80; a USD 5 repayment moves debt to USD 15, while its asset funding effects are counted separately. Moving assets into collateral or a locked wallet does not create another liability or remove their economic ownership.

See [historical valuation](historical-net-worth.md) for price selection and [Airtable findings](../findings/airtable-model.md) for missing account, yield, subscription, composite-position, and claim relationships.

## Reviewed tracking exclusions

[Account-scoped asset reviews](tracking-exclusions.md) now filter financial totals/history and pricing eligibility, preserve audit effects and raw EVM snapshots, and expose a shared earnings-eligibility check. Unknown contracts need no fabricated scale. Liabilities retain their economic role and remain visible. Full position/product policy and earnings attribution remain proposed.
