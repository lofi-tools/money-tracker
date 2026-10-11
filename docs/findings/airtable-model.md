# Airtable data model comparison

Reviewed 10 October 2026. The spreadsheet tracked a portfolio of providers, products, yield offers, and subscriptions, with manual balances and follow-up notes. The current application has a useful transaction ledger and historical valuation foundation, but cannot yet preserve most of those relationships. Restore account and product metadata, yield terms, liquidity state, and broader loan/claim metadata before expanding connector coverage widely. See [the master todo list](../todo/master-todo.md) for goals and implementation todos.

## Export evidence and limits

The actual directory is `.cache/airtable_export/`, singular. Two CSVs are available:

| Export | Evidence |
| --- | --- |
| `products-model2-draft.csv` | 108 rows; 72 distinct populated `provider` strings; 20 rows without a provider; 13 parent links; 59 signed-up flags; 5 closed flags. |
| `wallets-Default view.csv` | 124 rows; 73 populated old account fields; 63 APY links; 34 migration flags; 45 movable flags; 3 placeholder rows named ` - `. |

These are mixed legacy and draft views. Neither has a snapshot timestamp or explicit fiat valuation currency. Historical nonzero values and signed-up flags are evidence of previous use, not current holdings or service availability. A blank checkbox is not proof of an active account, unrestricted withdrawal, or a never-used provider.

The referenced `APYs` and `individual_subs` tables are not exported. They appear to hold the rate tiers and subscription details that replaced some old wallet fields. The `amount (rollup from individual_subs)` column is empty in every wallet row. Recover those tables before claiming full migration parity.

Do not add provider totals and child product totals together. `provider_amount_fiat (for sorting)` repeats parent totals across children. Do not add crypto quantities across different currencies. Old and new wallet fields coexist, and names repeat: `Coinlist - MINA`, `Binance - BNB`, `Nexo - BNB`, and others need explicit identity resolution rather than name-based deduplication. Keep personal balances and free-text financial amounts in the ignored source exports; this document records their structure and implications.

## What already exists

The implementation, rather than the older spec text, is the baseline:

- [Core types](../../libs/lib-core/src/types.rs) define `ProviderId`, `Asset`, `ExternalAssetId`, `Product`, `UserPosition`, transactions, and balance observations. `UserPosition` already has `product_id`, `start_date`, and `end_date`. `Product` has one `asset_id` and one `apy: f64`.
- [DuckDB storage](../../libs/lib-core/src/storage.rs) persists transaction IDs/times, signed effects, position asset/economic role, asset scales, prices, price coverage, imported file hashes, and independent balance observations. Roles distinguish user assets, user liabilities, and external counterparties. It does **not** persist the full `Product` or `UserPosition` relationships and dates, a provider catalogue, or an account entity.
- [Binance](../../apps/cli/src/adapters/binance.rs) and [statement mapping](../../apps/cli/src/adapters/binance/statements.rs) cover Spot, Funding, Flexible Earn, and Locked Earn. Product/position objects are constructed, but APY is currently a placeholder `0.0`; Locked Earn is aggregated by asset. Current observations are kept separate from ledger effects for reconciliation.
- [Nexo](../../apps/cli/src/adapters/nexo.rs) imports CSV movements into aggregate per-asset positions. Its provider interface explicitly cannot identify individual wallet/product positions and returns no products. Credit activity has a separate mapping, discussed below.
- [CoinGecko](../../apps/cli/src/adapters/coingecko.rs), [Binance Vision](../../apps/cli/src/adapters/binance_vision.rs), and [fallback prices](../../apps/cli/src/adapters/fallback_prices.rs) provide pricing. KuCoin candles and ECB FX already exist as fallbacks; they are not account connectors.
- The [CLI](../../apps/cli/src/main.rs) runs the Nexo/valuation path. Binance workflows are currently exposed through tests and Nix commands described in [the README](../../README.md) and [Binance history guide](binance-history.md). An adapter existing in source is different from a normal multi-provider user workflow.

## Missing and incomplete model features

Ordered by importance for migration and reliable provider integration.

| Priority | Spreadsheet evidence | Current gap | Required model or behavior |
| --- | --- | --- | --- |
| P0 | `provider`, `parent_product`, `DisplayName`, `short_name` | Portfolio/account metadata and position membership now persist. Full provider/product/held-investment relationships remain missing; legacy position IDs still retain their original provider/bucket format. | Stable `Provider`, `Account`, `Product`, and `Position` relationships; display names and aliases; optional product parent; separate account instance IDs for multiple accounts at one provider. |
| P0 | `-> APYs`, `moved_to_individual_subs`, repeated provider/asset rows | Multiple positions are possible in Rust, but subscription identity and terms are not persisted. Binance aggregates Locked positions; Nexo lacks identifying data. | Separate product offers/rate tiers from individual subscriptions. Store source subscription ID, account, asset, principal evidence, opening date, maturity, renewal, and closure. Leave unknown terms unset. |
| P0 | `amount_manual (old)`, empty subscription rollup | No Airtable importer or manual balance workflow. Independent source observations already exist in DuckDB. | Import/manual observations with timestamp, source, confidence, and provenance. Keep an observation distinct from a transaction. An opening checkpoint must be explicitly dated and evidenced; it must not invent earlier history. |
| P0 | `obligation/loss`, provider closure, historical lenders | Asset/liability/counterparty roles and signed liability valuation are now implemented; no general claim model or loan lifecycle metadata. | Add recovery claims, loan principal/interest/repayment metadata, impairment, recoveries, claim valuation, and evidence. |
| P1 | APY links with 7/14/360-day offers, balance caps, first/after threshold bands | One scalar APY; no rate history, tier cap, reward asset, qualification, or term model. | Effective-dated rate schedules with APR/APY semantics, amount bands, fixed/flexible term, compounding, payout asset, and eligibility. `Unknown` must differ from zero yield. |
| P1 | `earn_rate (old)`, `apy-lookup`, `apy-rollup`, `effective_apy` | Live provider rates and portfolio yield are not implemented. | Separate advertised rate, expected income, and observed realized return. Compute a fiat-value-weighted portfolio rate only across comparable, dated inputs. Do not average APYs across unlike terms or assume every linked offer is held. |
| P1 | `epy_crypto` old/new, `epy_fiat`, product yield rollups | No expected annual income or realized income report. Provider event semantics are largely lost in the generic persisted transaction. | Preserve interest, staking reward, cashback, bonus, fee, and gain classifications; derive projections by reward asset and reporting currency; compare projections with actual credited income. Confirm what legacy `epy` means before rebuilding formulas. |
| P1 | `movable`, `Unstaking`, card/exchange staking, fixed terms | No available/locked/pending/unstaking/frozen lifecycle or unlock date. | Liquidity state, withdrawable amount, maturity/unbonding dates, notice period, and restrictions. Report accessible value alongside total value. |
| P1 | Product/wallet `expected movement` notes | No structured unresolved movement or reconciliation issue entity. | Expected incoming/outgoing transfer or reward, source/destination, asset, expected amount/date, status, and retained note. Connect to evidence when resolved; never book a guessed movement automatically. |
| P1 | `Quickswap` ETH/MATIC and MATIC/QUICK rows; Alpha `ibETH`; MetaPool links | One-asset products cannot express pool exposures, receipt-token conversion, or protocol debt. | Composite positions with underlying components, receipt shares, conversion/index observations, pool/contract identity, and reward balances. Value either the receipt or its underlying exposure once. |
| P1 | MetaMask chain children; ImToken, Trust Wallet, Yoroi, Polkadot, NEAR | No persisted chain/address/contract/validator identity. `AssetId` is used as a ticker in adapters. | Chain-qualified token identity and decimals, wallet address/account identity, protocol/version/contracts, validator and delegation data. Preserve economic grouping separately from token identity. |
| P2 | `signed up`, `closed`, `tags`, `short_name` | No account lifecycle, categorization, or research catalogue. | Separate personally used accounts from researched providers/products. Track account status and closure date independently of provider service status; tags, aliases, archive state, and notes. |
| P2 | `amount_fiat (old)`, `total_amount_fiat`, `provider_amount_fiat (for sorting)` | Valuation exists, but not the hierarchical reporting or historical quote metadata from the spreadsheet workflow. | Derived position/product/provider/portfolio reports with explicit quote currency and time. Preserve old figures as reference observations; record price source, staleness, and approximation. |
| P2 | Structured/options tags and Matrixport dual-currency position | No structured payoff or contingent settlement model. | Strike, expiry, settlement currencies, principal at risk, payoff terms, and settlement events. Do not project these as ordinary guaranteed interest. |

### Liability valuation correction

The initial review found that Nexo recorded debt on non-owned positions while net worth valued only owned positions, and credit totals included all non-owned counterparties. This is now corrected: persisted economic roles classify Nexo credit positions as user liabilities and Binance market/reward/fee positions as external counterparties. Signed liability balances reduce net worth; only liability positions enter credit totals. Pricing includes liability-only currencies, and missing liability prices fail valuation.

Older databases automatically migrate known `NEXO-CREDIT:*` positions on open, retaining transactions and imported-file markers. New writes use explicit roles. This restores valuation for debt already represented by the Nexo mapper; it does not add new Nexo transaction types, claim valuation, or a general loan lifecycle model.

### Asset identity and precision need work for on-chain imports

`ExternalAssetId::eq` compares only `id`, while its hash includes `issuer_name` as well. Equal IDs from different issuers can therefore hash differently, violating the equality/hash contract. Fix that before relying on it for provider mappings. Cross-provider asset mappings on `Asset` are currently commented out.

Introduce token identity by chain and contract/mint, native-asset identity by chain, and effective-dated aliases/migrations. Investigate the exported CHSB, MATIC, Elrond eGLD, and LUNA labels against actual source history; do not treat a renamed token, migrated token, fork, and wrapped token as interchangeable merely because tickers resemble each other.

The initial review found Nexo float rounding and a `u64`/signed `BIGINT` ledger boundary. The first foundation implementation now uses arbitrary-precision integer units, lossless legacy migration, exact Nexo/Binance decimal parsing and rejected scale conflicts; see [the current model](../spec/data-model.md). Existing float-derived ledger results are preserved until deliberate source replay. Chain-qualified asset mappings, full-history scale-change operations and on-chain adapters remain future work. Provider decimal parsers are bounded and reject unsupported precision.

### On-chain balances can change without transfer events

Transfers alone are insufficient for yield-bearing positions. Lido distinguishes rebasing stETH from fixed-balance, value-accruing wstETH; Rocket Pool rETH uses an evolving ETH conversion rate. Aave debt balances include accrued interest. Preserve time/block-specific balances, shares, and indices alongside decoded movements. Sources: [Lido token accounting](https://docs.lido.fi/guides/lido-tokens-integration-guide/), [Rocket Pool staking overview](https://docs.rocketpool.net/liquid-staking/overview.html), [Aave tokenization](https://www.aave.com/docs/aave-v3/smart-contracts/tokenization).

## Migration rules

1. Preserve file hash, row identity, raw text, and column names. Airtable record IDs are absent; assign import-scoped row IDs rather than using display names as keys.
2. Import the product catalogue and explicit parent links first; detect cycles and unresolved links. Model parent catalogue rows without forcing an asset onto every row.
3. Preserve old and new wallet fields independently. APY lists indicate candidate offers, not necessarily separate owned subscriptions. Recover missing linked tables to resolve that distinction.
4. Quarantine the three placeholder rows. Retain duplicates for review. A migrated old balance may duplicate a subscription, and a pool's two asset rows may describe one economic position.
5. Treat blank provider fields as unresolved or catalogue-only. IndexCoop, Bitso, Radiant, and others still appear by display name; searching only the `provider` column would miss them. Jump Crypto and Electric Capital are research entries, not automatically financial accounts.
6. Preserve `NaN`, blanks, unknown timestamps, and uncertain notes as unknown states. Do not turn them into zero balances, zero rates, or confirmed movements. Legacy wallet `epy_crypto (old)` has many `1.000` values despite sparse account fields; validate formulas before reuse.
7. Recompute fiat totals and income projections from normalized, dated inputs. The export's rounded totals are comparison evidence, not authoritative valuation history.
8. Reconcile imported observations against provider records at a common cutoff where possible. Keep completeness boundaries visible: a current balance or opening checkpoint cannot establish earlier income, performance, or cost basis.

## Existing work to extend

- [Persistence](../spec/persistence.md) already stores ledger movements, economic roles, prices, and import evidence. Portfolio/account and position membership have been added; full provider/product/held-investment metadata remains outstanding.
- [Historical valuation](../spec/historical-net-worth.md) already subtracts liabilities and excludes external counterparties. Composite-position valuation and broader loan/claim metadata remain outstanding.
- [Binance account history](../spec/binance-account-history.md) includes independent reconciliation and statement/API replay. Prioritize a normal application import/sync workflow and explicit coverage.
- [Nexo ingestion](../spec/nexo.md) already supports configurable paths and atomic file-hash deduplication. Managed uploads and discovery remain future work in [the implementation planning notes](../todo/implementation-gaps.md).
- [Core domain types](../spec/data-model.md) describe the implemented model; the earlier enum IDs, Decimal effects, and embedded position amounts have been superseded.

This comparison records export evidence and gaps. The [prioritized roadmap](../todo/provider-roadmap.md) owns implementation ordering.
