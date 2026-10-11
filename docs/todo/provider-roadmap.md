# Provider discovery inventory

Updated 11 October 2026. Delivery priorities and all actionable checklists now live in the [provisional master todo list](master-todo.md). Kraken's [account-scoped Spot foundation](../spec/kraken.md) is implemented alongside Nexo/Binance, with one private account capture reconciled and broader real-account/product coverage still outstanding. The [EVM native/ERC-20 foundation](../spec/evm-wallets.md) is implemented; complete live five-chain reconciliation and protocol coverage remain outstanding. Integration order continues with **EVM completion → Solana → NEAR**.

Confirmed chains are Ethereum, Polygon, BNB Chain, Arbitrum, zkSync, Solana, and NEAR. Public indexers are the default, with RPC-node overrides. Use the Airtable exports to identify wallets/positions, not to impose their complete data model or treat researched offers as held investments.

The tables below retain discovery evidence and access-source research from 10 October 2026. Their numeric ranks reflect the older export-based shortlist, **not current implementation order**. Recheck API, service, region, and history facts when implementing a connector. No bespoke connector is required merely because a provider appears in an export.

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

## Connector completion criteria

Use the existing-provider documentation and Kraken sections of the [master checklist](master-todo.md), then apply the same evidence, mapping, exact-amount, import/replay, independent-balance, and provider-only diagnostic requirements to each connector. Pricing support alone does not establish account-history support.
