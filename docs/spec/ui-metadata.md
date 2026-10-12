# Persistent UI metadata and manual actions

Implemented first-UI stage 3 on 11 October 2026. Sources: [typed records and Store APIs](../../libs/lib-core/src/ui_metadata.rs), [exact decimals](../../libs/lib-core/src/decimal.rs), [migrations](../../libs/lib-core/src/migrations.rs), [metadata service](../../libs/lib-app/src/service.rs), and [managed worker](../../libs/lib-app/src/worker.rs). [Current-value/earnings projections](portfolio-queries.md) are implemented in stage 4; provider rate acquisition, refresh execution and frontend controls remain later stages.

## Storage and upgrades

Append-only migration ordering remains mandatory: every Store open reruns every step transactionally. Stage 3 appends:

| Step | Tables and behavior |
| --- | --- |
| `007_ui_catalogue_hierarchy` | Account structure, provider/asset display records, namespace-qualified alias candidates, product templates, subscriptions, unique subscription-position links. Seed only missing records from existing exact IDs; preserve source labels and local preferences. |
| `008_ui_evidence` | Immutable identified rate, exact-price, independent-balance and verification records. No legacy float-price conversion or synthetic rate/coverage backfill. |
| `009_ui_actions_policy_refresh` | Revisioned local state, ordered manual actions, portfolio/exact-asset spam decisions, portfolio ledger/policy/metadata revisions and per-source refresh state. Extend `active_asset_exclusions` to union existing account reviews with asset-wide policy. |

Records use typed Serde payloads in `VARCHAR`, with explicit primary/scope keys for identities, subscription ownership, unique position attribution, local targets and ordered actions. Financial decimals and units serialize as strings; payloads are decoded in Rust, without DuckDB JSON extensions or downloads. This is a minimum metadata store, not a normalized event/lot engine or a catalogue performance claim. Later read models can add query indexes/projections as needed.

Existing transaction/position/account IDs, effects, scales, import markers, observations and historical prices are preserved. Bootstrap keeps accounts flat, source nodes at mapping version 0, and display names/tickers/locators unknown unless evidenced. Original asset IDs remain available as unresolved labels. It creates no subscription, term, rate or independent verification. Newly registered/imported assets and accounts receive the same conservative metadata. Nexo remains an aggregate export; Binance legacy buckets and Kraken/EVM namespaces retain their identities.

EVM import/cached replay additionally persists the exact chain/contract locator and source-reported ticker from its frozen snapshot, retaining the snapshot hash as evidence. Unresolved contracts retain their reason/label; no scale is invented. Provider display is `EVM wallets`. This metadata does not merge identically named tokens or change movements, gas, import hashes or strict reconciliation.

## Accounts, products and subscriptions

`AccountStructure` stores Source/Group kind, nullable source parent and mapping version. `AccountCustomization` stores a local label independently of the original account label, plus an explicit parent override flag; that flag distinguishes a user-selected root from following provider defaults. New provider grouping nodes can be registered without a connection or financial movement. Source structure updates require a newer mapping version; replaying the same version must have identical content.

Source updates never change local state. Parent validation rejects self-parenting, cycles, unknown parents and cross-provider/portfolio links. Metadata reads validate the effective tree before returning it. Clearing a parent override and Undo both revalidate today's tree; they fail rather than restoring a cycle. User group creation/move controls and provider-specific richer default mapping policies are later host/adapter work; bootstrap deliberately preserves the existing flat structure.

`ProductTemplate` is a provider/source-identified description, separate from the legacy transient `Product.apy: f64`. `Subscription` is an account-owned instance with its own ID, base asset, source reference/label, optional opening/closing dates, Unknown/Flexible/Locked status, nullable maturity/renewal state and linked ledger positions. Provider/account/product/base-asset identities cannot silently change. Positions must belong to that account and cannot link to two subscriptions; unsupported aggregate imports remain unattributed. No balances are copied into subscriptions, so grouping cannot introduce another economic value.

Closed records remain stored. [Stage 4](portfolio-queries.md) filters zero balances and closed subscription metadata while retaining any nonzero residual holdings, and produces unique-position allocation rollups. Receipt/underlying exposure relationships and DeFi valuation routes remain separate later work.

Provider/asset display setters preserve exact identity; an established chain/contract locator cannot be edited into a different asset. `AssetAlias` records issuer + namespace + external ID + candidate asset ID and evidence. Multiple candidates are explicitly ambiguous; missing candidates are unknown. No lookup remaps effects, rescales units or selects a candidate by ticker. The first evidence for an identical alias candidate is retained; raw provider snapshots retain later source observations. Full reviewed mapping revision/validity and economic equivalence remain future work.

## Rates, prices and overrides

`ExactDecimal` now lives in core and is reexported at the existing `lib_app::dto::ExactDecimal` path. Parsing/JSON behavior is unchanged: exact canonical plain-decimal strings, no numeric JSON, exponents, floating-point rounding or scale truncation.

A `RateEvidence` identifies portfolio, scope, effective time, observed/update times, source/reference and nullable `AnnualRate`. Scopes cover subscription, held-product aggregate, account/asset, provider, asset, provider/asset and a specific liability position. Held-product rates are aggregate evidence/overrides, not advertised offers copied into subscriptions. Annual rates use a nonnegative fraction (`"0.05"` = 5%), APR/APY basis, optional compounding frequency and reward denomination. Borrowing has its own liability scope; the calculation layer deducts its cost. Missing rate differs from known zero. Existing adapter placeholder 0% values are not imported as evidence.

Subscriptions have separate opening/later rate records. An opening rate is stored only if evidenced; otherwise the opening period remains unknown until a later observation/correction. Scope/portfolio mismatches and immutable evidence-ID reuse with different payloads are rejected. Replaying an identical evidence record preserves timestamps and revisions.

Local override state stores dated manual rate/price periods with update times and pins; prior states remain in the action log. Selection first limits records to applicable effective dates, preserves later periods when correcting an earlier one, and resolves updates within a period by update time (manual wins an exact tie). The latest applicable manual period, if pinned, remains selected despite new provider evidence. Otherwise a newer applicable provider period/update supersedes an older manual value. Pinned historical corrections do not erase later manual periods. Selection returns the source evidence ID or a manual indication; it does not compute annual earnings.

`PriceEvidence` stores exact asset/quote, as-of time, observed/update evidence and quote value. Manual prices use the same dated/pin policy, scoped to one portfolio. Zero can represent an explicit loss valuation; negative prices are invalid. Historical `asset_prices` doubles remain unchanged and are not relabelled as exact or copied automatically.

`override_requires_review` detects changed relevant evidence or finer held-subscription/product/account/provider-asset data, including partial detail. It retains the override and its pin; the future UI must prompt review rather than automatically invalidate it. [Stage 4](portfolio-queries.md) resolves allocation precedence and avoids overlapping aggregate/subscription earnings; review/editing controls remain stage 7.

## Tracking, verification and refresh state

Spam policy is portfolio-wide for an **exact asset identity**, covering current and future accounts in that portfolio. It unions with existing account/asset reviews; clearing global spam does not undo an independent account review. Global spam requires a reason and accepts unresolved exact IDs without a scale. It changes eligibility in existing balance, price-discovery and historical net-worth queries through the shared exclusion view, preserving raw history. Liabilities remain included. EVM reconciliation already uses that same tracked-scope policy and retains exact equality.

Dust state is per portfolio/provider/asset: Auto, Dust or Significant. It never changes financial inclusion; global threshold/reporting preferences remain in the stage-2 JSON settings file. Actual dust grouping/calculation awaits the read model.

`BalanceEvidence` records source/reference, account/optional position, exact asset/units/protected scale, Total/Available/Locked/Debt basis, cutoff, completeness and independence. It never inserts effects. `VerificationRecord` retains account/portfolio/cutoff, observation IDs, ledger/policy revisions, outcome, explicit history completeness and structured position/account issues with expected/actual units and withheld figure names. A verified record requires complete independent observations, matching scope/cutoff, exact ledger equality, nonoverlapping coverage and every nonzero tracked position covered. Available/locked evidence alone cannot verify total holdings. A current balance match never sets historical completeness automatically; callers must supply that separately from supported evidence.

Accepted financial writes increment affected portfolio ledger revisions atomically, including replacement effects' former portfolio scopes. Duplicate completed imports do not increment them. Existing account reviews and new global spam mutations increment policy revisions. Reports retain their evidence but `verification_is_current` becomes false after relevant ledger/policy changes. Metadata revision is separately retained; this conservative invalidation applies at portfolio level, not just one position. Existing provider reports are not automatically converted into these generic records; stage 4 consumes generic coverage/issues, while adapter population remains stage 8/provider work.

Per-source `RefreshState` persists next due, last attempt and last successful update separately. Failed attempts may advance last attempt without advancing success. Scheduling/coalescing/network refresh remain stage 8. Stage-2 settings stay in their existing JSON file; there is no competing DB preference store.

## Atomic local changes and Undo

`apply_manual_change(portfolio, target, expected_revision, value)` validates a typed target/value, commits revisioned local state and its `ManualAction` together, and rejects no-op/stale writes. Targets are account customization, subscription name, rate, price, exact-asset spam and provider/asset dust. Clearing uses absent state; revision numbers never reset. Actions have globally ordered IDs/sequences, schema version, portfolio/target, time, exact before/after state, evidence fingerprint and optional Undo link.

`undo_manual_action` applies a compensating action rather than deleting history. It rejects already-undone/other-portfolio actions, newer conflicting local state, invalid restored parents, or changed relevant rate/price source evidence. Validation and mutation are transactional. It never restores provider labels, snapshots, imports, source rates/prices or ledger effects. Source evidence conflicts require review; this stage does not provide a force-Undo UI. The action log and local state survive restarts, including explicit restoration of a missing override.

`Application::ui_metadata` and `DatabaseWorker::ui_metadata` return scoped version-1 metadata with effective parents, source names/references, catalogue records, held subscriptions, revisions and local state. The worker exposes the low-level typed manual change/action-log/Undo primitives using its selected portfolio. Frontend editing commands, controls, warning clearing, display-name selection, Undo affordances and query invalidation remain stage 7; [current-value/earnings calculations](portfolio-queries.md) are now implemented in stage 4.

## Validation

Deterministic core tests cover two same-product subscriptions at different opening rates and a later rate change; backdated corrections, unknown/zero rates, pins, immutable replay and finer-data review; exact prices and unchanged source history; hierarchy scope/cycles and conflicting Undo; ambiguous aliases and protected locators/scales; asset-wide spam on new accounts with debt/audit preservation; exact independent verification and stale revisions; persisted refresh timestamps; action restart/reversal and atomic rollback/retry after an injected action-log failure. A malformed migration seed table rolls back step 007 and succeeds after repair without duplicate backfill. Managed-worker tests verify metadata/local actions and Undo after reopening.

Commands: `cargo test -p lib-core`, `cargo test -p lib-core --test ui_metadata`, `cargo test -p lib-app --test startup`. The deterministic workspace suite keeps provider mappings/replay/reconciliation tests; private/live acquisition remains excluded. The opt-in offline cached Nexo/Binance upgrade test validates unchanged financial records across repeated opens of temporary copies, preserving the originals.

Stage-3 validation passed: 149 deterministic workspace tests (13 opt-in tests ignored; one private-cache test filtered), plus the opt-in preservation check on temporary copies of all three cached Nexo/Binance databases. Formatting, locked offline Cargo metadata, CLI help and documentation links also passed. No live provider acquisition was required.
