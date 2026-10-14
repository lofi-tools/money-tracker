# Provider display groups

Implemented for review on 14 October 2026. `lib-app::provider_grouping::ProviderGrouping` lets provider-owned adapters describe partition sections and chain labels. The portfolio tree consumes these policies; economic IDs, raw accounts, positions, quote sources and rate scopes remain unchanged.

Nexo groups inferred fixed pots and unresolved cohorts under **Fixed terms**, with **Flexible savings** alongside it. Asset/exposure detail removes the redundant exact-asset holding header. Overview retains the canonical holding node so section detail targets resolve consistently. Informational partitions and section headers have no ledger positions and never add ownership twice.

Annotated EVM accounts receive a software-scoped display provider (`wallet-software:<normalized name>`). MetaMask is shown separately from unannotated **EVM wallets** and other software annotations. The raw provider remains `evm`. Software provider trees merge accounts under chain headings, then show their tokens; multiple accounts on the same chain remain distinct holdings. Chain section IDs use a stable account selected across all matching metadata, so asset and provider detail targets resolve to the same canonical section. Holding details retain source-account annotation controls.

The navigation meta-asset is labelled **Others**. Unknown/unpriced tokens keep their individual audit identities inside it.

Provider navigation uses bundled Nexo, Binance, Kraken and MetaMask marks. Solana uses the existing pinned CC0 coin-icon source for its colour navigation mark and a monochrome quantity symbol. Provenance and licensing are recorded in `apps/desktop/src/assets/providers/README.md` and the coin assets `provenance.json`. Fiat quantity symbols remain font glyphs; the existing inline sizing contract applies to the SOL SVG.

Regression checks cover Nexo value/earnings conservation, unknown-rate qualification, inferred versus saved dates and backfill, software grouping with several accounts/chains, and canonical section lookup. Source-cutoff and price warnings remain visible.

Review checks: core/application/native Rust suites passed (private/live cases excluded), including all 20 portfolio integration tests. Desktop TypeScript and production build passed, as did six desktop browser tests and four review browser tests. Solana is covered by the offline bundled-icon check. The selected CLI balance report remains unchanged after account annotations.

## Inline value presentation follow-up

Monochrome quantity SVGs are rasterized locally at mount to measure painted alpha area. Their geometry scales toward a shared ink-area target, bounded to the text line. The horizontal SVG viewport is cropped to the adjusted ink bounds, including stroke padding, so its gap to the number matches the font-glyph gap. This optical adjustment applies to all bundled crypto quantity symbols; fiat symbols remain text. No external request is needed. **Others** uses an original bundled SVG depicting several currencies emerging from an isometric box.
