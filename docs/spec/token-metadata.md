# Token metadata and underlying-asset navigation

Implemented on 13–14 October 2026; approved for commit on 14 October. Sources: [acquisition and display resolution](../../libs/lib-app/src/token_metadata.rs), [cache storage](../../libs/lib-core/src/token_metadata.rs), [reviewed display groups](../../libs/lib-app/src/asset_groups.rs), [portfolio projections](../../libs/lib-app/src/portfolio.rs), and [desktop presentation](../../apps/desktop/src/asset-presentation.ts). Research and live coverage are in [source findings](../findings/token-metadata-sources.md).

## Cache and identity

Migration `011_token_metadata` appends a separate typed JSON payload table keyed by exact asset ID. The initial fetch covers currently nonzero, tracked Ethereum ERC-20 ledger holdings in one selected portfolio, deduplicated by exact contract. Excluded assets, zero holdings, unsupported chains, native currencies and unknown-scale contracts are not acquired by this operation. It sends token contract addresses, never wallet addresses or quantities, to the chosen public services. Display grouping has a separate small reviewed native/contract catalogue.

`TokenMetadata` stores asset ID, chain ID, lowercase contract, nullable name/ticker/icon URL, cached PNG/JPEG image data, source decimals, current RPC decimals, verified CoinGecko ID, nullable last successful fetched time, response evidence with per-source fetched times, and explicit warnings. CoinGecko IDs require an exact Ethereum platform/contract match; ticker matching never establishes an ID, price mapping or economic equivalence. Cached IDs are evidence for later pricing work; this task does not automatically start pricing these tokens.

The cache is independent of `AssetDisplay`. Existing names/tickers take precedence, then fetched metadata, then explicitly reviewed native/contract labels. Refresh never writes manual displays or their evidence. Cached responses/images survive source failures; their timestamps remain unchanged until a successful response. Missing fields and failures stay explicit. Optional on-chain metadata and block-number evidence are display evidence, not independent balance verification or historical ledger-scale evidence.

Neither acquisition nor cache persistence registers/changes scales, rescales movements, edits prices, adds aliases, changes exclusions, imports account data, or relaxes reconciliation. A source/RPC scale mismatch is retained as a warning beside the protected ledger scale. An absent decimal is different from zero. ERC-20 metadata fields are optional; destroyed or unusual contracts can have explorer facts without current RPC facts.

## Explicit acquisition and offline review

Close other DB writers and back up the selected database before acquiring. The production database default remains unchanged; development uses `APP_DATA_DIR` or an explicit example argument.

```sh
cargo run --locked -p lib-app --example refresh_token_metadata -- .cache/data/portfolio.duckdb local:default
cargo run --locked -p lib-app --example check_metadata_preservation -- .cache/data/portfolio.duckdb PATH_TO_BACKUP.duckdb
cargo run --locked -p money-tracker-desktop --example export_preview -- .cache/data/portfolio.duckdb .cache/portfolio-review.json
```

The operation uses Blockscout Ethereum token info, optional CoinGecko contract lookup, and Ethereum RPC metadata at an explicit observed block after checking chain ID. Requests have time/response limits and pacing; source failures preserve the cache. Name/symbol fallback handles standard ABI strings and legacy bytes32. There is no automatic metadata schedule or desktop acquisition button in this slice. The frozen review export is regenerated from a temporary copy of the selected DB; it never substitutes a fixture for that DB.
