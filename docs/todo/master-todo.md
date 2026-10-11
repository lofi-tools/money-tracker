# Provisional master todo list

Updated 11 October 2026. This is the single delivery checklist for the product direction established in the [user interview](../findings/user-needs.md) and [product brief](../spec/product-goals.md). It is deliberately broad and provisional: sections express priority/dependencies, not promised release dates. Unchecked means outstanding; inclusion does not imply implementation or approval to publish anything. Suggested engineering work is identified separately below.

The first work packages were **data model review → existing-provider documentation → Kraken**; their implemented foundations are recorded below. Current connector work is EVM live coverage and remaining mappings, then Solana and NEAR. Near-term design-system, homepage/docs, and three-OS CI work follow alongside the foundational work; they should not wait for the distant features.

The product sequence remains **holdings → received/current annual earnings → same-asset opportunity comparisons → withdrawal planning**. Integrations after Nexo/Binance are **Kraken → EVM wallets → Solana → NEAR**. Initial coverage is crypto plus related fiat, with broader financial accounts later. First-version actions are suggestions only. Beyond the first three work packages, sections group workstreams rather than impose a strict waterfall: shared portfolios remain soon after single-user tracking, and near-term web/CI work can run alongside core work.

## 1. First: data model review and minimum supporting changes

Deliverable: a source-grounded proposed design under `docs/spec/`, with a feature-to-model mapping, minimal schema/API changes, phased migrations, and acceptance examples. Start from [current types](../spec/data-model.md) and [persistence](../spec/persistence.md), not the Airtable columns.

Review delivered 11 October 2026 in [proposed data model evolution](../spec/data-model-proposal.md). It includes recommended entities, ingestion/query boundaries, phased migrations and acceptance examples. Checked design-only items below mean a proposed contract is documented, not that the capability is implemented. The first portfolio/account and exact-quantity core slice is implemented; broader model work remains outstanding. Calculation conventions explicitly identified as open must be settled before their phases.

- [x] Map every confirmed user question to the data and queries needed to answer it; separate immediate, next-stage, and later requirements.
- [x] Inventory current persisted versus transient entities, identifiers, amount types, roles, and evidence; document reusable foundations and actual gaps.
- [x] Define portfolio/entity ownership and provider/account instances, including multiple accounts at one provider and future family/client portfolios.
- [ ] Persist the minimum provider/account/product/position relationships, labels, dates, and external references needed for holdings and earnings; define retrieval APIs.
- [x] Separate held positions/subscriptions from available product offers and researched providers. Do not recreate unused spreadsheet catalogue fields automatically.
- [x] Define chain-qualified native/token identity and cross-provider asset aliases; distinguish renamed, migrated, wrapped, and receipt assets.
- [ ] Fix `ExternalAssetId` equality/hash consistency and define persistent source asset mappings.
- [x] Choose exact amount/scale semantics that support chain balances beyond the present signed `BIGINT` range; replace Nexo float parsing and define overflow/scale-change behavior.
- [ ] Preserve economic asset/liability/external roles, signed debt, and owned collateral while extending the model; do not rebuild the already implemented role fix.
- [ ] Preserve event semantics alongside effects: deposits, withdrawals, internal transfers, interest, staking, cashback, bonuses, promotions, fees, loan interest, and recoveries.
- [ ] Scope source transaction identities and import hashes by connection/account; preserve repeated identical legitimate events and deduplicate overlapping API/CSV records.
- [x] Define source/import/mapping provenance, UTC/source timezones, original amounts, parser versions, and reproducible normalized records.
- [ ] Model independent current balance observations, dated coverage boundaries, reconciliation status/issues, and warnings separately from ledger movements.
- [ ] Model tracked/excluded assets and positions so hidden spam never changes totals or yield; decide how exclusion changes apply to historical views.
- [x] Define reward receipt identity, principal deposits/withdrawals, disposal attribution, realized proceeds, fees, and retained reward quantities.
- [ ] Model dated rate observations, reward currency, APR/APY semantics, tiers/caps, qualifications, unknown rates, and current annual earning calculations.
- [ ] Model composite/share positions and dated underlying conversions without double counting receipt and underlying value.
- [x] Identify later-compatible boundaries for lock terms, collateral, claims, risk evidence/dependencies, eligibility, and shared access without implementing every later entity now.
- [x] Define whether source revisions/mapping corrections need an immutable audit trail; current stable-ID effect replacement is not append-only history.
- [x] Append ordered idempotent migrations for the first portfolio/account and exact-unit slice; preserve data/import markers, transactionality, rerun-at-every-open behavior, and rollback/retry acceptance cases. Later schema changes must follow the same contract.
- [ ] Validate the proposed model with meaningful examples: Nexo debt, Binance Earn, Kraken ledger fees, a different-currency reward, partial disposal, changing principal, and a composite DeFi position.

Implemented foundation: portfolio/account records and scoped import/query APIs; arbitrary-precision units; scale conflict rejection; exact Nexo/Binance parsing; migrations 004/005. Existing provider entry points retain legacy IDs. Remaining work includes connections/catalogue asset mappings, product/held-investment metadata, provenance/reconciliation coverage, account selection in application workflows and deliberate legacy source remapping. Cached Nexo/Binance upgrade checks operate on temporary copies.

## 2. Second: document every existing provider and acquisition source

Deliverable: a consistent provider/source guide with verified code paths and runnable provider-only commands. These documents describe current behavior and limitations, not planned support as completed.

- [x] Define a common provider-guide structure: supported accounts/products, data sources/endpoints/files, source fields/IDs, timezones/precision, position roles, transaction-effect mappings, independent balances, coverage, price inputs, tests, and known gaps. See [provider conventions](../findings/provider-diagnostics.md).
- [x] Document Nexo CSV acquisition, configurable input, exact-byte hash imports, stable IDs, aggregate asset positions, credit liabilities, duplicate draw handling, and unsupported product detail. See [Nexo](../spec/nexo.md).
- [x] Document Binance API and statement/ZIP paths, account families, trade/fee/transfer/reward mappings, overlap deduplication, legacy Savings evidence, independent observations, cache layout, and rate limits. See [Binance](../spec/binance.md).
- [ ] Document CoinGecko current/recent history and fiat cross-rate handling as a **price source**, not an account integration.
- [ ] Document Binance Vision archive closes, pair/month caches, unavailable periods, and USDT proxy assumptions as a price source.
- [ ] Document KuCoin candle, ECB FX, POL/MATIC alias, and Nexo export-implied pricing fallbacks, including provenance and approximation limits.
- [x] Add source-record → normalized event → persisted signed effects examples for representative provider operations, including fees in a third asset and debt activity.
- [x] Explain asset sums by cutoff/scale/role, liabilities separately, and net-worth inclusion/exclusion; show why counterparties are not user debt.
- [x] Record which balance checks are independent and which merely confirm source-to-ledger consistency; do not label CSV self-comparison as independent reconciliation.
- [x] Inventory existing provider-only print tests and Nix wrappers. Nexo has `latest_amount_after_cached_export`; Binance has `latest_amount_after_cached_account` and the statement display path used by `nix run .#binance`.
- [ ] Standardize a provider-only diagnostic/print workflow: provider/account selection, exact balances, liability totals, cutoff, reconciliation warnings, import/cache progress, and offline replay.
- [ ] Extend Kraken's account-scoped diagnostic isolation to Nexo/Binance. Kraken now has meaningful mixed-database verification; legacy diagnostics still rely on separate fixture DBs.
- [x] Document commands, initial API-key/CSV prerequisites, persisted fixture DBs, frozen observations, ignored/live test behavior, and synthetic/private fixture boundaries for Nexo and Binance.
- [ ] Link directly to export pages when verified, with format/date-range/timezone guidance and account coverage limitations.
- [ ] Add the same guide and diagnostics to every new connector as part of its completion criteria.

Source review findings: Nexo has no independent balance reconciliation and ignores generic CSV fee columns; Binance covers no debt account family. Current print queries are unscoped and rely on separate fixture DBs. The Nexo print test uses a fixed private export and approximate display with legacy expected balances. Binance display success does not assert reconciliation. Both adapters retain legacy provider-wide IDs. These limits are documented in the guides; account-scoped diagnostics, source provenance, reconciliation status and broader product/debt modeling remain covered by the existing model/diagnostic tasks above.

## 3. Third: implement Kraken

- [x] Verify official ledger/balance/asset/authentication contracts and read-only permissions for the first default-wallet Spot scope; record explicit Earn suffixes and unsupported account families.
- [x] Add synthetic ledger/Earn/fee fixtures and separate dated balances, plus signed mock HTTP acquisition/cache tests.
- [x] Validate one private default-wallet account: 509 ledger entries plus independent current balances reconcile exactly after the bonded DOT alias fix; cached replay passes without credentials (2026-10-11).
- [ ] Broaden real fixture coverage for other regions, remote wallets, products, retention and dated exports; one account's passing capture does not establish universal coverage.
- [x] Implement the Kraken acquisition client with pagination, serialized signing/nonces, bounded throttle recovery, cached responses and resumable fixed-cutoff history.
- [ ] Adopt Kraken's preferred cursor pagination and assess cross-process key/nonce coordination; current documented offset pagination and serialization are process-local.
- [x] Normalize explicit Kraken identifiers/aliases and exact arbitrary-precision decimal quantities while protecting existing asset scales; fail unknown/ambiguous mappings.
- [x] Map ledger amounts, actual fees, funding/trades/rewards and supported Earn bucket movements to account-scoped signed effects; preserve aggregate ownership across represented transfers.
- [ ] Add individual Earn allocation/strategy metadata and supplement with trades/exports only where additional evidence is needed; prevent duplicate economic movements.
- [x] Fetch independent BalanceEx quantities; reconcile the selected account including historical/current-only and zero positions, without balancing plugs.
- [ ] Add API-generated export recovery (AddExport/ExportStatus/RetrieveExport), then manual CSV ingestion with native-ID deduplication and independent reconciliation. User export instructions are documented; neither ingestion path is implemented.
- [ ] Import independent dated balance exports from Kraken's website and reconcile all-time ledger effects through the report's timestamp. BalanceEx has no historical date parameter; the documented export API only supports ledger/trade reports.
- [ ] Support remote wallet selection and verify recovery of unavailable historical data.
- [ ] Preflight API-key permissions and history date restrictions using GetApiKeyInfo; surface restricted backfill explicitly.
- [ ] **Later — optional Kraken OAuth:** contact Kraken about partner approval/app registration; confirm eligibility, costs, public desktop callbacks and PKCE, then implement Connect/Fast API onboarding with minimal permissions. [Manual read-only API keys are the current chosen approach](../findings/kraken-connection.md); OAuth does not block the next tasks.
- [x] Import effects/registrations/independent observations/markers atomically under stable account and native ledger identities; test replay, changed snapshots, fees, precision, transfers, rewards and failure rollback.
- [x] Add Kraken-only exact balance print and strict frozen-cache reconciliation tests with mixed-provider/account isolation.
- [x] Expose Kraken in the ordinary CLI portfolio DB via `kraken --account ID`, with cache/portfolio selection, offline replay and explicit refresh.
- [x] Write [the Kraken provider guide](../spec/kraken.md), source-backed mappings, commands, coverage and limitations.
- [x] Document [user connection instructions and acquisition strategy](../findings/kraken-connection.md), including OAuth availability, unverified localhost support and API/manual export boundaries.

Implemented Spot foundation uses existing idempotent startup migrations and does not modify legacy Nexo/Binance IDs. Margin/credit/derivatives, tokenized assets, KFEE credits, unsupported ledger types and unknown mappings fail explicitly rather than silently overstating coverage. Full provider metadata, reconciliation status and historical earnings classification remain model work.

## 4. Near term: TypeScript design-system package

Reference: [Taskstream structure notes](../findings/taskstream-reference.md). Its root package is named `taskstream` at `/Users/me/src/me/todo-lofi`; this is a structure reference, not a runtime dependency on that checkout.

- [ ] Add a standalone TypeScript package, provisionally `libs/web-design-system`, with a reproducible workspace/lockfile and stable exports.
- [ ] Use the selected SolidJS + Panda CSS + Ark UI stack and define its boundaries with the Rust core/Tauri shell.
- [ ] Define tokens, typography, colors, spacing, themes, responsive behavior, and accessible focus/keyboard states.
- [ ] Add reusable controls, tables, filters, settings, navigation, warnings, empty/loading/error states, and public/paid feature indicators.
- [ ] Add holdings/earnings/price formatting, chart primitives, accessible chart summaries, dates, currencies, signed amounts, and missing-data states.
- [ ] Add marketing/docs components, navigation/sidebar/TOC, prose, callouts, code blocks, and shared page layouts.
- [ ] Provide clean Panda codegen/import conventions usable by the public site, Solid web app, and Tauri frontend; keep generated output reproducible.
- [ ] Add an isolated showcase/demo plus typechecking and focused component/interaction/accessibility checks.
- [ ] Check reuse/license boundaries if adapting Taskstream components; do not copy its product-specific content or add sibling-path dependencies.

## 5. Near term: homepage and documentation site

- [ ] Follow the Taskstream pattern of a separate site app, shared design-system package, `/` homepage, `/downloads`, `/roadmap`, and audience-specific `/docs` navigation; evaluate its Astro approach for the site.
- [ ] Use [product questions, pain points, and solutions](../spec/product-goals.md) as homepage source material: holdings, earnings, better same-asset opportunities, privacy, and later liquidity planning.
- [ ] Label shipped, next, and later capabilities accurately; do not advertise all provisional tasks as available.
- [ ] Add user guides for install, connect/import, reconciliation warnings, earnings definitions, wallet exclusions, settings, and optional services.
- [ ] Add contributor guides for architecture/data model, migrations, provider mappings, diagnostics/tests, and contribution workflow.
- [ ] Explain subscription-free local use, public catalogue/data, paid risk/dependency APIs, future bank/broker integrations, and optional managed cloud clearly.
- [ ] Add platform downloads tied to actual releases and publish the provisional roadmap in readable Now/Next/Later form.
- [ ] Define how repository specs/findings feed public docs without publishing private exports, credentials, or paid risk data.
- [ ] Add responsive layouts, keyboard navigation, link checking, metadata/SEO, and basic build/browser smoke checks.
- [ ] Plan public opportunity catalogue/dataset routes and a documented dataset schema/access format; deliver the real catalogue in the opportunity stage.

## 6. Near term: CI and reproducible builds on three systems

Assumption for “3 systems”: **Linux, macOS, and Windows**. CPU architecture coverage, packaging formats, and signing policy are separate choices.

- [ ] Add CI build/test lanes for Linux, macOS, and Windows for the existing Rust workspace first.
- [ ] Resolve the external `typed-ids` sibling-path dependency in `Cargo.toml` so clean checkouts build without the author's local directory tree.
- [ ] Pin/document toolchains and native DuckDB/C++ dependencies; support Windows-native builds as well as the current Linux/macOS Nix environment.
- [ ] Cache dependencies/build outputs responsibly and use lockfiles for reproducibility.
- [ ] Run formatting, appropriate linting, deterministic unit/mapping/storage tests, and documentation checks on pull requests.
- [ ] Audit existing private/live/cache-first tests, including non-ignored real-account tests; keep ordinary CI free of financial credentials and private fixtures.
- [ ] Add an explicitly opt-in private/live reconciliation lane with controlled secrets and no publication of private artifacts.
- [ ] Add TypeScript codegen/typecheck/test/site build lanes when those packages exist; add all-platform Tauri builds when the desktop shell exists.
- [ ] Publish per-platform build artifacts and smoke-test install/startup/migrations with temporary data; decide release packaging and supported architectures.
- [ ] Add release tagging, changelog/versioning, checksums, and later signing/notarization/update-delivery decisions before distributing production desktop bundles.

## 7. Foundations and holdings workflow

- [ ] Expose Nexo/Binance/Kraken connection/import/reconcile/query flows in normal CLI commands with account selection and one chosen portfolio DB.
- [ ] Remove Nexo's dated default export path; support managed selection/import of multiple original files, stable IDs, and hash deduplication.
- [ ] Resolve the unused config-file flag: implement loading/precedence or remove the option; decide whether automated content input is useful.
- [ ] Build the first Tauri desktop shell using the shared Solid frontend and typed Rust/frontend contracts.
- [ ] Add account setup/status, verified current balances, provider warning icons, asset/provider/platform groupings, liabilities, and net worth.
- [ ] Target history from first platform activity; recover exports/manual evidence only where automatic acquisition cannot fill it.
- [ ] Recover missing Airtable tables/definitions only where they resolve confirmed wallet/position/term needs; safely preserve unresolved rows/provenance without importing the entire research catalogue as holdings.
- [ ] Keep current observations separate from ledger history; withhold unreconciled historical totals/yield and label reconciled-provider-only historical subtotals.
- [ ] Implement automatic balance/rate refresh while running with a configurable interval; manual refresh belongs only in options.
- [ ] Implement reporting-currency settings (USD default), available alternative currencies, timestamps, stale-source indicators, and offline cached views.
- [ ] Show failed-platform funds as zero/loss until recovered, retaining claim and distribution evidence rather than invented expected recoveries.

## 8. Early: received earnings and historical performance

- [ ] Report net earnings prominently with gross earnings, fees, borrowing costs, and explainable source detail.
- [ ] Separate interest/staking from cashback/signup/promotional rewards; include these in total earnings while separating price gains/losses.
- [ ] Track realized and unrealized price gains independently, with oldest-first attribution by default and configurable/overridable rules.
- [ ] Value retained rewards at today's prices and sold/swapped rewards at actual realized proceeds after fees; avoid double counting reward proceeds and price changes.
- [ ] Treat crypto-to-crypto reward swaps as realization for portfolio reporting without claiming a universal tax rule.
- [ ] Default received-earnings periods to trailing 12 months; support calendar year, all time, and custom dates.
- [ ] Build historical net-worth, earnings, and performance graphs with explicit reconciled coverage and source/cutoff information.
- [ ] Define and implement the early buy-and-hold benchmark: same dated external deposits/withdrawals, no staking/lending yield, fee-aware comparison, and no internal-transfer contributions.
- [ ] Resolve benchmark starting assets, contribution/withdrawal allocation, actual-trade handling, valuation, and return calculation so market gains and fresh deposits remain distinguishable.
- [ ] Define effective-yield calculation for changing principal using invested amounts/duration; choose annualization conventions with transparent examples.
- [ ] For different-currency rewards, show historical effective return from hypothetically retaining all receipts since opening and converting them into the earning asset now.
- [ ] Show current immediate APY assuming reward conversion into the earning asset on receipt, separately from the historical effective rate.
- [ ] Answer “how much am I currently earning per year?” for existing positions at current rates, exposing reward-value assumptions rather than forecasting renewal decisions.
- [ ] Compare retained rewards with immediate conversion into the earning asset; define how to show consistently negative relative outcomes.
- [ ] Verify partial disposals, changing principal, costs in other currencies, sold reward attribution, and missing/reconciled history in calculations.

## 9. Next connectors: EVM wallets, then Solana, then NEAR

- [x] Implement the [EVM native/ERC-20 foundation](../spec/evm-wallets.md) for Ethereum, Polygon, BNB Chain, Arbitrum and zkSync Era: DB-stored accounts, contract-qualified identities, exact amounts, cached replay and pinned independent RPC observations. BNB needs Etherscan chain access or an indexer override.
- [ ] Complete live reconciliation for the registered wallet on all five chains; see [current contract/source blockers](../findings/evm-contract-resolution.md). Keep exact comparisons with no rounding tolerance or balancing entries.
- [x] Identify SafePromo and INS Promo; recover canonical historical decimals and retain raw RPC evidence through `evm --resolve-tokens`. INS Promo scale is zero; known destruction still causes a 777-unit ledger discrepancy. SafePromo's fake constant balance and missing scale remain quarantined.
- [x] Implement optional Blockscout universal API authentication/pacing and configured Etherscan routing for Polygon/Arbitrum. Etherscan history works for both; the supplied Blockscout key accesses Arbitrum but needs a higher plan for Polygon.
- [x] Select public Polygon dRPC after verifying oldest transaction availability absent from the previous RPC.
- [ ] Add reviewed nonfinancial contract classifications/tracking exclusions without fabricating decimals; preserve diagnostics and exclude reviewed spam from financial totals/yield.
- [ ] Model proven destroyed/retired-contract losses with effective timestamps and evidence; preserve historical units instead of inferring a balancing transaction from zero RPC balances.
- [ ] Obtain exact Classic-era Arbitrum feeStats and map fees; Nitro receipt multiplication and Blockscout's fee field reproduce an approximate value, leaving an 87,468-wei difference in the first capture.
- [x] Recheck Polygon/Arbitrum metadata at their frozen cutoffs; five Arbitrum contracts and one Polygon contract still have getter execution failures with RPC code 3 (the final Arbitrum pass fails four balance calls and one decimals call). Preserve method-specific failure evidence.
- [ ] Investigate the four Polygon contract discrepancies (PolyDough, DxDex.io, lookalike UЅDС, Zepe) and the six contracts with reverting getter calls; implement only evidenced protocol/accounting mappings or reviewed tracking classifications. Native Polygon and 41 readable token quantities match.
- [ ] Add partial-page acquisition resume, Etherscan split-range backfill beyond 10,000 rows, robust reorg rollback and block-aware historical queries.
- [x] Map native/internal transfers, ERC-20 receipt events and failed-transaction gas with stable chain/account identities; swaps retain actual transfer effects.
- [ ] Add semantic swap/protocol classification and unsupported token/fee mechanisms.
- [ ] Discover holdings and DeFi positions automatically; provide spam hiding/tracking exclusions that also affect totals/yield.
- [ ] Model receipt/share conversions and underlying drill-down; use the Airtable wallets/positions to identify protocol coverage, not to assume every researched offer is held.
- [ ] Handle pagination/history limits, chain reorgs, indexer/RPC discrepancies, unrecognized contracts, and unresolved positions explicitly.
- [ ] Implement Solana wallet/token/staking/DeFi acquisition and independent observations, including chain-specific fees and event identities.
- [ ] Implement NEAR account/token/staking/protocol acquisition and relevant exported wallet/MetaPool evidence.
- [x] Add the EVM provider guide, exact quantity print diagnostic, synthetic account/chain fixtures and frozen offline reconciliation.
- [ ] Apply those provider guide and diagnostic conventions to Solana and NEAR.
- [ ] Keep the broader [provider discovery inventory](provider-roadmap.md) as candidates for later demand-driven connectors; no older ranking overrides the confirmed sequence.

## 10. Opportunities and net earnings comparisons

- [ ] Build a per-asset opportunity catalogue separate from personally held positions.
- [ ] Collect provider API/website rates where possible with human review, manual additions/corrections, provenance, and freshness/version history.
- [ ] Publish opportunities, yields, and descriptive risk summaries on a website and accessible dataset; choose schema, delivery format, license, and update cadence.
- [ ] Show all opportunities with eligibility indicators and an eligible-only filter; define residency/account/balance/qualification inputs.
- [ ] Display headline APY but compare actual-balance net earnings after fees, tiers, caps, qualification costs, and switching costs.
- [ ] Default comparison to one year with selectable periods; apply switching costs over that chosen period.
- [ ] Include platform-token/membership requirements with costs/risks and opt-out filters; distinguish capital tied up in a token from a nonrecoverable fee.
- [ ] Suggest same-asset splits across opportunities to improve net earnings and use caps/tiers; do not make asset-reallocation recommendations implicitly.
- [ ] Preserve user liquid reserves and exposure limits; explain assumptions and reject stale/unverified rates/assessments from allocation suggestions.

## 11. Risk ratings, preferences, and paid APIs

- [ ] Define our documented assessment methodology using third-party evidence, traceable sources/reasoning, and human review.
- [ ] Choose risk dimensions, score scale/direction, aggregation, uncertainty handling, review cadence, and methodology versioning.
- [ ] Provide common overall and per-risk ratings for everyone; keep personalized use of ratings separate from shared assessments.
- [ ] Add conservative-to-aggressive presets plus per-risk limits; define warnings, weighting, and behavior when both presets and explicit limits are set.
- [ ] Show known risks exceeding limits with warnings by default and allow filtering; show unknown/stale risks clearly and exclude such opportunities from allocations until verified/current.
- [ ] Support maximum exposure percentages per platform, protocol, and blockchain; define overlapping exposure attribution and denominators.
- [ ] Include provider/asset concentration in portfolio-level risk and avoid false diversification through shared dependencies.
- [ ] Model evidenced provider relationships/dependencies, including direction and uncertainty, for possible cascading failures; offer this map through the paid API.
- [ ] Publish paid ratings/dependency API contracts, auth/entitlements, billing, versions, usage limits, source traceability, and public-versus-paid boundaries.
- [ ] Define free desktop behavior when paid risk data is unavailable; free holdings/earnings must remain usable.
- [ ] Much later: interactive provider-failure scenarios showing direct exposure and knock-on effects, with explicit assumptions rather than invented loss probabilities.

## 12. Withdrawal planning and opt-in notifications

- [ ] Let users set an immediately withdrawable minimum per asset; only suggest locking the remainder.
- [ ] Model product lock/maturity/unbonding/withdrawal rules and current restrictions, with evidence and unknown states.
- [ ] Suggest staggered repeating fixed terms (including Nexo) to improve withdrawal frequency and availability.
- [ ] Build the combined reporting-currency availability graph using today's prices held constant and stopping renewals from today by default.
- [ ] Answer time to 50%, 100%, or a specified withdrawable value across all platforms/products; define percentages, unavailable funds, and expected reward treatment.
- [ ] Add target amount/deadline plans showing what to withdraw where and when, with suggested buffers and infeasibility explanations.
- [ ] Start with funds unlocked on-platform; distinguish this from money delivered to a destination.
- [ ] Add debt-aware availability before/after repayment, costs and funding effects, and collateral/safety constraints; resolve stressed-price assumptions.
- [ ] Implement opt-in notifications with no notifications enabled by default and a setup suggestion.
- [ ] Let users select maturity, renewal-deadline, and better-opportunity alerts, channels/lead times, and repetition rules.
- [ ] Support minimum net-dollar and percentage improvements after switching costs over the chosen period; define threshold combination and non-USD behavior.
- [ ] Later: money router mapping platform/blockchain connections, fees, conversions, timing, eligibility, and route risks to minimize movement costs.
- [ ] Later: target-date plans for funds converted and delivered to a chosen wallet/bank account using the router.

## 13. Soon after single-user tracking: family/client portfolios

- [ ] Add separate portfolio/entity ownership and management views without mixing clients' assets into the manager's personal net worth.
- [ ] Let a manager add/track a friend/family member's accounts before that person joins.
- [ ] Implement local preconfigured invitations first: carry appropriate API keys/manual imports and clone that portfolio's history into the recipient's DB.
- [ ] On recipient signup, expose existing data and retain inviter shared-admin rights until the recipient explicitly removes them.
- [ ] Keep source transactions read-only in this sharing workflow; settings/connections are read-write. Resolve how manual corrections/attribution overrides fit without altering source evidence.
- [ ] Specify secure handoff, record selection, identity acceptance, credential ownership, and revocation effects; do not imply deleted access erases an already retained local copy.
- [ ] OAuth connections require their owner; the manager receives imported data rather than logging into someone else's OAuth account.
- [ ] Add the same invitation flow to cloud portfolios using shared row access rather than per-user cloned rows.
- [ ] Later: continuous synchronization between local installations, including recipient-imported OAuth data; initial handoff does not claim this is already supported.
- [ ] Extend shared use next to couples/groups/households, then companies, with ownership shares and permissions scoped to those needs.

## 14. Later service and financial expansion

- [ ] Add optional managed cloud portfolios: one login, managed provider keys/sync, OAuth where available, ownership/access rules, and data lifecycle.
- [ ] Support banks, stocks, pensions, mortgages, and other net-worth assets/liabilities with account-appropriate data models and evidence.
- [ ] Offer bank/broker integrations through a paid cloud API plan; verify jurisdictions, provider access costs, history, and independent balances.
- [ ] Add tax reporting as a cloud service syncing local data; choose jurisdictions, required evidence, accounting/export rules, and privacy boundaries before promising filing support.
- [ ] Add spending tools later; scope categorization, recurring/unused subscriptions, document evidence, and any AI enrichment separately.
- [ ] Add community integrations later: extension contracts, packaging, compatibility, documentation, and trust/permission boundaries.
- [ ] Add mobile and wider device sync later, defining offline edits, conflicts, encryption, identities, and local-versus-cloud choices.
- [ ] Add encrypted backup/restore later, including key recovery and tested restore/migration behavior.
- [ ] After paid cloud/API plans are live: add affiliate monetization with disclosed placement and independence from risk/opportunity assessment.
- [ ] Later: transition desktop UI to Rust + GPUI; reuse core behavior and decide coexistence/support for the Tauri version.
- [ ] Much later: invoicing, with formats and accounting links scoped when needed.
- [ ] Consider transfer/staking/renewal execution later as a separate product decision; it is not a first-version requirement or committed roadmap promise.

## 15. Suggested usual engineering/product work

These are proposed supporting tasks in response to “what else usually belongs in todos?”, not newly confirmed user features. Schedule them proportionately; they must not pull later backups, sync, or execution into the first release.

- [ ] Select an open-source license and clarify app/public-data/paid-service boundaries; add contribution, issue/reporting, and release guidance.
- [ ] Document clean-checkout setup and remove machine-specific paths; make example config and deterministic fixtures work without real accounts.
- [ ] Add a maintained provider support/coverage matrix and a definition of done tying integration docs, mapping, diagnostics, and reconciliation together.
- [ ] Version schemas, data/mapping formats, API contracts, and releases; keep migration compatibility and upgrade notes visible.
- [ ] Protect local credentials appropriately, use minimal provider permissions, redact secrets/private balances in logs/errors, and define crash-report contents.
- [ ] Add secret/dependency checks and a lightweight dependency/license inventory; define a vulnerability-reporting route for an open-source financial-data app.
- [ ] Add cancellation/progress, retries, partial-import recovery, connection error states, and source freshness diagnostics.
- [ ] Add accessibility, keyboard, responsive, locale/currency/date, and theme checks for the shared UI and public docs.
- [ ] Benchmark realistic large transaction histories, query/indexing performance, migration startup, and chart responsiveness before speculative optimization.
- [ ] Keep essential use usable offline; clearly distinguish cached observations from fresh balances and missing data from zero.
- [ ] Define user-controlled diagnostics/support exports, portability, and deletion boundaries without implementing the deferred encrypted-backup feature prematurely.
- [ ] Define public-dataset licensing and paid-service privacy/terms, account deletion, subscription cancellation, and API-support expectations before public commercial launch.
- [ ] Establish a small acceptance dataset spanning deposits/withdrawals, transfers, fees, rewards, debt, current-vs-history disagreement, missing prices, and nested positions.
- [ ] Add release smoke checks, compatibility support policy, feedback collection, and a small milestone/acceptance checklist for each shipped phase.

## Status and supporting references

Already implemented foundations include core integer effects, DuckDB ledger/price persistence, Nexo import hashing, explicit asset/liability/counterparty roles, liability-aware valuation, ordered idempotent startup migrations, and Binance statement/reconciliation work. Verify these against source before modifying them; they are not new unchecked features to rebuild.

[Provider discovery](provider-roadmap.md) retains historical/export-based candidates and sources. [Implementation notes](implementation-gaps.md) retain earlier design alternatives without a competing checklist. [Taskstream reference](../findings/taskstream-reference.md) records the inspected site/package structure. Use this master list for delivery status and priorities; update specifications only as behavior is delivered or proposed design is explicitly selected.
