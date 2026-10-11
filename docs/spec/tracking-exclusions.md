# Reviewed asset exclusions

Implemented in [core storage](../../libs/lib-core/src/storage.rs), [migration 006](../../libs/lib-core/src/migrations.rs), and the [EVM adapter](../../libs/lib-app/src/adapters/evm.rs).

A review identifies an exact account and asset locator, records exclusion/restoration, reason, evidence, revision and UTC review time. Ticker/name heuristics never create exclusions. Decisions do not require an asset scale: an unresolved contract can be excluded without inventing decimals or assigning it a counterparty role. Current policy applies to every displayed historical period. Identical repeated decisions are idempotent; restoration appends a revision. Decisions persist across startup and cache replay.

Asset balances, asset totals, portfolio totals, net-worth history and price-discovery eligibility omit excluded **asset** positions. For an account-scoped review, the same asset in another account remains included unless another applicable review or [asset-wide policy](ui-metadata.md) excludes it. Liabilities remain included even when their currency has an asset exclusion. `Store::position_is_tracked` is the eligibility contract for financial/earnings inputs; no earnings calculation engine exists yet, and that future engine must apply this policy to principal, rewards and attribution before computing yield. Position metadata is intentionally unfiltered audit information, not an earnings input.

Exclusion never deletes or modifies effects, positions, scales, import markers or independent balance observations. `account_audit_balances_at` exposes unfiltered account quantities. EVM imports additionally preserve the complete frozen JSON snapshot in `account_source_snapshots`, including receipt logs, unresolved contracts and raw metadata evidence. Replaying an already imported cache backfills this evidence. For unresolved SafePromo there are no normalized token effects because no trustworthy scale exists; its raw history is available in both the cache and DB. Native movements and gas from the same transactions still map normally.

EVM reconciliation compares both ledger and independent observations over the tracked scope only, with exact equality and no adjustments/tolerance. Reports identify reviewed exclusions explicitly. An excluded unresolved contract no longer blocks that scope; any other unresolved contract, tracked mismatch, or missing Arbitrum Classic exact-fee coverage still fails. A successful report describes tracked scope, not verification of an excluded token. Restoring an unresolved token reinstates its coverage blocker; restoring a resolved token reinstates its exact balance comparison.

## SafePromo setup

The user authorized the first reviewed classification for Ethereum SafePromo. The [review evidence](../findings/evm-contract-resolution.md#safepromo-a-promotional-contract-not-a-normal-transfer-ledger) records verified source/runtime hashes and the reason. This review is selected explicitly, never automatically applied to a wallet by discovery. No private address is hardcoded.

List saved accounts, then select the Ethereum account ID printed there:

```sh
cargo run -p cli -- --data-dir .cache/data --cache-dir .cache evm --list
cargo run -p cli -- --data-dir .cache/data --cache-dir .cache --offline evm --account "$EVM_ACCOUNT_ID" --exclude-safe-promo
```

`--exclude-safe-promo` requires a saved account and rejects non-Ethereum accounts. Add `--register-only` to record the decision without acquisition/reconciliation. The decision persists even when replay reports another mismatch. To restore tracking without deleting the review trail:

```sh
cargo run -p cli -- --data-dir .cache/data --cache-dir .cache evm --account "$EVM_ACCOUNT_ID" --restore-token 0x31a240648e2baf4f9f17225987f6f53fceb1699a --register-only
```

## Verification and remaining work

Tests cover repeated review/startup, restoration, unknown scales, account isolation, historical totals, pricing eligibility, liabilities, raw effects/observations, preserved snapshots, unresolved exclusions and strict native/resolved-token mismatches. SafePromo is the only built-in review command; other contracts require separate evidence and review before adding classifications.

General exclusion settings/UI, position/product scopes, composite underlying exposure filtering, saved-analysis policy revisions and earnings attribution are future work in [the master todo](../todo/master-todo.md). Current account/asset policies do not implement those broader proposed contracts.

## Proposed first UI controls

The [desktop interview](desktop-app.md) now includes user dust/spam declarations and preserved history in the first UI milestone. The confirmed UI design requires exact-asset-wide spam policy across existing and future accounts; the current account/asset review foundation needs extending to support it. Never propagate by ticker/name alone. The UI is not implemented. Dust is provider/asset-specific, based on the holding at that provider rather than an asset-wide flag, and is a presentation classification rather than a tracking exclusion: include known values/earnings normally, and group dust holdings under one Dust navbar entry while retaining significant holdings of the same asset elsewhere. Individual holdings remain available in detail. Asset navbar/detail totals still include their dust holdings across all providers; Dust is an overlapping filtered view, not an additional balance to add to financial totals. Automatic dust assignment uses a configurable fiat-value threshold at provider/asset scope, with manual mark/unmark and reset-to-Auto controls; the default threshold is strictly below USD 1. Unpriced dust remains subject to partial-total warnings; no invented zero value. Manual price entry resolves missing valuation only, never quantity/scale or independent balance mismatches.

Stage 3 adds portfolio/exact-asset spam state, applied dynamically to existing and future accounts through the same exclusion view. Local mutations and compensating Undo are logged atomically, while existing account reviews retain their own evidence and revisions. Clearing asset-wide spam does not clear an independent account review. Per-provider/asset dust Auto/manual state is presentation only and never changes tracked quantities.
