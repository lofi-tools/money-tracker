# Public token metadata sources

Researched and fetched 13 October 2026. This is basic metadata evidence, not a token endorsement, risk rating or guaranteed mapping catalogue. The [implemented contract](../spec/token-metadata.md) defines the cache and presentation boundary.

## Source comparison

| Source | Access and coverage | Decision |
| --- | --- | --- |
| [CoinGecko contract metadata](https://docs.coingecko.com/demo/reference/coins-contract-address) | Free Demo API with a key; listed contracts can provide name, symbol, images, platform decimals and market ID. Existing environment key support is reused; keyless access is best effort and availability is not guaranteed. | Supplemental exact-contract metadata and market ID; never search by ticker. Failed requests do not establish that a coin is unlisted. |
| [Blockscout APIs](https://docs.blockscout.com/devs/apis), [token schema](https://github.com/blockscout/blockscout-api-v2-swagger/blob/main/swagger.yaml) | Public per-instance token endpoints have explorer coverage including old/unlisted contracts; availability/access limits vary by instance. Authenticated universal APIs have separate access plans. | Primary keyless Ethereum lookup at `eth.blockscout.com/api/v2/tokens/CONTRACT`. Validate returned address and ERC-20 type. |
| [Etherscan token info](https://docs.etherscan.io/api-reference/endpoint/tokeninfo) | Token-info endpoint is PRO, Standard plan or above. Free account history support does not imply free metadata access. | Not required for this metadata slice. |
| [Uniswap token lists](https://github.com/Uniswap/token-lists) | Public JSON lists carry chain, address, name, ticker, decimals and optional logo URI. Membership/coverage is list-dependent and a list is not authoritative decimal evidence. | Useful later bulk fallback; not fetched automatically in the initial implementation. |
| [ERC-20](https://eips.ethereum.org/EIPS/eip-20), [Ethereum JSON-RPC](https://ethereum.org/developers/docs/apis/json-rpc/) | Name, symbol and decimals are optional contract methods; RPC access can be free, throttled, or omit archive state. RPC has no standard token icon or market ID. | Check chain ID and fetch at an observed explicit block. Use for decimal evidence and missing name/symbol fallback; never reinterpret stored ledger scale. |

The practical strategy is Blockscout → supplementary CoinGecko → RPC evidence/fallback, with independent nullable fields, retained cache and explicit failures. Network acquisition is explicit; startup and portfolio rendering are offline.

## Observed selected-portfolio coverage

The selected `.cache/data/portfolio.duckdb` was backed up before the migration/fetch. Twelve currently held tracked Ethereum token identities received cache records: eleven names, nine cached raster icons, seven successful current RPC decimal values, and two verified CoinGecko IDs. These counts describe this capture, not service-wide coverage. Some older contracts have explorer metadata but no current RPC return. One contract remains unnamed; its raw evidence is retained and it appears in Dust.

The explicitly requested `0x0cf0ee63788a0849fe5297f3407f701e122cc023` resolves to **Streamr XDATA / XDATA**, with an offline icon, exact-contract CoinGecko ID `streamr-xdata`, and explorer/RPC decimals of 18 matching its protected scale. This old token is not silently mapped to another Streamr contract or renamed ticker.

A read-only `EXCEPT ALL` comparison verified all 31 preexisting tables against the backup exactly. Only the new metadata table changed. This includes ledger effects/scales, prices, imports, raw/source metadata, account jurisdiction, manual overrides, exclusions and saved reconciliation evidence. The review JSON was exported from a validated temporary copy of the selected DB.

## Reviewed navigation relationships and icons

Underlying groups use explicit identities from provider mapping plus [Circle's USDC addresses](https://developers.circle.com/stablecoins/usdc-contract-addresses), [Tether's supported protocols](https://tether.to/en/supported-protocols/), and Uniswap's [Ethereum](https://developers.uniswap.org/docs/protocols/v3/deployments/v3-ethereum-deployments) / [Arbitrum](https://developers.uniswap.org/docs/protocols/v3/deployments/v3-arbitrum-deployments) WETH deployment records. These are display relationships, not guaranteed exchange parity; valuation keeps each token's own price. The same metadata resolution recognizes reviewed native currencies and exact Circle contracts without extending network acquisition beyond Ethereum.

Main-asset colour SVGs come from [Cryptocurrency Icons](https://github.com/spothq/cryptocurrency-icons) at a pinned commit under CC0, with an official [NEAR documentation](https://github.com/near/docs) mark extracted from its wordmark for small-size rendering. Source commits, licenses and adaptation notes live beside the bundled files. The Polygon icon uses the upstream Polygon/MATIC brand mark for the reviewed native POL presentation. Fiat icons for Nexo fiat representations indicate currency denomination and do not imply an issuer logo or risk equivalence.

Dynamic token icons remain bounded cached PNG/JPEG; trusted bundled SVGs are a separate code-reviewed asset set. No remote SVG is injected into app markup and no broader CSP is needed.

## Verification completed

Scoped checks passed: 24 core library tests, 75 deterministic application library tests, 16 portfolio projection tests, 3 desktop Session unit tests, 4 desktop portfolio integration tests, TypeScript typechecking, frontend production build, formatting/diff checks and 2 review-browser acceptance tests. Private/live tests remain opt-in. The existing desktop refresh schedule assertion was updated to the previously implemented missing-price-only launch bootstrap and supplied an empty synthetic acquisition result; provider refresh-on-open remains disabled. macOS proxy initialization required running that integration lane outside the filesystem/network sandbox. The final web preview was inspected at the user's 687px-wide viewport, including SVG icons, underlying grouping, Streamr cached facts and readable identities.

Later review refinements label the shared unknown/dust bucket **Others / dust**, use stored provider display names with brand defaults, and omit the sole uncustomized legacy Nexo container from desktop rows. A Session regression verifies direct provider/holding nesting, retained original account references, unchanged figures, and preservation of account/provider display choices. Native currency facts can additionally be reviewed against the ethereum-lists chain registry for Ethereum mainnet and Arbitrum One; both are ETH.
