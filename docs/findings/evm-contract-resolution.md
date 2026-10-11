# Ethereum contract resolution and EVM source access

Evidence checked 11 October 2026. This records contract identity and limitations, not a declaration that every account reconciles. Wallet addresses and API keys are omitted. [The adapter contract](../spec/evm-wallets.md) and [master todo](../todo/master-todo.md#9-next-connectors-evm-wallets-then-solana-then-near) own implemented behavior and remaining work.

## SafePromo: a promotional contract, not a normal transfer ledger

Contract `0x31a240648e2baf4f9f17225987f6f53fceb1699a` is named **SafePromo** in [Blockscout's verified contract source](https://eth.blockscout.com/api/v2/smart-contracts/0x31a240648e2baf4f9f17225987f6f53fceb1699a). Its displayed name is “New ICO 2018 (https://safe.ad) - 1 Year Gift”, with a URL-like symbol. Those strings are untrusted token metadata, not endorsements or links the app should open automatically.

The verified source's `balanceOf` always returns 777 for any address; ordinary token transfers/allowances are disabled. The promotion function emits Transfer-shaped logs to recipients. There is no decimals getter. RPC confirmed an empty decimals response both at the known event block 5,418,302 and at the frozen cutoff, and returned 777 raw balance units. Blockscout also reports no decimals.

The verified deployed bytecode matches the cutoff RPC bytecode exactly. SHA-256 of runtime bytes is `87b628c1cc1a0456d1ffb1b84ef5832ad1640dcb91f631a893cd6ac180be754f`; SHA-256 of the returned UTF-8 source is `278d2cb99679df3e3bdb3daad2a889a598259daed8898aa82a03dd25bbf6fc8c`.

Identity is resolved, but a financial asset mapping is intentionally withheld. Assuming 18 or zero decimals would invent a protected asset scale; matching a promotional event to a constant balance would not establish an economic holding. The adapter retains raw balance/decimals evidence and an unresolved-scale warning. The user subsequently authorized a reviewed spam exclusion. [Account-scoped tracking reviews](../spec/tracking-exclusions.md) now record this decision with the source/runtime hashes, preserve the raw snapshot in the DB, and remove SafePromo from financial totals, earnings eligibility and tracked reconciliation coverage. No scale or priced holding is invented; other contracts are not automatically excluded. Applying the review to the saved Ethereum account and replaying twice offline confirmed persistence: native currency and 14 tracked ERC-20 quantities still match exactly, while INS Promo retains its 777-versus-zero failure.

## INS Promo: historical decimals resolved, destroyed contract

Contract `0x52903256dd18d85c2dc4a6c999907c9793ea61e3` is **INSPromoToken / INS Promo (INSP)**. [Etherscan's contract record](https://etherscan.io/address/0x52903256dd18d85c2dc4a6c999907c9793ea61e3#code) has verified source, fetched through its authenticated `getsourcecode` API. Source explicitly declares decimals of zero, maintains token/virtual balance logic, and includes an owner-only destruction function.

Archive RPC confirmed decimals of zero and balance 777 at the recorded transfer block 4,634,973. At the frozen cutoff, decimals/balance calls return no data and `eth_getCode` returns `0x`.

[Destruction transaction](https://etherscan.io/tx/0x54ad1a3d34954fb92919534abb18dda3d926f583b3c703be16b83c6d054f7987) succeeded at block **4,794,689**, targets this contract and invokes selector `0x41c0e1b5`. Independent RPC confirms 2,675 deployed bytecode bytes immediately before that block and no deployed code at the end of it. Raw transaction, receipt and before/after code evidence are retained locally under `.cache/evm-evidence/`.

The generic resolver now recovers scale zero from a canonical historical RPC call, records that block/hash and observes zero only after proving code absence at the cutoff. Replay books the original movements without an invented adjustment: **ledger 777, current observation 0, difference −777**. This is a known destroyed-contract lifecycle/accounting gap rather than unknown decimals. Strict reconciliation still fails. An explicit provenance-backed retirement/loss model is needed; disappearance of code alone must not manufacture a balancing transaction or erase history.

## Public indexers and RPC findings

| Source | Polygon | Arbitrum One |
| --- | --- | --- |
| Original per-instance Blockscout APIs | HTTP 403 challenge | HTTP 403 challenge |
| Blockscout universal API, no key | HTTP 402; key/payment required | HTTP 402; key/payment required |
| Blockscout universal API, supplied key | HTTP 402; response requires Builder, Business or Pro plan | Transactions endpoint HTTP 200; rapid additional requests hit 429 |
| Etherscan V2, supplied key | History acquisition works | History acquisition works |
| Routescan keyless Etherscan-compatible endpoint | Reports chain not supported | Reports chain not supported |

Use the documented universal Blockscout shape `https://api.blockscout.com/CHAIN/api/v2/...`, with `BLOCKSCOUT_API_KEY` attached only to that service's discovery requests. [Blockscout API reference](https://docs.blockscout.com/api-reference/mud/list-of-mud-world-systems) demonstrates the chain-qualified universal base. Endpoint-specific plan access is checked from actual responses rather than assumed from possession of a key. No subscription upgrade or X402 payment was made.

With both keys configured, Polygon/Arbitrum default to the working Etherscan V2 history path. An explicit `--indexer https://api.blockscout.com/42161` selects Blockscout, with paced requests and bounded throttle retries. This preserves a choice of public providers; it does not claim a working unrestricted keyless indexer for either chain.

Polygon's former publicnode RPC returned null for the oldest normal transaction at block 14,572,069. Public `https://polygon.drpc.org` returned the exact requested transaction; it is now the default. Transaction, receipt, canonical header and pinned-balance verification still apply. Full live Polygon reconciliation is evaluated separately from this endpoint-availability check.

## Arbitrum Classic fee evidence and remaining coverage

The first Etherscan capture contains 41 transaction hashes, seven token contracts, and a native difference of **−87,468 wei**. One outgoing transaction is from Classic block 17,630,496. Nitro's receipt gives `gasUsed=417680` and `effectiveGasPrice=223747538`; their product is 93,454,871,671,840 wei. Blockscout's “actual” fee field gives that same product, so it is not an independent correction for this legacy fee.

[Arbitrum's troubleshooting guide](https://docs.arbitrum.io/build-decentralized-apps/troubleshooting-building) explains that Classic receipts represented through Nitro can use a rounded effective gas price; exact fees require the four-component `feeStats` from a Classic node. Classic ended at the Nitro upgrade block 22,207,818. The default public endpoint does not expose the probed legacy `arb_getTransactionReceipt` / `arb_getTxReceipt` methods. [Archive-node documentation](https://docs.arbitrum.io/run-a-node/more-types/run-archive-node) describes separate Classic state support.

The Classic rounding explanation is consistent with the difference, but the adapter has not obtained exact feeStats proving its amount. The ledger difference must not be accepted as rounding tolerance or filled by a balancing entry. Five token contracts initially had RPC execution/archive errors; a later same-block decimals probe succeeded for one, so rechecking frozen evidence is appropriate before classifying those tokens as unsupported. JSON-RPC throttles now receive bounded retries and safe error-code diagnostics.

## Final saved captures and blockers

Polygon acquisition through Etherscan and public dRPC completed at block **95,344,840**: 125 transactions, 45 readable token contracts and one unresolved contract. Native quantity matches exactly, and **41 of the 45** readable tokens match their receipt-ledger sums. Four contract-specific discrepancies remain:

| Contract | Untrusted reported symbol | RPC minus receipt-ledger units |
| --- | --- | --- |
| `0x474532388a5c2373dd1584beb636157a533f8dac` | PolyDough | `7675081453650042` |
| `0x81067076dcb7d3168ccf7036117b9d72051205e2` | DxDex.io | `-800000000000000000000000` |
| `0xaa1a12d10df6e029866adcda26b0557aab417b99` | UЅDС (contains non-ASCII lookalike letters) | `258592700` |
| `0xe4fb1bb8423417a460286b0ed44b64e104c5fae5` | Zepe | `-750000000000000000000000` |

These values are raw contract units, not dollars or estimates of collectible value. The lookalike symbol never maps to canonical USDC. Etherscan returned no verified source for that contract or Zepe during this investigation. Different receipt/RPC quantities require contract-specific evidence and accounting/classification; suspected reflection or promotional behavior is not yet an implemented mapping. The unresolved Polygon contract `0x32898be3deab4d9d25331b980e77cd5e35cbb265` fails its `balanceOf` call with RPC execution error code 3 at the saved cutoff.

Arbitrum's saved capture at block **513,819,105** has 41 transactions, two readable matching token contracts and five unresolved contracts. Rechecking the original block still gives token-getter execution failures with code 3 for `0x0df5dfd95966753f01cb80e76dc20ea958238c46`, `0x107583adaa37dfd1cc0bf577183bf91351d07413`, `0x48a29e756cc1c097388f3b2f3b570ed270423b3d`, `0x6aed705a1e8e7be9a3965743cbdc35fc9252d17a`, and `0xab04c0841f39596c9f18a981a2bd32f63ab7a817`. A successful decimals probe does not make a reverting balance getter a valid quantity observation. Four final Arbitrum failures are explicitly retained as `balance_error` beside successful decimals responses. The `0x6aed…2d17a` contract fails the initial decimals call before reaching balance probing; its safe RPC code remains in the unresolved diagnostic. Provider error text is not echoed where it could expose credentials.

The adapter explicitly withholds exact fee coverage for outgoing Arbitrum Classic transactions until feeStats mapping exists, even if aggregate balances happen to match. The current native difference remains −87,468 wei; no tolerance or adjustment was introduced. After applying the reviewed SafePromo exclusion, Ethereum remains blocked by destroyed INS Promo lifecycle handling (ledger 777 versus observation zero). Polygon remains blocked by the four discrepancies and one unreadable contract. Arbitrum remains blocked by Classic fees and five unreadable contracts. These are recorded before any move to the next connector; account/snapshot identities, existing data and strict comparisons are preserved.
