# Provider implementation goals and todo list

Reviewed 10 October 2026. Prioritize replacement of the user's existing spreadsheet workflow, then broader coverage. The recommended next providers are **Crypto.com, EVM wallets, Kraken, and OKX**, alongside a generic history/claims import path. Coinbase already appears in the exports. Major omissions worth adding include **Aave, Uniswap, Morpho, Bybit, Bitget, KuCoin, Gate, and Bitstamp**, plus Bitcoin and Solana wallet coverage. These are integration priorities, not recommendations to move funds or use any financial product.

Read [the model findings](airtable-model-findings.md) first. A provider catalogue, an account connector, a chain reader, a DeFi decoder, and a price feed serve different purposes and should not be counted as equivalent integrations.

## Goals and completion criteria

| Order | Goal | Done when |
| --- | --- | --- |
| 1 | Preserve the existing portfolio without waiting for every API connector | Every meaningful export row is mapped or explicitly unresolved; linked APY/subscription tables are recovered or marked unavailable; manual historical observations can be retained with provenance. |
| 2 | Report accurate assets, liabilities, and accessible value | User debt reduces net worth; external counterparties do not; locked, frozen, and pending value are visible; nested products and receipt tokens are not counted twice. |
| 3 | Replace the highest-value personal tracking workflows | Crypto.com surfaces, relevant wallet chains, Kraken, and OKX import history and independent balances with documented scope and reconciliation at a known cutoff. |
| 4 | Restore yield and income tracking | Rate tiers and subscription terms survive storage/reload; unknown rates are explicit; expected income and actual rewards are separate; locked terms and withdrawal availability are visible. |
| 5 | Expand to reusable market coverage | Additional exchange and protocol adapters reuse the same account, identity, provenance, precision, and completeness contracts. |

Priority reflects exported evidence of use, breadth unlocked, data accessibility, and prerequisites. Historic fiat amounts are not used as current portfolio weights. Reorder a connector upward when current holdings, missing history, or an unresolved recovery make it more valuable.

## Immediate todos in descending order

- [ ] **1. Recover missing Airtable inputs.** Obtain `APYs`, `individual_subs`, and any currency/provider tables or formula definitions. Preserve original exports locally. Record the snapshot date and fiat reporting currency if known.
- [ ] **2. Define and persist provider/account/product/position relationships.** Extend the existing persistence proposal; support multiple account instances, nested products, aliases, and subscription IDs. Add catalogue-only products with unknown assets/terms where appropriate.
- [ ] **3. Complete liability and claim tracking.** Basic asset/liability/counterparty roles, legacy Nexo migration, signed liability valuation, and liability pricing are implemented. Remaining work: broader loan repayment mapping/metadata and claims/recovery evidence; retain owned collateral as an asset.
- [ ] **4. Add safe Airtable/manual import.** Preserve row provenance, old/new fields, unresolved links, and independent observations. Require dated evidence for opening checkpoints. Report duplicates and formula uncertainty instead of silently merging.
- [ ] **5. Finish a normal multi-provider application workflow.** Expose existing Nexo/Binance imports and reconciliation through ordinary CLI commands into a chosen portfolio database. Support account selection, offline replay, incremental sync, retained errors, and provider coverage reports.
- [ ] **6. Make identity, exact amounts, and provenance reusable.** Fix `ExternalAssetId` equality/hash, persist provider asset mappings, define chain-qualified tokens and exact wide amounts, remove floating-point ingestion from Nexo, and enforce stable scale semantics. Scope transaction IDs and file deduplication by provider plus account instance.
- [ ] **7. Implement Crypto.com App CSV ingestion.** Start with token wallet, cash wallet, and card exports; retain event categories and account separation. Identify whether Earn and staking transfers are fully represented before declaring those positions reconciled.
- [ ] **8. Implement EVM address coverage.** Ethereum, BNB Smart Chain, and Polygon first; Cronos for Crypto.com on-chain holdings; then Arbitrum and zkSync as demonstrated by the catalogue. Add Base and Optimism for broader reuse. Include native transfers, internal value movements, token events, gas, failed/reverted transactions, block identity, and rollback of reorged observations.
- [ ] **9. Implement Kraken, then OKX.** Use account ledgers and independent snapshots, with trades/rewards/product detail as needed. Add CSV/archive ingestion for historical gaps rather than claiming that recent API history is complete.
- [ ] **10. Add legacy history and claims tracking.** Import retained statements, balances, claim evidence, distributions, and impairment decisions for historical lenders/exchanges. Promote above live connectors when the user has an unresolved claim or unexplained loss.
- [ ] **11. Restore offer rates, subscriptions, and liquidity state.** Distinguish APR/APY, tiers, reward currency, fixed term, renewal, unbonding, and withdrawable amount. Store unknown rates as unknown; replace Binance's placeholder APY only when a real rate source exists.
- [ ] **12. Implement remaining Crypto.com surfaces and relevant DeFi.** Exchange API/CSV is a separate adapter; on-chain wallet coverage is a chain/protocol path. Decode QuickSwap pools, Lido, and Rocket Pool after composite/share accounting; add Aave and Uniswap for wider coverage.
- [ ] **13. Implement YouHodler, CoinList, SwissBorg, HTX, and Uphold as justified by current use.** Start with verified user statement formats where account-history API access is not yet established. Follow with remaining native chains and the smaller personal providers below.
- [ ] **14. Implement Coinbase, then the missing major exchange batch.** Coinbase is registered in the export but has no exported nonzero product total. Raise it if currently used. Default expansion order: Bybit, Bitget, KuCoin, Gate, Bitstamp; see scope constraints below.
- [ ] **15. Restore reports and follow-up work.** Provider/product/asset rollups, expected and realized annual income, liquidity timeline, transfer matching, unresolved movements, and archived/researched account views.
- [ ] **16. Add fiat accounts, performance, and tax evidence.** Monzo first, then Wise/Revolut/N26 and other used banks; explicit reporting currency and FX; cost-basis lots, disposals, fees, cashflow-aware returns, and supporting records. Country-specific tax calculations require a separate jurisdiction decision and specification.
- [ ] **17. Add advanced structured products and remaining market coverage.** Dual-currency settlement, options/derivatives, vaults, additional DEXs, and long-tail exchanges only after core accounting and personal coverage are reliable.

## Providers already represented in the exports

Rows below are ranked within this table. Existing support is shown separately from new implementation work.

| Rank | Provider or family | Evidence and recommended first scope |
| --- | --- | --- |
| Existing | Nexo | Imported CSV and historical valuation already exist. Improve exact ingestion, debt semantics, and evidence-backed product metadata; the current CSV does not establish individual subscription identity. |
| Existing | Binance and Binance Earn | Spot/Funding/Flexible/Locked plus statements already exist. Expose normal import/sync, retain reconciliation, add individual Locked contracts and rate metadata. Margin, derivatives, loans, P2P, and other omitted products need separate scope. |
| 1 | Crypto.com | Signed-up umbrella and App/DeFi rows; Earn/card/Exchange relationships and CRO staking/unstaking in wallet rows. Build App CSV, then on-chain holdings, then Exchange API/CSV. Preserve transfers between these accounts. |
| 2 | MetaMask, Trust Wallet, ImToken, Exodus, Guarda, Argent, Atomic Wallet | Build address/chain connectors, with wallet labels and multiple addresses. Import one address once even if viewed in several apps. MetaMask has explicit BSC, Polygon, Ethereum, zkSync, and Arbitrum catalogue entries; ImToken includes DOT/ATOM and therefore also needs non-EVM chains. |
| 3 | Kraken | Signed up, historic nonzero product total, DOT wallet and MINA APY entry. Ledger/balance/history plus staking/Earn; preserve product-specific balances. |
| 4 | OKX and OKX Staking | Signed up with historic nonzero total; funding and multiple fixed/flexible term APY links. Funding/trading/Earn account separation and archive ingestion. |
| 5 | Historical recovery family | AAX, Hodlnaut, BlockFi, Celsius, FTX, Vauld, CoinLoan, Haru Invest, Bittrex, Nuri, Bitgrail, Pokket, and Hashflare appear in the catalogue or wallet view. Triage service status and user evidence individually. Start with generic legacy imports/claims rather than assuming a usable live API for every name. |
| 6 | YouHodler | Signed up and 15 APY-linked wallet rows, the most frequent provider in that column. Prioritize statements/observations, yield tiers, and loan data where applicable; confirm export/API accessibility before designing a live connector. |
| 7 | CoinList | Signed up; MINA duplicates, ALGO and USD wallets. Import sale allocations, vesting/staking, rewards, transfers, and balances with distinct subscriptions where evidence supports them. |
| 8 | SwissBorg | Signed up; BTC/ETH and CHSB premium/yield staking entries. Verify current asset naming and statement formats; retain premium qualification and staking state. |
| 9 | Huobi and Uphold | Both signed up with wallet history; Huobi has fixed Earn rows and expected rewards; Uphold has BAT. Resolve historical/current provider names explicitly, and preserve rewards plus funding movements. |
| 10 | QuickSwap, Lido, Rocket Pool, Alpha Homora, MetaPool, Cream | QuickSwap has paired pool-component rows; others appear as receipt/yield offers. Decode protocol/version/chain after shares, underlying exposure, and debt support. Prioritize confirmed held positions over researched offers. |
| 11 | Yoroi/Cardano, Polkadot.js/Substrate, NEAR, MultiversX, Cosmos, Cronos | Yoroi ADA; Polkadot.js DOCK and ImToken DOT; NEAR wallet/MetaPool; Elrond wallet/Maiar; ImToken ATOM; Crypto.com CRO staking. Implement chain readers and delegation/unbonding support rather than connectors tied to obsolete wallet websites. |
| 12 | Coinbase | Already in products, signed up, historic zero product total. Add Coinbase App transaction/account data and Advanced Trade fills; preserve legacy Pro history if the user has it. |
| 13 | Matrixport | Signed up and a BTC/USDC dual-currency wallet. Statements/positions first; contingent settlement requires the structured-product model. |
| 14 | AscendEX and Blockchain.com | Signed up with wallet evidence. Scope AscendEX as an exchange; distinguish Blockchain.com custodial accounts from its self-custody wallet addresses. |
| 15 | Gemini and other smaller CeFi | Gemini/Gemini Earn, Bitmart, Bitrue, Bitso/Bitso+, Mercatox, Nebeus, MyCointainer, MyConstant, CakeDefi, WhaleFin, Pillow, TryAnchor, and Nash appear. Validate current product/service identity and usage before a bespoke adapter. Exchange trading and historical Earn/claims are separate scopes. |
| 16 | Bridges and other DeFi | Connext/xpollinate, ZKSpace/ZkSwap, wallet.zksync.io, BloxStaking, IndexCoop, Radiant, Jones DAO, ETHMAXY, Stake DAO, Ribbon, Cega, Theta Nuts, Pods, Tap Finance, structure.finance, and Decommas appear. Generic chain ingestion first; protocol decoders follow demonstrated positions, versions, and historical evidence. |
| 17 | Fiat, managed investing, and research entries | Monzo, Revolut, Transferwise, N26, Shine, Finom, Paysend, Banc Sabadell, Soon/AXA, and Yomoni are present. Begin CSV/manual imports, then evaluate bank APIs/open banking. 3commas, CoinRule, Streak, and Mudrex may be orchestration/catalogue entries; reconcile their underlying exchange accounts to avoid duplication. Jump Crypto and Electric Capital are research entries. |

Some of these names are legacy labels. Preserve them as aliases; do not silently rename a historical product or assume its modern counterpart has identical assets and records.

### Verified access paths for the first connectors

- **Crypto.com App:** official guidance supports CSV exports for token wallet, cash wallet, and Visa card. It currently allows up to three years per report and downloads expire after 30 days. Retain generated files locally. The Exchange has its own API and separate statements. The Exchange API alone must not be presented as App/DeFi coverage. Sources: [App exports](https://help.crypto.com/en/articles/3438579-how-do-i-export-my-transaction-history-app), [Exchange API](https://exchange-docs.crypto.com/exchange/v1/rest-ws/index.html).
- **Kraken:** ledger entries carry asset, amount, fee, balance, and event type; current documentation provides pagination and wallet account selection. Staking/Earn suffixes require product-aware mapping. Supplement with trades, exports, and Earn allocations without counting the same movement twice. Source: [Kraken ledger reference](https://docs.kraken.com/api-reference/account-data/get-ledgers-info).
- **OKX:** the ordinary archived bills endpoint covers three months; a separate archive-generation path provides unified-account bills since February 2021, excluding the current quarter. Funding/Earn and any older history still require explicit coverage checks. Source: [OKX API guide](https://www.okx.com/docs-v5).
- **EVM wallets:** Etherscan V2 provides a common API across supported chains through `chainid`. Use chain RPC/indexers as replaceable data sources, not as the user's custody provider. Endpoint availability, paid tiers, internal transactions, token discovery, and archive access need per-chain checks. Source: [Etherscan introduction](https://docs.etherscan.io/introduction).
- **Historical recovery:** Hodlnaut identifies itself as in liquidation; Celsius states that its app closed in February 2024; FTX publishes recovery distribution guidance. These support prioritizing history/claim records over ordinary live account sync for those services. Other legacy names require their own status checks. Sources: [Hodlnaut liquidator update](https://www.hodlnaut.com/press/an-update-from-the-companys-liquidators-on-3-august-2026), [Celsius service closure](https://cases.stretto.com/celsius/contact-us/), [FTX distributions](https://support.ftx.com/hc/en-us/articles/33190623459092-General-Information-on-Distribution-Service-Providers).
- **Coinbase:** its App API exposes account-scoped transaction history with a read permission. Combine that with the applicable trading records and independent balances; do not assume trading fills cover all wallet activity. Source: [Coinbase transactions](https://docs.cdp.coinbase.com/coinbase-app/track-apis/transactions).

## Major providers missing from both exports

“Missing” here means absent from product display names/provider fields and wallet account/APY references, after reading both views. This is a coverage shortlist, not a market-share ranking. Names already present, including Coinbase, Kraken, OKX, Gemini, Lido, and Rocket Pool, are deliberately kept in the preceding table.

### Exchanges in recommended expansion order

| Rank | Missing provider | Initial scope and reason |
| --- | --- | --- |
| 1 | Bybit | Unified account transactions/balances plus Funding, deposits, withdrawals, and transfers. Broadens exchange coverage. Its Unified transaction log supports up to two years, so retain exports for older history and distinguish account generations. [Transaction log](https://bybit-exchange.github.io/docs/v5/account/transaction-log), [Funding history](https://bybit-exchange.github.io/docs/v5/asset/fund-history). |
| 2 | Bitget | Spot balances, bills, trades, transfers, deposits, and withdrawals; add Earn only with identified product data. Official spot bills support transaction groups and bounded date windows; evaluate unified/classic account generation separately. [Spot bills](https://www.bitget.com/zh-CN/api-doc/classic/spot/account/Get-Account-Bills). |
| 3 | KuCoin | Trading/Funding ledger, balances, trades, and exports. Existing price-feed use does not implement user accounts. The new unified-account ledger documents only seven days of queryable history; use the applicable classic/unified API and statement exports instead of assuming lifetime history. [Unified ledger](https://www.kucoin.com/docs-new/rest/ua/get-account-ledger), [Export guide](https://www.kucoin.com/support/14047696578841). |
| 4 | Gate | Spot account history and funding flows first. Add lending/Earn and derivatives only after their corresponding model capabilities. Check region/account-specific base URLs and API coverage. [Gate API reference](https://www.gate.com/docs/developers/apiv4/en/). |
| 5 | Bitstamp | Balances, user transactions, deposits, withdrawals, and fees. A useful additional fiat/crypto exchange path. Document timestamp/offset limits and older-history recovery. [Bitstamp API](https://www.bitstamp.net/api/). |
| 6 | Bitfinex and MEXC | Discovery backlog: establish actual user demand, read-only history endpoints, independent snapshots, export access, and retention before committing implementation. Promote if currently used. |

### DeFi and chain coverage in recommended expansion order

| Rank | Missing provider or capability | Initial scope and dependency |
| --- | --- | --- |
| 1 | Aave | Supplied assets, debt, collateral, interest indices, rewards, and liquidation events. Requires user liabilities and index/share accounting; generic token transfers are insufficient. [Aave tokenization](https://www.aave.com/docs/aave-v3/smart-contracts/tokenization). |
| 2 | Uniswap | Decode swaps and liquidity positions, starting with relevant deployed versions. V3 positions have NFT identity and two underlying tokens; principal and uncollected fees need separate valuation. [V3 position guide](https://developers.uniswap.org/docs/protocols/v3/guides/managing-liquidity/getting-started). |
| 3 | Morpho | Vault shares, underlying amounts, and lending positions where used. Reuse ERC-4626 conversions without treating every receipt share as one underlying unit. [Vault mechanics](https://docs.morpho.org/developers/earn/concepts/vault-mechanics/). |
| 4 | Compound | Supply, collateral, borrow, accrued interest, and liquidations. Compound III uses signed principal plus supply/borrow indices; implement by version. [Collateral and borrowing](https://docs.compound.finance/collateral-and-borrowing/). |
| 5 | Curve, then PancakeSwap | Additional pool/staking/fee coverage after composite positions. Curve's registry provides pool/coin metadata; select exact protocol versions and deployed chains before implementing. PancakeSwap is a discovery candidate for BSC coverage. [Curve registry](https://docs.curve.finance/developer/integration/meta-registry/). |
| 6 | Bitcoin address and descriptor coverage | The exports include BTC holdings but no explicit Bitcoin self-custody account. Add UTXOs, change detection, fees, addresses/descriptors, and historical transactions when the user has self-custody BTC. Wallet/app labels are metadata. |
| 7 | Solana wallet and staking coverage | SOL is present on exchanges, but no explicit Solana self-custody account appears. Add native SOL, SPL/Token-2022 assets, stake accounts, signatures/transactions, fees, and program decoding. A token-account query gives current token accounts, not complete history. [Solana token account RPC](https://solana.com/docs/rpc/http/gettokenaccountsbyowner). |
| 8 | Jupiter, Raydium, Orca, and Solana liquid staking | Discovery backlog after Solana ingestion and composite/share accounting. Prioritize actual positions and transactions; a swap router is not automatically a separately held account. |
| 9 | Pendle, Yearn, Convex, Beefy, Maker/Sky, and restaking protocols | Discovery backlog after time-dependent receipt tokens, maturity, liabilities, and protocol/version identity are stable. Select held products first. |

Ledger, Trezor, and Phantom are also absent by name, but should usually be supported through their chain accounts rather than bespoke balance-provider integrations. Base and Optimism are useful missing EVM networks, distinct from financial providers. A portfolio indexer can accelerate discovery, but should be optional and replaceable: storing results locally does not prevent a remote service from learning queried addresses.

## Requirements for every implemented connector

- [ ] Declare supported account types, products, regions, history boundaries, and unavailable records. A balance-only connector is valid if labelled as such.
- [ ] Use account-scoped stable record IDs, preserve raw evidence locally, and version parsing/mapping. Keep original file formats and UTC/source timezone information.
- [ ] Parse amounts exactly, preserve asset scale and chain identity, and distinguish fees/rewards/interest/transfers from principal movements.
- [ ] Persist source event semantics, external references, and import provenance alongside the normalized ledger; a generic amount alone cannot reconstruct income or tax meaning.
- [ ] Resume pagination and incremental sync with overlap; reconcile duplicate API/CSV views while preserving legitimately repeated identical movements.
- [ ] Fetch independent balances/positions at a stated cutoff and retain discrepancies. Mark unmatched or partially decoded events unresolved rather than inventing balancing entries.
- [ ] Match transfers between owned accounts without creating income or counting both as fresh deposits. Keep network/withdrawal fees and pending bridge movements explicit.
- [ ] Support offline replay and rate-limit recovery. Share provider pacing where needed; keep credentials out of fixtures, logs, and documents.
- [ ] Verify meaningful fixtures: overlapping reimport, history gaps, same-asset positions, multiple account instances, fees in another asset, rewards, locked balances, and negative liabilities where applicable.

## Other work in descending priority

1. **Coverage and reconciliation reports:** identify complete/partial/current-only accounts and dated unresolved differences. This determines whether any portfolio number is trustworthy.
2. **Yield and liquidity reporting:** expected income by currency, realized rewards, net yield after fees, maturity/unbonding timeline, and accessible cash. Historic APYs cannot be presented as current offers.
3. **Transfer follow-up and recovery notes:** replace spreadsheet notes with linked evidence and resolution status; preserve claims and distributions without classifying recoveries automatically as ordinary yield.
4. **Valuation provenance and reporting currency:** configurable GBP/EUR/USD; price quality/staleness, missing-price state, FX evidence, and disclosed stablecoin/receipt valuation assumptions.
5. **Cashflow-aware performance:** distinguish deposits/withdrawals from returns, capital gains, and income before offering performance comparisons.
6. **Tax evidence:** retain acquisition/disposal lots and source records; select jurisdiction and accounting rules before implementing tax-specific calculations.
7. **Broader financial accounts and UI:** bank/broker/pension/debt models and a persistent account dashboard once the core account workflow is usable.
8. **Yield discovery and comparisons:** refresh offer data with timestamps, eligibility, liquidity, and risk context. Prioritize tracking held products before recommendations or catalogue scraping.

## Implementation preparation

- [ ] Align the persistence, Nexo, historical valuation, and Binance proposal task states with actual source before creating overlapping work.
- [ ] Prepare focused OpenSpec changes for account metadata/manual migration, economic roles/valuation, yield/subscription terms, chain identity/amounts, and each connector batch.
- [ ] Define connector acceptance criteria and collect anonymized format fixtures before implementing. A provider present only as a research entry needs no live connector until it solves a real coverage gap.

API and export facts above were checked against official sources on 10 October 2026. Unverified long-tail providers remain discovery tasks; regional permissions and exact account-generation coverage should be checked when their implementation starts.
