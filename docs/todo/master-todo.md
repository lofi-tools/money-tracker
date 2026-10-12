# Provisional master todo list

Updated 12 October 2026. This is the single delivery checklist for the product direction established in the [user interview](../findings/user-needs.md) and [product brief](../spec/product-goals.md). It is deliberately broad and provisional: sections express priority/dependencies, not promised release dates. Unchecked means outstanding; inclusion does not imply implementation or approval to publish anything. Suggested engineering work is identified separately below.

**Current priority: build the first usable desktop app/UI**, using the existing provider and storage foundations. The selected stack remains Tauri + SolidJS + Panda CSS + Ark UI. The [desktop spec](../spec/desktop-app.md) records the agreed first-build scope; the [ordered implementation plan](first-ui-implementation-plan.md) sequences the existing tasks and defines the minimum model/query changes, dependencies and validation. Stages 1–6 are implemented under the user’s subsequent authorization; remaining stages are deferred until requested. Supporting CI/build work moves forward with that milestone. Homepage/docs follows the usable app foundation; additional connectors and broad model expansion follow unless a narrowly scoped fix is needed by the app.

The earlier work packages were **data model review → existing-provider documentation → Kraken**, followed by EVM foundations. Their completed work and remaining limitations stay below. Unreconciled providers must remain clearly labelled in the UI; completing every EVM contract mapping is not a prerequisite for building the app.

The product sequence remains **holdings → received/current annual earnings → same-asset opportunity comparisons → withdrawal planning**. Later integration order remains **Solana → NEAR** after existing EVM coverage. Initial coverage is crypto plus related fiat. The UI milestone does not imply that proposed earnings, benchmark or opportunity engines are implemented. Section 0 owns the immediate work; other numbered sections retain stable references and group supporting/later work rather than impose a waterfall.

## 0. Now: first usable desktop app / UI

Deliverable: a running local app displaying balances, allocations and current earnings from the already-imported portfolio DB; account connections, CSV imports and transaction-history UI come later. The first-build scope and proposed acceptance scenarios are documented in [the desktop spec](../spec/desktop-app.md). Follow the [ordered implementation plan](first-ui-implementation-plan.md); this master list remains the canonical completion checklist. Supporting calculation/configuration details need implementation-stage review.

- [x] Document the first-build UI scope: primary journey, initial screens, setup/import scope, navigation, financial detail, reconciliation states, visual direction and proposed acceptance scenarios. The read-only slice is delivered in stage 6; remaining controls and acceptance scenarios follow the staged plan.
- [x] Implement the [separate pnpm TypeScript design-system package](../spec/web-design-system.md) at `libs/web-design-system`, with a committed lockfile, stable TSX/formatting/CSS exports and an isolated Solid showcase consuming `workspace:*`. The real app binding is delivered in stage 6.
- [x] Add the [shared Rust application-service layer and exact typed JSON DTOs](../spec/application-services.md), consumed by the CLI and available to future desktop hosts; extract provider adapters independently of the CLI executable.
- [x] Bind the SolidJS + Panda CSS + Ark UI frontend to Rust through the stage-6 Tauri shell; exact manually maintained TypeScript wire types and browser acceptance use actual Rust serialization.
- [x] Define tokens, typography, colors, spacing, themes, responsive behavior, and accessible focus/keyboard states; use balanced density with compact readable rows and enough padding/whitespace for a polished appearance. Use neutral surfaces/text with one primary accent plus accessible semantic status cues. Support light/dark themes, default to the system preference, the stage-2 settings store persists System/Light/Dark; stage 6 must bind the real app controls.
- [x] Build the first Tauri desktop shell using the shared Solid frontend and typed Rust/frontend contracts. See [delivered scope and limits](../findings/desktop-stage-6.md).
- [x] Implement shared CLI/desktop database configuration: Unix default `~/.data/money-tracker/portfolio.duckdb`, `APP_DATA_DIR` → config `data_dir` → default precedence, and real config-file loading. JSON format/location and native Windows default documented in [database startup](../spec/database-startup.md); preserve databases at previous locations.
- [x] Implement Rust startup services to open the resolved database for one explicitly scoped portfolio, detect missing/legacy files, and explicitly copy an existing DB or create a new one; serialize access through a dedicated managed worker and persist validated local preferences.
- [x] Bind startup services to Tauri: display the selected portfolio and offer Copy existing / Start new for a missing DB. Preserve the source file, run idempotent migrations and show invalid/unreadable DB errors explicitly. Database-file copying is in scope; provider connections/CSV import UI remain deferred.
- [ ] Display existing account status, verified current balances, provider warning icons, liabilities, and net worth; add left navigation inside the main view with “by asset” / “by provider” grouping controls, defaulting to assets across all accounts. Each asset/provider entry shows name, current value and a smaller current APY; sort both lists by current value, largest first. Add name/ticker search that filters navigation without changing financial totals. Place Overview above the list and select it on launch; its content is total current value and current annual earnings only.
- [x] Supply Overview, navigation-summary and allocation-level APY/current annual earnings from evidenced current positions/rates using the agreed calculation contract; expose unavailable/partial coverage instead of fabricated values. Use value-weighted overall current APY including 0% balances (the confirmed ETH example gives 4%); do not treat unknown rates as zero. Label known annual earnings as a partial subtotal where coverage is incomplete, and keep the scope’s overall APY unknown until every included allocation has a rate. Deduct annual borrowing costs as negative earnings once; label net totals partial if borrowing costs are unknown. Calculate overall APY as net annual earnings divided by gross current asset value, including idle assets; debt reduces net worth but not this denominator. See [stage-4 calculation conventions and limits](../spec/portfolio-queries.md); historical reward adjustment, provider population and full fee/tier/cap work remain in section 8.
- [ ] Add persisted manual APY entry/overrides per product/subscription and for aggregate provider/asset holdings lacking finer detail; distinguish manual from sourced rates, preserve source evidence and label unknown rates until supplied. Use the latest applicable manual/provider update unless a manual override is pinned; edit APY inline in table rows with a pin/unpin icon and an effective date defaulting to today, allowing earlier dates for corrections; preserve subsequent applicable rate changes, source evidence and update timestamps and do not treat old cache replay as a fresh update. Persist subscription-specific effective-dated rate histories, including evidenced opening rates and later changes; keep advertised product rates separate and derive product summary APY from held subscriptions; prompt review of aggregate overrides when any finer data arrives, even partial. Defer a dedicated rate-history browser/editor; historical achieved yield comes from transactions and capital-time evidence, not entered rates. Do not automatically invalidate pinned overrides. Preserve provenance, allow explicit retention or replacement with detailed rates, avoid double-counted earnings and do not infer remainder rates.
- [ ] Add clearly labelled partial current-value totals and visible unpriced holdings with warnings; allow manual current-price entry when unknown, with exact asset identity, provenance and preserved source evidence. Use the latest applicable manual/provider price unless the manual price is pinned; preserve timestamps/source evidence, edit current prices inline in table rows with a pin/unpin icon, and do not rewrite historical prices, quantities/scales or source transactions.
- [ ] Add user dust/spam declarations and restore controls in the first UI, preserving history and strict tracked-asset reconciliation. Extend the account/asset review foundation with exact-asset-wide spam policy covering existing and future accounts, never ticker-based propagation. Dust is provider/asset-specific and stays tracked: include known values/earnings normally and group dust holdings under one Dust navbar entry, preserving individual detail and significant holdings of the same asset elsewhere. Asset navbar/detail totals include their dust holdings across all providers; Dust is a filtered view, not another amount added to portfolio totals. Assign dust automatically below a configurable fiat-value threshold, with persisted manual mark/unmark and reset-to-Auto controls. Default the threshold to USD 1 (strictly below), with unknown dust prices still following partial-valuation rules.
- [ ] Build the selected asset/provider right pane: current value first, asset quantity as a smaller secondary number, annual earnings, and an expandable allocation table scoped to that selection’s remaining breakdown. Represent accounts as a parent–child tree; show top-level accounts and their immediate children initially, leaving deeper descendants collapsed. Allow local grouping creation and account reparenting within a provider through a “Move to…” action in the details pane or context menu, with a valid-parent selector and cycle prevention; drag-and-drop is not required. Start from provider-specific defaults and preserve user edits on refresh. Allow finer provider/product/subscription detail where it exists; preserve ancestry in asset/provider scoped views and avoid counting parent rollups again with children. Sort sibling rows by current value, largest first, retaining the hierarchy. Reuse the model relationship tasks in section 1; preserve separate subscriptions and avoid double counting grouped totals. Allow persistent local account/subscription display names keyed by stable identity, retain source names/references in details, and allow resetting to the source name without provider refreshes overwriting local labels. Show current holdings only, deferring closed and exact-zero-balance subscriptions to the later history view while preserving their records for audit/reconciliation; retain unpriced nonzero and zero-yield holdings. Use available imported detail only; label missing product/subscription attribution and defer manual allocation splitting. Show subscription lock status, maturity date and auto-renew status when available; distinguish unknown from known non-applicable values. Open scoped details when an account/subscription name is clicked; use a separate arrow for child expansion, independent of inline editing/pinning. Include source references, rate provenance and term metadata in a right-side sliding details panel; keep the allocation table visible and preserve its selection, scroll and expansion state, with accessible close/focus behavior and reduced-motion support. Show debts as clearly marked liability rows, with borrowed amounts and negative annual borrowing-cost earnings separately. Allow inline annual borrowing-rate edits with pin/unpin icons and the same effective-date/provenance/update rules as earning rates; calculate costs from outstanding debt magnitude and applicable rate conventions, deduct once, and retain partial-net-earnings warnings when rates are unknown.
- [ ] Add Undo for manual rates, local names, grouping and dust/spam changes; capture prior state including override/pin metadata, preserve source/review evidence, validate restoration against current data and recalculate affected figures. Persist an ordered action log with prior/resulting state and undo links, atomically with each change; restore Undo availability after app restart and record validated compensating actions rather than deleting history.
- [x] Add clickable provider and affected-row warning icons with scoped details explaining missing/mismatched data and withheld/partial figures; clear warnings resolved by active manual overrides, retain unrelated issues and original evidence, and clear parent warnings when no issues remain.
- [ ] Keep current observations separate from ledger history; withhold unreconciled historical totals/yield and label reconciled-provider-only historical subtotals.
- [ ] Implement automatic balance/rate refresh while running, daily by default with the interval in settings. Add a separate refresh-on-open toggle, off by default; show stored data without an ordinary launch refresh unless enabled, but perform one catch-up refresh when scheduled intervals were missed. Persist scheduling state and coalesce catch-up/launch triggers into one refresh. Closing the window or explicit Quit ends the process and refresh work; no tray/background process. Manual refresh belongs only in options; retain cached views and freshness/coverage indicators on failure.
- [x] Implement reporting-currency settings (USD default, valuation requires available exact quotes), timestamps/source evidence and offline cached views. Add app-wide privacy modes: hide monetary amounts/asset quantities, or show much lower amounts using one common scale factor while preserving proportions, signs and percentage rates. Cover earnings/costs, warning details and hover/accessibility text; label scaled figures and never alter stored data, financial calculations or dust classification. Default scaled mode to 10% of actual amounts and let users configure and persist the common scale factor.
- [x] Add the initial Linux Rust CI workflow: locked metadata, formatting, deterministic production-crate tests excluding private/live fixtures, and CLI help. Hosted execution remains to be verified.
- [ ] Extend Rust CI build/test lanes to macOS and Windows.
- [x] Remove the unused external `typed-ids` dependency, replace Rust workspace globs with explicit members and track Cargo.lock; isolated checkout metadata excludes TypeScript directories and needs no sibling repository.
- [x] Register `apps/desktop/src-tauri` explicitly in the Rust workspace.
- [x] Configure the Linux design-system/showcase CI lane for frozen installation, codegen/typechecking, formatting, unit/browser accessibility tests and production build; hosted execution remains unverified.
- [ ] Extend TypeScript CI to the future public site and all-platform Tauri builds when those apps exist.
- [ ] Complete full milestone acceptance, including stage-7 edits/Undo and stage-8 refresh failures. Stage 6 exercises deterministic Rust wire output and temporary private DB copies for read-only navigation, exact/partial figures and privacy; final all-platform acceptance stays stage 9.

Historical graphs and earnings remain early product goals in section 8. Decide what the first UI milestone can truthfully display before treating those calculation features as ready. General design-system components, website work, packaging and remaining holdings workflow tasks retain their canonical entries below.

## 1. Data model review and supporting changes

Deliverable: a source-grounded proposed design under `docs/spec/`, with a feature-to-model mapping, minimal schema/API changes, phased migrations, and acceptance examples. Start from [current types](../spec/data-model.md) and [persistence](../spec/persistence.md), not the Airtable columns.

Review delivered 11 October 2026 in [proposed data model evolution](../spec/data-model-proposal.md). It includes recommended entities, ingestion/query boundaries, phased migrations and acceptance examples. Checked design-only items below mean a proposed contract is documented, not that the capability is implemented. The first portfolio/account and exact-quantity core slice is implemented; broader model work remains outstanding. Calculation conventions explicitly identified as open must be settled before their phases.

- [x] Map every confirmed user question to the data and queries needed to answer it; separate immediate, next-stage, and later requirements.
- [x] Inventory current persisted versus transient entities, identifiers, amount types, roles, and evidence; document reusable foundations and actual gaps.
- [x] Define portfolio/entity ownership and provider/account instances, including multiple accounts at one provider and future family/client portfolios.
- [x] Persist the minimum UI metadata and retrieval primitives: account structure/customization, provider/asset displays and alias candidates, product templates, held subscriptions with unique position links, dated rates/prices, verification records, tracking state and atomic manual action/Undo records. See [stage 3](../spec/ui-metadata.md); source population and full financial/UI projections remain separate tasks.
- [x] Bind the persisted relationships to [allocation queries](../spec/portfolio-queries.md), preserving source identities and unique-position subtree rollups with cycle/cross-portfolio validation.
- [ ] Populate provider-specific hierarchy defaults. The tree/storage/query foundations are implemented; preserve independent reconciliation scopes. Add provider-specific default hierarchy mappings with stable grouping identities and persisted user customization, so refreshes seed new/uncustomized records without overwriting edits.
- [x] Separate held positions/subscriptions from available product offers and researched providers. Do not recreate unused spreadsheet catalogue fields automatically.
- [x] Define chain-qualified native/token identity and cross-provider asset aliases; distinguish renamed, migrated, wrapped, and receipt assets.
- [ ] Fix `ExternalAssetId` equality/hash consistency and define persistent source asset mappings.
- [x] Choose exact amount/scale semantics that support chain balances beyond the present signed `BIGINT` range; replace Nexo float parsing and define overflow/scale-change behavior.
- [ ] Preserve economic asset/liability/external roles, signed debt, and owned collateral while extending the model; do not rebuild the already implemented role fix.
- [ ] Preserve event semantics alongside effects: deposits, withdrawals, internal transfers, interest, staking, cashback, bonuses, promotions, fees, loan interest, and recoveries.
- [ ] Scope source transaction identities and import hashes by connection/account; preserve repeated identical legitimate events and deduplicate overlapping API/CSV records.
- [x] Define source/import/mapping provenance, UTC/source timezones, original amounts, parser versions, and reproducible normalized records.
- [x] Persist generic exact current balance/verification/issue records separately from effects, with observation scope, completeness, independence and ledger/policy revision invalidation; see [stage 3](../spec/ui-metadata.md).
- [ ] Populate generic verification and dated history coverage from actual provider evidence. Stage 4 consumes generic records and binds scoped warnings/withheld figures; provider acquisition remains outstanding.
- [x] Implement account/asset reviewed exclusions applying to all historical totals and financial/earnings eligibility; preserve review revisions and raw history. See [the contract](../spec/tracking-exclusions.md).
- [ ] Extend tracking policy to position/product scopes, composite exposures, user settings and saved-analysis policy binding; enforce it in future earnings attribution.
- [x] Define reward receipt identity, principal deposits/withdrawals, disposal attribution, realized proceeds, fees, and retained reward quantities.
- [x] Persist dated rate evidence and subscription-specific periods separately from offers, with exact APR/APY/reward denomination, unknown/zero distinction, corrections, pins and effective-period selection; see [stage 3](../spec/ui-metadata.md).
- [ ] Populate evidenced provider rates and implement detailed rate/principal intersections, tiers/caps and qualifications. Stage 4 implements current held-rate annual earning calculations and dated selection; broader provider/term calculations remain outstanding.
- [ ] Model composite/share positions and dated underlying conversions without double counting receipt and underlying value.
- [x] Identify later-compatible boundaries for lock terms, collateral, claims, risk evidence/dependencies, eligibility, and shared access without implementing every later entity now.
- [x] Define whether source revisions/mapping corrections need an immutable audit trail; current stable-ID effect replacement is not append-only history.
- [x] Append ordered idempotent migrations for the first portfolio/account and exact-unit slice; preserve data/import markers, transactionality, rerun-at-every-open behavior, and rollback/retry acceptance cases. Later schema changes must follow the same contract.
- [ ] Validate the proposed model with meaningful examples: Nexo debt, Binance Earn, Kraken ledger fees, a different-currency reward, partial disposal, changing principal, and a composite DeFi position.

Implemented foundation: portfolio/account records and scoped import/query APIs; arbitrary-precision units; scale conflict rejection; exact Nexo/Binance parsing; migrations 004/005. Existing provider entry points retain legacy IDs. The [stage-3 metadata slice](../spec/ui-metadata.md) now persists product/held-investment, dated evidence and action foundations. Remaining work includes connections/reviewed economic asset mappings, provider provenance/coverage population, financial projections, account selection in application workflows and deliberate legacy source remapping. Cached Nexo/Binance upgrade checks operate on temporary copies.

## 2. Provider and acquisition-source documentation

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

## 3. Kraken foundations and remaining coverage

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

The package/stack/token foundation is promoted to section 0. These remaining components support the app first, then the public website.

Package foundation and first-app primitives are implemented in [stage 5](../spec/web-design-system.md). The showcase is synthetic; real application and broader website components remain outstanding.

Reference: [Taskstream structure notes](../findings/taskstream-reference.md). Its root package is named `taskstream` at `/Users/me/src/me/todo-lofi`; this is a structure reference, not a runtime dependency on that checkout.

- [x] Add reusable first-app buttons/fields, grouping/search/navigation, expandable allocation tables, inline edit/pin, nonmodal details, warning/value and empty/loading/offline/error states.
- [ ] Add broader filter/settings widgets and public/paid feature indicators as the real app/site needs them.
- [x] Add holdings/earnings/price formatting with full currency amounts with thousands separators; USD uses two decimals below $100, whole dollars at $100 or more, and “<$0.01” for positive nonzero sub-cent amounts, preserving signs for costs and exact underlying values; trim token quantity trailing zeros, retain small nonzero quantities and expose exact quantities in details subject to privacy mode. Exact rational/BigInt formatting and known/partial/unknown/not-applicable states are implemented; no financial calculations move into the package.
- [ ] Add chart primitives, accessible chart summaries and reusable date displays with the planned history/site work.
- [ ] Add marketing/docs components, navigation/sidebar/TOC, prose, callouts, code blocks, and shared page layouts.
- [x] Provide clean Panda codegen/import conventions usable by the public site, Solid web app, and Tauri frontend; keep generated output reproducible.
- [x] Add an isolated synthetic consumer, typechecking, exact formatting tests, production-build/browser/axe checks and light/dark/narrow visual review; confirm a clean frozen offline rebuild.
- [x] Keep Taskstream as structure reference only: new components are implemented here with no copied source/product content or sibling runtime dependency. Public release/licensing decisions remain in the release tasks.

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

Initial build lanes, clean-checkout dependencies and frontend/Tauri build coverage are promoted to section 0. Remaining packaging/release tasks follow as the app becomes distributable.

Assumption for “3 systems”: **Linux, macOS, and Windows**. CPU architecture coverage, packaging formats, and signing policy are separate choices.

- [ ] Pin/document toolchains and native DuckDB/C++ dependencies; support Windows-native builds as well as the current Linux/macOS Nix environment.
- [ ] Cache dependencies/build outputs responsibly and use lockfiles for reproducibility.
- [ ] Run formatting, appropriate linting, deterministic unit/mapping/storage tests, and documentation checks on pull requests.
- [ ] Audit existing private/live/cache-first tests, including non-ignored real-account tests; keep ordinary CI free of financial credentials and private fixtures.
- [ ] Add an explicitly opt-in private/live reconciliation lane with controlled secrets and no publication of private artifacts.
- [ ] Publish per-platform build artifacts and smoke-test install/startup/migrations with temporary data; decide release packaging and supported architectures.
- [ ] Add release tagging, changelog/versioning, checksums, and later signing/notarization/update-delivery decisions before distributing production desktop bundles.

## 7. Foundations and holdings workflow

The desktop shell and primary holdings/status/refresh/settings views are promoted to section 0. These supporting ingestion/model tasks follow the agreed UI scope.

- [ ] Later, after the existing-data UI: add account connection setup and CSV imports inside the app, reusing provider mappings and reconciliation.
- [ ] Later, after the balances/allocations/current-earnings UI: add transaction-history views for selected assets/providers with source and reconciliation context.
- [ ] Later, lower priority: add custom groups spanning providers in the detail view for an asset selected in by-asset navigation. Define asset-scoped presentation memberships without globally reparenting source accounts or double counting holdings; preserve provider-specific trees.
- [ ] Expose Nexo/Binance/Kraken connection/import/reconcile/query flows in normal CLI commands with account selection and one chosen portfolio DB.
- [ ] Remove Nexo's dated default export path; support managed selection/import of multiple original files, stable IDs, and hash deduplication.
- [ ] Target history from first platform activity; recover exports/manual evidence only where automatic acquisition cannot fill it.
- [ ] Recover missing Airtable tables/definitions only where they resolve confirmed wallet/position/term needs; safely preserve unresolved rows/provenance without importing the entire research catalogue as holdings.
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
- [x] Identify SafePromo and INS Promo; recover canonical historical decimals and retain raw RPC evidence through `evm --resolve-tokens`. INS Promo scale is zero; known destruction still causes a 777-unit ledger discrepancy. SafePromo's fake constant balance and missing scale remain unnormalized, with an explicit reviewed exclusion now available.
- [x] Implement optional Blockscout universal API authentication/pacing and configured Etherscan routing for Polygon/Arbitrum. Etherscan history works for both; the supplied Blockscout key accesses Arbitrum but needs a higher plan for Polygon.
- [x] Select public Polygon dRPC after verifying oldest transaction availability absent from the previous RPC.
- [x] Implement reviewed SafePromo exclusion without fabricated decimals: persisted account policy, reversible review trail, raw DB snapshots, totals/earnings eligibility filtering and strict tracked-scope reconciliation.
- [ ] Add further nonfinancial contract reviews only with evidence; user dust/spam declaration UI is promoted to section 0.
- [ ] Model proven destroyed/retired-contract losses with effective timestamps and evidence; preserve historical units instead of inferring a balancing transaction from zero RPC balances.
- [ ] Obtain exact Classic-era Arbitrum feeStats and map fees; Nitro receipt multiplication and Blockscout's fee field reproduce an approximate value, leaving an 87,468-wei difference in the first capture.
- [x] Recheck Polygon/Arbitrum metadata at their frozen cutoffs; five Arbitrum contracts and one Polygon contract still have getter execution failures with RPC code 3 (the final Arbitrum pass fails four balance calls and one decimals call). Preserve method-specific failure evidence.
- [ ] Investigate the four Polygon contract discrepancies (PolyDough, DxDex.io, lookalike UЅDС, Zepe) and the six contracts with reverting getter calls; implement only evidenced protocol/accounting mappings or reviewed tracking classifications. Native Polygon and 41 readable token quantities match.
- [ ] Add partial-page acquisition resume, Etherscan split-range backfill beyond 10,000 rows, robust reorg rollback and block-aware historical queries.
- [x] Map native/internal transfers, ERC-20 receipt events and failed-transaction gas with stable chain/account identities; swaps retain actual transfer effects.
- [ ] Add semantic swap/protocol classification and unsupported token/fee mechanisms.
- [ ] Discover holdings and DeFi positions automatically; extend the implemented reviewed exclusion foundation with user-facing spam hiding and position/exposure/earnings filtering.
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

- [ ] Add a portfolio switcher alongside family/client tracking; keep the first UI scoped to one portfolio.

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
