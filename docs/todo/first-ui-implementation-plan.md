# First UI — ordered implementation plan

Status: **stages 1–6 implemented; stages 7–9 remain proposed and deferred**. The user authorized stages 1, 2, 3, 4, 5 and 6 separately; inclusion here does not authorize the remaining stages. Grounded in repository source on 12 October 2026. The [master todo](master-todo.md#0-now-first-usable-desktop-app--ui) remains the canonical completion checklist. This document sequences its existing tasks and defines dependencies and completion evidence; it is not a second backlog.

The master todo covers the agreed feature scope. What it lacked was an explicit implementation order, a minimum schema/query slice, and intermediate working milestones. Follow the [desktop contract and acceptance scenarios](../spec/desktop-app.md), [design-system package plan](../spec/web-design-system.md), and [data-model proposal](../spec/data-model-proposal.md).

## Starting point and boundaries (before stage 1)

| Current evidence | Consequence for the first UI |
| --- | --- |
| [Store](../../libs/lib-core/src/storage.rs) opens DuckDB and runs migrations; exposes portfolio/account identity, exact balance queries and protected scales. | Extend these contracts; preserve existing IDs, transactions, amounts, import hashes and cached replay. |
| [Account](../../libs/lib-core/src/identity.rs) has provider/portfolio identity and source reference, but no parent or local-name override. `save_account` updates the source label. | Add separate hierarchy/customization metadata; source refresh must not overwrite local labels/parents. |
| Products and position descriptions are transient; `Product.apy` is one `f64` in [types](../../libs/lib-core/src/types.rs). | Persist actual held subscriptions and dated rate evidence; do not interpret adapter placeholder 0% as verified idle yield. |
| Current metadata queries are low-level; `position_metadata` is unscoped, while exact portfolio/account balance APIs exist. Historical valuation returns floating-point values. | Build explicit portfolio-scoped UI projections; retain exact units and introduce decimal-safe current valuations/rates rather than exposing old float helpers as exact. |
| Reviewed exclusions persist at account/asset scope; raw source history remains available. | Extend policy to asset-wide spam and separate provider/asset dust presentation, preserving strict reconciliation. |
| Kraken/EVM reconciliation lives in CLI adapters and returns reports/errors; observations exist but generic persisted issue/coverage state does not. | Extract structured results and retain source/cutoff provenance; do not infer historical completeness from snapshot equality. |
| [CLI config](../spec/cli.md) has environment/flag paths, but config-file loading is absent and the current OS-based default differs from the agreed Unix default. | Implement shared path resolution before desktop opening/copying. Do not silently abandon an older DB. |
| No frontend package, pnpm workspace, Tauri host or CI workflows exist. [Cargo.toml](../../Cargo.toml) uses `libs/*`/`apps/*` members and an external `typed-ids` path. | Introduce explicit Rust/TypeScript workspace boundaries and make clean-checkout builds independent of that external checkout. |

Scope is one local portfolio, existing imported data, current holdings/allocations and current annual earnings with truthful unknown/partial states. Account connections/CSV imports in the UI, transaction history, historical graphs/benchmarks, APY-history editing, manual allocation splitting, cross-provider custom groups and portfolio switching remain outside this first UI milestone. Historical earnings and graphs remain early follow-up work; this plan does not remove them from the broader roadmap.

## 1. Stabilize application contracts and build boundaries

Canonical tasks: master todo sections 0, 1, 2 and 6.

Implemented: [shared services and exact DTO contract](../spec/application-services.md), provider extraction, explicit Rust membership, removal of unused `typed-ids`, tracked Cargo lockfile and an initial Linux CI workflow. The CLI consumes the shared library; 129 deterministic tests, formatting, CLI help and isolated-checkout metadata passed locally. Hosted CI remains unverified. Current-value/earnings UI projections and all frontend work remain later stages. The baseline table above records the pre-stage-1 gaps.

Define a reusable Rust application-service boundary, provisionally `libs/lib-app`, depending on core/provider libraries rather than the CLI executable. The CLI and eventual Tauri host consume the same services. Extract adapter functionality incrementally; preserve existing CLI behavior and provider fixtures. Keep ingestion, valuation and verification in Rust and reusable presentation in the TypeScript design system.

Specify typed read/write DTOs before binding screens. Serialize integer units, exact decimals and rates as strings with asset scales; never use JavaScript numbers as financial truth. Stable IDs, source/as-of timestamps, calculation basis, coverage and issue references accompany values. Unknown and not-applicable must differ from zero. Group expansion/search/privacy are presentation state, not query filters that change the portfolio's economic perimeter.

Adjust Cargo membership so TypeScript-only `libs/web-design-system` is not treated as a Rust crate; register the planned Tauri crate explicitly, for example `apps/desktop/src-tauri`. The unused `typed-ids` dependency was removed after reviewing its actual use; no replacement package was needed. Choose compatible Tauri/Node/pnpm/tool versions when implementation begins; no scaffold/version selection is required by this documentation task.

Completion evidence: documented service/DTO contracts, existing Rust checks preserved, clean-checkout Cargo metadata succeeds without sibling repositories, and an initial CI lane excludes private/live tests. Design-system work in stage 5 can start once these DTO/presentation boundaries are settled; it does not need all backend stages finished.

## 2. Make database opening and settings reliable

Canonical tasks: master todo section 0 database/configuration/startup entries.

Implemented: [shared configuration, database startup and settings contract](../spec/database-startup.md). JSON path/preferences loading, legacy-location detection, explicit copy/new actions, source-preserving validated installation, portfolio selection and a dedicated serialized DB worker are delivered. CLI configuration uses the shared resolver; the future Tauri screens remain stage 6, and refresh execution/scheduling remain stage 8. Settings live in the local config file; stage 3 now persists DB-backed action/scheduling state, while scheduler execution remains stage 8. Validation passed: 138 deterministic workspace tests, an offline preservation check on temporary copies of all three cached Nexo/Binance DBs, formatting, metadata and CLI help. Native Windows runtime checks remain for the cross-platform lane.

Implement shared path/config resolution: Unix `~/.data/money-tracker/portfolio.duckdb`; desktop `APP_DATA_DIR` → config `data_dir` → default, while retaining explicit CLI flags as its highest priority. Specify config format/location and a native Windows default. Detect and explain the previous location; opening or copying an existing DB is an explicit choice, never a silent empty replacement.

Open one explicit portfolio, normally `local:default`; never merge portfolios found in the same DB. For an unusual multi-portfolio DB, require explicit configuration of the portfolio ID until the later switcher exists. Missing DB offers Copy existing or Start new. Copy to a temporary destination and validate before installing it, protect the source, refuse overwriting an existing destination and handle a source currently in use. Corrupt/unreadable DBs show an error.

Keep DuckDB access behind a managed worker/serialized service boundary, off the UI thread. Define close/cancellation and contention handling if the CLI also has the file open. Load settings locally: reporting currency, theme, privacy mode/scale, dust threshold, refresh interval/launch toggle and active portfolio. A new empty DB has a clear empty state; UI account setup is deferred.

Completion evidence: path precedence tests, temporary-file copy/new/error scenarios, existing DB preservation and repeat opening with ordered idempotent migrations. No provider fetch occurs merely because a DB is opened.

## 3. Persist the minimum UI metadata and action model

Implemented: [minimum UI metadata and action contract](../spec/ui-metadata.md), migrations 007–009, conservative bootstrap and immutable evidence/typed local-state APIs. The managed worker exposes scoped metadata and atomic action/Undo primitives. Existing providers retain financial identities/data; the EVM snapshot supplies evidenced display locators/tickers. Settings remain in the stage-2 JSON file; refresh scheduling state is now persisted separately in DB. Current financial projections are delivered in stage 4; provider rate/verification population and editing controls remain later stages. Validation passed: 149 deterministic workspace tests, an offline preservation check on temporary copies of all three cached Nexo/Binance databases, formatting, locked metadata and CLI help.

Canonical tasks: master todo sections 0 and 1; detailed entity rules in the data-model proposal. Append small ordered migrations in [migrations.rs](../../libs/lib-core/src/migrations.rs), rerun each at every open and keep changes transactional. The table below describes logical responsibilities; the linked implemented contract lists the actual tables and current boundaries.

| Proposed records/extensions | Required behavior |
| --- | --- |
| Account hierarchy/customization | Nullable parent, grouping/source node kind, local label separate from source label, user-selected parent flag and provider mapping version. Validate cycles, portfolio/provider scope and stable identities. |
| Persistent asset/provider display metadata | Exact IDs, known names/tickers/locators/source aliases and unresolved labels. Seed from evidenced adapter metadata; ticker matches never merge assets. |
| Product and held subscription metadata | Product template distinct from subscription instance; account/base asset/source references, lifecycle/lock/maturity/renewal fields and position links. Unknown attribution remains aggregate; no fabricated terms. |
| Subscription/aggregate/borrowing rate evidence | Effective and observation/update times, exact rate, APR/APY/compounding basis, reward denomination, source/revision and nullable unknown. Preserve each subscription's opening/later changes separately from advertised product offers. |
| Override selection | Scoped manual rates/prices, provenance, effective date and pin policy; preserve provider evidence and newer updates. A backdated correction does not erase later applicable periods. |
| Current price observations | Exact decimal quotes where source evidence supports them, source/as-of/fetched times and manual selection policy. Preserve existing historical price rows without falsely upgrading their precision. |
| Tracking/presentation policy | Asset-wide spam by exact identity, existing reviewed account exclusions, provider/asset dust Auto/manual states and global threshold. Liabilities remain visible; dust never changes totals. |
| Observation/verification state | Account/position scope, independent source/cutoff, completeness, ledger/policy revision, verification outcome and structured issue records. Keep balances separate from ledger events. |
| Persistent manual action log | Ordered action IDs, target/portfolio, action type, exact prior/resulting local state, revision links and undo relationship. Commit mutation and action atomically; validate compensating actions on Undo after restart. |
| App refresh/settings state | Persist settings and scheduling state needed to detect missed refreshes and coalesce launch/catch-up work; distinguish last attempt from successful source updates. |

Provider default hierarchy mappings initialize only uncustomized/new nodes. Backfill existing Nexo/Binance legacy accounts conservatively without rewriting their transaction/position IDs. Preserve Kraken/EVM account namespaces. Aggregate Nexo exports do not establish individual fixed-term subscriptions; show that limit. Rates/terms and historical coverage stay unknown where not evidenced.

Completion evidence: rerun/no-op, rollback/retry and DB-upgrade cases; counts and exact Nexo/Binance balances/import hashes unchanged. Show two same-product subscriptions with different rates, a later rate change, pinned evidence, asset-wide spam on a newly discovered account, and action log survival after close/reopen. Use synthetic data and copies of private DBs for upgrade review.

## 4. Build portfolio read models and current earnings queries

Implemented: [snapshot portfolio queries and calculation contract](../spec/portfolio-queries.md), exact BigInt rational money/annual fractions, scoped overview/navigation/allocation/details/issues, unique-position rollups, dust and selected metadata, independent quantity/verification selection, honest partial/unknown figures and gross-assets APY. The worker binds saved settings and selected portfolio; a synthetic wire fixture supports frontend work. Provider population, historical reward adjustment, tier/cap/qualification/fee evaluation and frontend controls remain deferred. There is no float-price upgrade or projection cache. Validation passed across 162 deterministic workspace tests, the offline three-database preservation check, formatting, locked metadata, CLI help and documentation links.

Canonical tasks: master todo sections 0 and 1, with the minimum current-earnings slice of section 8.

Provide one consistent projection per requested portfolio/cutoff/revision, using unique economic positions as the base. Resolve ledger-derived balances and usable independent observations explicitly; retain each source's timestamp and verification status. Display trusted current observations with warnings when history mismatches, without manufacturing balancing movements or calling mixed timestamps a common verified cutoff.

Implemented application queries (the original proposed responsibilities retained below):

| Query | Output |
| --- | --- |
| `overview` | Gross assets, liabilities, net current value, net annual earnings and completeness/source information. UI Overview renders only current value and current annual earnings. |
| `navigation` | Asset/provider summaries, names, value-weighted APY, Dust entry, ordering and scoped warnings; no heterogeneous quantity sum. |
| `allocation_tree(selection)` | Account/product/subscription nodes, direct/root position references, quantities/values, rate and annual earnings/costs, metadata and child IDs. |
| `details(target)` / `issues(scope)` | Local/original names, source references, applicable evidence/pin state, term metadata, exact quantities, affected warnings and withheld/partial figures. |
| `settings` / `manual_actions` (existing worker primitives) | Persisted app preferences and available manual-action restoration information. |

Select current rates from resolved effective periods, not today's advertised offer or the newest timestamp in an unrelated historical period. Define APR-to-annual-earnings versus APY conventions, compounding, rate tiers/caps, costs and reward denomination with worked examples before calculation code. Current annual earnings answer what the current position earns per year, not a renewal/switching forecast. Preserve the agreed separate immediate rate and reward-relative historical adjustment: use transaction-derived evidence when available; show unavailable adjustment when it is insufficient. Completing the historical reward/lot engine is not required to show known immediate current earnings honestly.

Compute net annual earnings as gross earning amounts minus annual borrowing costs; debt magnitude × applicable rate gives costs, deducted once. Overall APY is net annual earnings / gross assets, including known 0% idle allocations. Unknown rates are not zero; show a partial known earnings subtotal and withhold the scope's overall APY until rate coverage is complete. Zero gross assets gives undefined APY. Missing prices/costs and verification limits carry through rollups.

Parent summaries include direct holdings plus descendants once. Product APY is a rollup of held subscriptions. Exclude spam everywhere in totals/earnings, include priced dust, and keep unpriced nonzero holdings visible. Removing closed/zero subscriptions from the current tree does not delete historical records. Test the confirmed $1,000 + $0.50 dust asset total, 4% weighted ETH example, debt-negative earnings and unknown/partial cases. Never use rounded/scaled display values for calculations.

Completion evidence: deterministic Rust query fixtures match financial examples, two portfolios/accounts remain isolated, warning scopes are structured and no fake products/rates or double-counted receipt/underlying holdings appear. Supply documented fixture DTOs to frontend development.

## 5. Create the separate design-system package

Implemented: [package/component contract](../spec/web-design-system.md), explicit pnpm workspace with pinned dependencies/lockfile, SolidJS/Panda 2/Ark wrappers, prefixed full stylesheet and exact display/privacy helpers. A separate Vite/Solid consumer uses `workspace:*` with synthetic fixtures and no Panda config. Validation passed: 8 formatting tests, 6 Chromium interaction/accessibility tests, light/dark/narrow visual review, typechecking, production build and an isolated frozen offline rebuild. A Linux CI lane is configured but hosted execution remains unverified. No real Tauri app, persistent edit binding, homepage, charts or publication is included.

Canonical tasks: master todo sections 0 and 4; follow the [package plan](../spec/web-design-system.md).

Create the root pnpm workspace and `libs/web-design-system` with SolidJS, Panda CSS and Ark UI. Commit its lockfile, ignore generated outputs and expose documented component/style exports. The eventual Solid app uses `workspace:*`; both Tauri and the later web app share those components. Choose versions and prove Panda generation/build from a clean checkout.

Implement the neutral light/dark/system token foundation and minimum app primitives: controls, grouping tabs, search/nav entries, expandable table, inline edit/pin controls, partial/unknown/status displays, right-side details panel and Undo affordances. Keep portfolio services and persistence out of this package. Add currency/exact-quantity display helpers with confirmed thresholds, signed costs, privacy masking/scaling and accessible states.

Completion evidence: isolated synthetic showcase, typecheck and consuming build; keyboard/focus, expansion-versus-details, reduced motion, supported style variants and light/dark visual review. A showcase alone is not the portfolio app milestone. Website/marketing components and charts stay later.

## 6. Deliver the first real-data desktop slice

Implemented: [local Tauri/Solid portfolio browser](../findings/desktop-stage-6.md), using the shared design system and one Rust-owned exact snapshot. Includes missing-DB copy/new flow, Overview, asset/provider search/navigation, scoped expandable allocations and details/warnings, liabilities, saved settings, system theme and privacy. No provider/data-model mutation or new migration. Current rate/price edit UI remains stage 7, refresh remains stage 8, and cross-platform native acceptance remains stage 9. The requirements below record this stage’s design; final validation is recorded in the findings.

Canonical tasks: master todo section 0 shell/navigation/allocation/details entries.

Create the Tauri host and shared Solid frontend using the design system. Expose only typed service commands needed by the app; constrain filesystem actions to configured DB selection/copy flows. Do not put credentials, SQL or financial computations in frontend components.

Connect startup and stage 4 queries first: existing DB → Overview → by-asset/by-provider navigation → selected allocation tree → scoped details/warnings. Expand only immediate account children initially; name clicks open the right panel while arrows expand. Sort current values descending, retain state when details open and label missing attribution. Show debts in the same table with separate amounts/costs. Keep interface usable with partial data rather than hiding unresolved providers.

Add settings, system/theme override, privacy modes and formatting. Hidden/scaled amounts must also apply in details, errors, hover and accessibility text. The first real-data slice can show unavailable annual earnings until its rate evidence exists; completing the milestone requires working current-rate edits/calculations, not fabricated fixture yields in the real-data view.

Completion evidence: run against a temporary DB and a copy of existing imported data; validate read-only navigation and financial consistency with Rust DTO fixtures. The app opens without cloud login or subscription and does not mutate source transactions when browsing.

## 7. Connect manual edits, organization and persistent Undo

Canonical tasks: master todo section 0 manual override/classification/grouping/Undo entries.

Implement commands for rate/borrowing/price edits, pin/unpin, local names, create group, Move to, dust Auto/manual and spam exclusion/restore. Require stable scope IDs and expected revisions, validate exact inputs in Rust and commit each change with the action log. Inline edits default rate effective dates to today and permit earlier corrections. Borrowing costs update as negative earnings. Warn on finer-data aggregate-override review while retaining pinned overrides.

Invalidate/requery affected projections after commit. Clear only issues resolved by the override; unrelated reconciliation mismatches persist. Undo reloads from DB across restarts, records a compensating action and checks conflicts/cycle constraints. Do not undo provider refresh/imports or overwrite unrelated newer state.

Completion evidence: actual edit → updated summaries → restart → Undo scenarios for each required manual change. Include newer source evidence, pin protection, partial finer allocations, source-label refresh, invalid parent and policy restoration cases. No historical-rate editor or manual principal allocation splitting is introduced.

## 8. Add refresh lifecycle using existing acquisition paths

Canonical tasks: master todo section 0 refresh/settings entries; reuse provider acquisition tasks rather than expand coverage.

Refresh already configured accounts and price/rate sources through the shared application services; UI connection setup remains deferred. State provider prerequisites clearly: the Nexo CSV path can only re-read available files, not claim live Nexo balance acquisition; unsupported rate endpoints leave rates unknown/manual. Desktop credential availability must be explicit because a GUI launch does not necessarily inherit a terminal's direnv environment. Reuse existing locally configured credentials without logging them; do not add OAuth onboarding here.

Default to daily while running. Ordinary refresh-on-open is off; missed schedules trigger one catch-up, coalesced with an enabled launch refresh. Keep last saved figures/freshness and structured errors on partial failures. Persist scheduling state, avoid overlapping refreshes and run I/O off the UI thread. Manual refresh stays in options. Closing the window or explicit Quit cancels work and ends the process, with no tray/background process.

Completion evidence: controlled-clock missed-schedule/coalescing cases, partial-provider failures, cached/offline use and close/reopen lifecycle. Existing strict provider reconciliation/cached-replay cases still pass; network tests remain opt-in.

## 9. Finish cross-platform acceptance and build delivery

Canonical tasks: master todo sections 0 and 6. Start basic CI during stage 1 and extend it as packages appear; do not postpone integration failures until this stage.

Run clean-checkout Rust and pnpm typecheck/codegen/production builds, then Linux/macOS/Windows Tauri build lanes. Keep deterministic synthetic data in CI and private financial data/keys out of artifacts/logs. Validate the [desktop acceptance scenarios](../spec/desktop-app.md#first-version-acceptance-scenarios--proposed), startup migration preservation, offline/partial/missing-price views, privacy, accessibility, Undo persistence and refresh lifecycle. Smoke-test built app startup with a temporary DB on each supported system.

Record validation results and limitations, then update canonical master todo states and implemented contracts. Decide release packaging/architectures/signing before wider distribution through the existing release tasks; a local first build does not imply publication or auto-update support.

## Milestones and critical dependencies

| Milestone | Stages required | Visible result |
| --- | --- | --- |
| Contracts and migration foundation | 1–3 | Existing data preserved; metadata/settings/actions ready. No UI completion claim. |
| Real-data browse | 1–6 | Tauri opens imported holdings with allocations/details and truthful partial earnings. |
| Complete first UI behavior | 1–8 | Current-rate edits, debt costs, grouping/classification, persistent Undo and configured refresh work. |
| Validated first build | 9 plus prior stages | Acceptance evidence and reproducible three-system builds. |

The design system can progress alongside backend metadata/query work after contract design. A fake-data prototype may help visual review but cannot replace real-data validation. Do not wait for new providers, a complete opportunity catalogue, risk service, routing, sharing/sync or the full historical calculation engine before building this UI.
