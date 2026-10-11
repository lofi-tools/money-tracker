# EVM wallet quantities and reconciliation

Implemented foundation: [acquisition client](../../libs/evm-client/src/lib.rs), [ledger adapter](../../apps/cli/src/adapters/evm.rs), and `evm` CLI command. It tracks native currency and discovered ERC-20 contracts. Protocol position interpretation, underlying exposures, pricing aliases, spam exclusions and yield classification are still proposed work in [the master todo](../todo/master-todo.md#9-next-connectors-evm-wallets-then-solana-then-near).

## Connecting a wallet

Use a public address, never a seed phrase or signing key. Account metadata is persisted in the selected portfolio DB; subsequent commands select that saved identity. The user-supplied wallet has been registered in `.cache/data/portfolio.duckdb` on all five chains. Addresses are not hardcoded in source or documentation.

```sh
cargo run -p cli -- --data-dir .cache/data --cache-dir .cache evm --address YOUR_PUBLIC_ADDRESS --register-only
cargo run -p cli -- --data-dir .cache/data evm --list
cargo run -p cli -- --data-dir .cache/data --cache-dir .cache evm --account SAVED_ACCOUNT_ID
cargo run -p cli -- --data-dir .cache/data --cache-dir .cache --offline evm --account SAVED_ACCOUNT_ID
cargo run -p cli -- --data-dir .cache/data --cache-dir .cache evm --account SAVED_ACCOUNT_ID --resolve-tokens
```

`--chain 1,137,56,42161,324` is the default for registration/address-based runs. `--portfolio` selects the portfolio; `--rpc`, `--indexer` and `--snapshot` require a single selected account/chain. `--refresh` explicitly replaces a frozen cache after successful acquisition; it conflicts with global `--offline`. Listing and registration require no network access. Global flags precede `evm`.

| Chain ID | Network | Default RPC | Discovery source |
| --- | --- | --- | --- |
| 1 | Ethereum | `https://eth.drpc.org` | `https://eth.blockscout.com` |
| 137 | Polygon | `https://polygon.drpc.org` | `https://polygon.blockscout.com` |
| 56 | BNB Chain | `https://bsc-rpc.publicnode.com` | Etherscan V2; requires `ETHERSCAN_API_KEY` with BNB access |
| 42161 | Arbitrum One | `https://arb1.arbitrum.io/rpc` | `https://arbitrum.blockscout.com` |
| 324 | zkSync Era | `https://mainnet.era.zksync.io` | `https://zksync.blockscout.com` |

The defaults are replaceable public services, not guarantees of availability or free archive access. The original Polygon/Arbitrum indexers return HTTP 403 challenges. For these chains, configuration prefers `ETHERSCAN_API_KEY`, then `BLOCKSCOUT_API_KEY`, then the keyless per-instance default. Etherscan history access was verified with the supplied key. Blockscout universal API requires plan-specific chain access; the supplied key accesses Arbitrum, but Polygon returns a plan requirement. See [the acquisition findings](../findings/evm-wallet-inventory.md). `--indexer` accepts a compatible Blockscout v2 base URL. Use `https://api.blockscout.com/CHAIN` for the authenticated universal API; `BLOCKSCOUT_API_KEY` is sent only to that service, with paced requests and bounded retries. The exact endpoint `https://api.etherscan.io/v2/api` selects Etherscan with the environment key. BNB chain access may depend on the user's Etherscan plan. Credentials and full request URLs are excluded from transport error messages. Queries reveal wallet addresses to the chosen indexer/RPC services. Chain 324 means Era; old zkSync Lite activity requires another connector.

## Source evidence and frozen acquisition

Blockscout v2 address streams provide paginated normal transactions, token transfers, internal transactions and current ERC-20 inventory. Nonadvancing/repeated cursors, missing completeness markers and malformed responses fail acquisition. Etherscan V2 uses `txlist`, `tokentx`, and `txlistinternal`, bounded to the cutoff and ascending pages of 1,000; reaching 10 full pages fails rather than truncating history. It currently lacks a separate current token inventory query. Pagination contracts are documented in [Blockscout's API schema](https://github.com/blockscout/blockscout-api-v2-swagger/blob/main/swagger.yaml) and the [Etherscan account API](https://docs.etherscan.io/api-reference/endpoint/txlist).

Ethereum pre-Byzantium receipts have a state root rather than status. For those transactions only, explicit indexer success/failure evidence is retained and required; absent outcomes fail mapping. Modern receipts require their RPC status. [EIP-658](https://eips.ethereum.org/EIPS/eip-658) describes the receipt change.

The RPC chain ID must match the account. The cutoff is head minus `--confirmations` (64 by default). For every discovered transaction, RPC supplies the transaction, receipt and canonical block header. Hashes/block numbers must agree; receipt log indices provide event identity. Native `eth_getBalance` and ERC-20 `balanceOf` calls independently observe quantities at that same cutoff block. The cutoff hash is checked again before caching. [Ethereum's JSON-RPC reference](https://ethereum.org/developers/docs/apis/json-rpc/) defines these fixed-block calls.

Decimals come from the contract at the cutoff. If the optional `decimals()` method returns no data, the resolver checks its earliest recorded receipt block against a canonical header, retains the historical response/block/hash and accepts an explicit historical scale, including zero. Conflicting historical/indexer scales fail. If no historical scale is available, explicit indexer decimals may be used with a warning; a ticker never establishes scale or asset identity. A missing balance response counts as zero only when RPC confirms no deployed contract code at that block. Otherwise the contract remains unresolved with a reason. [ERC-20](https://eips.ethereum.org/EIPS/eip-20) makes decimals optional.

Snapshots include account/chain/cutoff identity, exact balances, successful token metadata, raw transactions/receipts/internal traces, warnings and unresolved contract diagnostics. Per-contract RPC resolution evidence is retained under `token_evidence`, including raw responses and historical scale provenance. Completed captures are atomically written to `evm/SHA256(account_id).json` under the cache directory. Cache-first/offline replay needs no API key or network. `--resolve-tokens` rechecks unresolved contracts at the existing snapshot cutoff, validates the saved account and canonical hash before/after, and atomically updates metadata evidence without advancing history or balances. It requires an existing cache and online access, conflicts with `--refresh`/`--offline`, and does not require an indexer key. Missing decimals remain unresolved; contract code absence never inserts a ledger write-off.

There is no partial-page resume yet; interrupted acquisition restarts. Caches contain public but personally identifying address/activity data and belong outside Git.

## Ledger mapping and asset sums

Account identity is `eip155:CHAIN:lowercase-address`; provider is `evm`. Native assets use `eip155:CHAIN/native`; tokens use `eip155:CHAIN/erc20:lowercase-contract`. Labels and equal tickers do not merge assets, accounts or networks. This deliberately avoids guessing a price/provider alias or DeFi underlying asset.

All quantities use arbitrary-precision integer smallest units; native scale is 18, token scales are protected by existing core persistence rules. No float conversion or bounded integer amount arithmetic occurs.

| Evidence | Booked effects |
| --- | --- |
| Successful native transaction value | Debit sender wallet and credit recipient wallet when tracked; external counterparties do not contribute to owned balances |
| Failed transaction | No value/token movement; gas is still charged to the sender |
| Receipt gas | `gasUsed * effectiveGasPrice`, plus explicit `l1Fee` where provided; outgoing Arbitrum Classic history explicitly withholds exact fee coverage until feeStats is supported |
| Successful ERC-20 Transfer receipt log | Contract-qualified asset movement; distinguish logs by index, preserve multiple events, self-transfers net to zero |
| Successful internal value transfer | Native movement under parent hash/trace index; skip the identical index-zero root movement already booked from the transaction |
| Independent RPC balance | Observation only; never a balancing ledger entry |

A single account-scoped `transaction:HASH` contains the transaction's effects; different chains/wallets cannot collide. Swaps appear as their actual incoming/outgoing transfers and gas, without a speculative swap or earnings classification. NFT logs are outside scope. Imports atomically persist effects, observations and an account-specific completed snapshot fingerprint. Repeat imports skip completed snapshots; precision conflicts or malformed/duplicate events roll back the batch. Existing Nexo/Binance records and migration behavior are unchanged.

Reconciliation compares cumulative owned ledger positions through the cutoff timestamp with independent RPC quantities, including zero balances for historic assets absent from observations. It prints ledger, RPC and difference in smallest units. A nonzero difference fails. Outgoing Arbitrum Classic transactions also fail exact fee coverage until legacy feeStats mapping exists, even if aggregate quantities happen to match; unresolved contracts also prevent a successful full-coverage result even if known balances match. Successfully observed positions can remain available for investigation, but a partial snapshot is not a verified portfolio total.

## Verification and boundaries

```sh
cargo test -p cli adapters::evm::tests::
cargo test -p evm-client
cargo test -p cli adapters::evm::private_tests::latest_amount_after_cached_wallet -- --exact --ignored --nocapture
```

The ignored provider-only diagnostic reads the saved Ethereum account from `.cache/data/portfolio.duckdb` by default, with `EVM_ACCOUNT_ID` selecting another saved account. `EVM_RPC_URL` and `EVM_INDEXER_URL` override its endpoints. It uses the same `.cache/evm` frozen snapshot as the CLI; missing caches trigger public acquisition. The Nix `evm` wrapper runs this diagnostic.

Synthetic tests verify values above i128, failed gas, internal/root deduplication, stable replay, mixed chains/accounts, scale-conflict rollback, persisted mismatches and unresolved coverage. A local HTTP fixture verifies paginated discovery and independent calls pinned to one block. Tests also cover historical zero decimals, metadata reorg/conflict rejection and an explicit Classic fee coverage guard, absent-code balances without synthetic write-offs, raw evidence for unknown scales, authenticated universal pages, and JSON-RPC throttle/error handling. These establish implemented behavior, not live coverage on every chain.

See [the contract-resolution findings](../findings/evm-contract-resolution.md) for SafePromo, destroyed INS Promo, provider access and Arbitrum Classic fee blockers.

Known limitations: indexer omissions can prevent discovery entirely; receipt verification cannot discover a contract the indexer never mentions. Rebasing/reflection tokens, protocol accounting, NFTs, unusual fee/payment schemes and special chain effects can require additional mappings. There is no automatic reorg rollback: refreshing does not delete disappeared old transactions. The core history query is timestamp-based, so multiple L2 blocks sharing a timestamp are not an exact arbitrary historical block boundary after later imports. Public RPCs must serve historical transactions and state; throttling, missing archive data and errors are visible failures. Confirmations reduce reorg exposure but are not a finality guarantee. No main portfolio price/net-worth integration is claimed until contract-qualified pricing mappings and coverage rules are added.
