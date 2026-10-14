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

Token names/tickers and icons appear in asset navigation and allocation labels. Selected Ethereum tokens offer expandable cached metadata with full plain contract, chain, source URLs, timestamp, protected/source/RPC decimals, market ID, and missing/conflict warnings. Read-only rendering makes no network requests: dynamic images are cached bounded PNG/JPEG, fetched only from an HTTPS host allowlist without redirects. Arbitrary remote SVGs, scripts, event attributes and external links are not embedded. Main assets use audited bundled colour SVGs with pinned upstream source/license records. The app CSP stays unchanged.

## Unresolved identities and direct underlying groups

The user's review requested that unresolved identities and dust share one meta-asset labelled **Others / dust** until resolved, and that full `eip155:` identifiers never appear as visible asset labels/quantities. An EVM asset lacking any sourced/local/reviewed name or ticker is therefore Dust even if its value is known or a local Significant choice exists. This is presentation classification only; amounts remain in all portfolio/provider totals. The saved Dust/Significant choice is preserved and applies again after resolution. Liabilities are never dust. The 14 October refinement also places named assets with unknown prices in Others / dust. Labels use chain names and short contract suffixes; exact identity fields and source references remain in the DTO/database for operations and audit.

`Selection::Exposure` groups only explicitly reviewed identities. The initial catalogue covers the held USD/USDT/USDC/USDX/xUSD family, EUR/EURX, GBP/GBPX, native ETH on Ethereum/Arbitrum/zkSync, BNB and Polygon native currencies, Circle's exact supported-chain USDC contracts, Ethereum USDT, and Uniswap-documented Ethereum/Arbitrum WETH. New stablecoins, bridges and wrappers require explicit relationship evidence; a token claiming a familiar ticker is not grouped. Staking derivatives, LP tokens, rebasing tokens and merely correlated investments are outside this mapping.

Navigation aggregates each group's existing financial contributions once and retains the original quantities by exact asset and role in its details. Every token uses its own quote; a stablecoin depeg or wrapper premium is never replaced with a synthetic 1:1 price. Debts remain signed and borrowing costs stay attributed to the original liability. Grouping does not create manual override scopes or ledger identities, and does not deduplicate source amounts by ticker. Overview/provider/exact-asset financial calculations and strict coverage rules are unchanged. Dust remains an overlapping view, never an extra financial amount.

## Acceptance

Deterministic tests cover exact contract/chain validation, optional/zero decimals, unsafe icon URLs, manual-display and scale protection, offline roundtrip, equal-ticker distinct contracts, unchanged financial figures/revisions, underlying selection/allocation, non-parity token prices, debt handling, unresolved priced holdings in Dust, and resolution restoring normal classification. Browser acceptance covers bundled SVG display, cached metadata without external requests and rejection of active SVG image payloads. Live preservation checks compare every preexisting table against the pre-fetch backup, including balances, scales, source references, jurisdiction, manual state and exclusions.

## Provider presentation refinement

Provider display names already persist separately from stable IDs in `ProviderDisplay`. The projection respects these names and supplies brand defaults (`Nexo`, `Binance`, `Kraken`, `EVM wallets`) when no name is stored. The desktop allocation table flattens the migration-created `legacy:nexo` container only when it is the provider's sole source account, at the root, without a local label/parent choice. Currencies/products then appear directly under Nexo, or directly in the provider-selected pane. Multiple accounts, custom organization and labels retain their rows. Core hierarchy, IDs, amounts, reconciliation scopes and manual actions are preserved; holding details retain the original account label/reference.

Native currency identity is reviewed from chain information, not ERC-20 methods. The public chain registry provides [Ethereum mainnet](https://github.com/ethereum-lists/chains/blob/master/_data/chains/eip155-1.json) and [Arbitrum One](https://github.com/ethereum-lists/chains/blob/master/_data/chains/eip155-42161.json) nativeCurrency facts (Ether/ETH/18); both display in the ETH group while their exact chain identities stay separate.

## Direct quantities and alternative representations (14 October 2026)

The asset navbar now displays the exact sum of **direct native asset** quantities instead of a representation count. The display DTO widens differing decimal scales exactly before summing into the group's native currency unit. Original ledger quantities, asset identities, role distinctions, quotes and contribution attribution remain unchanged.

Classification is provider-qualified in the reviewed mapping. Nexo ETH and native wallet ETH count as direct ETH; reviewed WETH contracts are separate wrapped representations. Native ETH on the supported L2 networks remains direct native currency in the ETH group, with chain identities retained in the allocation tree. Nexo's USD identifier represents tokenized USD, not real USD currency. USDT, USDC and xUSD credit remain separate proxy/claim quantities. By the user's currency-display convention, Nexo EURX, GBPX and USDX count toward EUR, GBP and USD quantities, alongside their legacy EUR/GBP balances. This classification is specific to Nexo; their exact identities and own market quotes remain unchanged. A same-ticker contract never becomes direct currency without a reviewed mapping. Missing identity and unpriced holdings remain Others / dust.

A **Wrapped / tokenized** accordion under the navbar line exposes alternative holdings by exact asset, provider and role, with their own quantity and signed reporting-currency value. If there are no direct holdings, the parent explicitly says **No direct USD** (or the relevant currency); it does not add token quantities into a fictitious direct balance. Debts never join direct asset quantities. The parent value still includes the group's existing assets and liabilities once. Alternative entries are an explanatory list, not extra ownership or new manual override scopes. Search includes their names and tickers.

Regression coverage checks Nexo USD versus another provider's direct USD, summed ETH with different native scales, chain-specific WETH with a non-parity price, preserved debt and unchanged financial totals. Browser coverage checks direct quantity formatting, accordion expansion/collapse and continued Overview selection.

Validation for this refinement: all 19 portfolio tests, the regenerated native wire-fixture test, TypeScript/build, 6 desktop browser tests and 3 review browser tests passed. The real preview shows 214.22 direct ETH; wrapped and tokenized alternatives remain separately expandable; the subsequent FiatX display convention includes Nexo FiatX in the parent currency quantities. No price acquisition or ledger edits were performed.

Navbar APYs use the bundled Lucide chart-no-axes-combined outline SVG, with `~` before estimated values instead of `(est.)`. Compact quantities remain on one line. Quantity suffixes use the font glyph when available (including USD/EUR/GBP and BTC). ETH, NEXO, BNB, NEAR, DOT and POL use monochrome shape SVGs adapted from the existing licensed coin geometry, without background circles. Crypto quantity icons are one inherited line-height high (`1lh`), matching the amount text box at its top and bottom without increasing the line height. Unsupported symbols and wrapped-asset tickers retain text, and accessible ticker names remain available. Warning buttons appear directly beside titles at text-line height. Source SVGs and license notices are retained under `apps/desktop/src/assets/outline`; all geometry is static and makes no runtime network requests.

The title-warning refinement passed all 6 desktop and 3 review browser tests, including separate warning activation, preserved Overview selection, line-height positioning and no-wrap quantities. Desktop/design-system types and build passed. Warning-only details guard absent provider references instead of trying to format an undefined string. No financial/database changes were made.

Fiat quantity glyphs inherit the complete amount font and line height, including tabular-number styling. Their text boxes share the amount's top and bottom edges; the earlier independent `line-height: 1` setting was removed after live alignment review.

Partial-value badges now read **Partial**, with the full known-subtotal explanation retained as descriptive title text. Navbar titles and values align at the first line; sidebar badges occupy their own right-aligned line within the value column, keeping room for longer titles. Titles can shrink without overflowing. Others / dust uses the same right-aligned value layout rather than special left-aligned rules.

The compact Partial layout passed 4 review browser tests (including a large negative partial value at 687px), 6 desktop browser tests, TypeScript and build. Badge/quantity bounds remain within the row and do not overlap its title. Financial values and coverage rules remain unchanged.
