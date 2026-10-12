# Shared Rust application services and exact DTOs

Implemented stages 1–4 of the [first-UI plan](../todo/first-ui-implementation-plan.md). Stage 1 introduced the [application library](../../libs/lib-app/src/lib.rs), [service](../../libs/lib-app/src/service.rs) and [exact JSON contracts](../../libs/lib-app/src/dto.rs); [stage 2](database-startup.md) provides startup/configuration, [stage 3](ui-metadata.md) metadata/action storage, and [stage 4](portfolio-queries.md) snapshot current-value/earnings queries. The [stage-6 desktop browser](../findings/desktop-stage-6.md) now consumes these services; provider evidence acquisition and historical earnings remain later work.

## Application boundary

`lib-app` depends on core and provider libraries, independently of the CLI executable. All former CLI adapters now live under `lib-app::adapters`; their mapping, source acquisition, price fallback, cache/replay, strict reconciliation and diagnostic behavior are retained. The CLI reexports that module and opens its existing DuckDB through `Application::open`, then passes `app.store()` to the shared adapters. Nexo construction uses its existing `from_path` constructor rather than depending on clap's CLI configuration type.

`Application::open(path)` retains `Store::open` behavior, including ordered idempotent migrations and creation when appropriate. The synchronous constructor does not apply desktop startup policy. Desktop hosts use the stage-2 `DatabaseWorker` for explicit missing-file/copy/new actions, portfolio selection and serialized off-thread access; see [database startup](database-startup.md). The actual chooser UI and refresh scheduler remain later stages. `Application::from_store` supports an already opened store; `store()` is the compatibility boundary for existing import/price/reconciliation flows.

Provider diagnostic tests now run under `cargo test -p lib-app adapters::<provider>::…`; CLI flag/configuration tests remain under `-p cli`. Nix wrappers and provider guides use the new package. The crate relocation retains the same two-directory depth, so existing repository-relative private cache/fixture paths resolve to the same files. Private/live tests remain opt-in except the pre-existing cache-first Binance test, which CI explicitly skips without changing its local behavior.

## Implemented ledger query

`Application::ledger_balances(ReadContext { portfolio_id, cutoff })` returns a version-1 `LedgerBalancesDto`:

- Rejects an unknown portfolio, enumerates its actual accounts and uses exact account-scoped balances and metadata. No ID parsing or cross-portfolio summation.
- Includes asset and signed liability quantities; excludes external counterparties and currently reviewed excluded assets. Liability signs remain ledger signs, not positive borrowed magnitudes.
- Returns stable account/provider/position/asset identities, source labels/references, integer-string units and protected scales. Reads do not mutate labels, imports, tracking policy or transactions.
- Labels evidence `ledger_replay`, with the requested cutoff and verification `unknown`. It never promotes a computed ledger quantity to an independently verified observation or assumes complete historical coverage. A scoped `ledger_history_unverified` issue identifies historical figures that require further evidence.
- Requires registered asset/scale metadata and fails rather than inventing a scale. Retains zero/closed ledger buckets where they exist; filtering current holdings belongs to the later allocation projection.

The new `Store::account_position_metadata` supports this boundary. Existing whole-database metadata APIs and asset-only `portfolio_balances_at` behavior remain unchanged. The application deliberately uses per-account queries to include signed debt.

## Exact transport contract

All DTOs use Serde with explicit string tags. Stable domain IDs serialize as strings. `contract_version` is 1; consumers must check this version before assuming a schema. Version and asset scale are ordinary small numeric metadata; financial values are never JSON floating-point numbers.

| Contract | Representation and meaning |
| --- | --- |
| `QuantityDto` | `asset_id`, signed arbitrary-precision integer-string `units`, and `scale`. `decimal_string()` renders exact quantity without rounding. |
| `ExactDecimal` | Validated canonical plain-decimal string for future quotes, money and rates. Rejects numeric JSON, exponents, NaN, malformed values and excessive inputs. Fraction precision is bounded by the core's `u8` scale; trailing zeros/sign-on-zero normalize without losing value. |
| `MoneyDto` | Denomination `asset_id` and exact decimal `amount`; no UI formatting or presumed USD conversion. |
| `FigureDto<T>` | `known`, `partial` (a labelled known subtotal with issues), `unknown` (no invented value), or `not_applicable`. Known zero differs from unknown. Includes evidence where there is a value. |
| `RateDto` | Annual fraction as an exact decimal: 5% is `"0.05"`; APR/APY basis, optional compounding/reward asset, effective date and pin state. Wrap in `FigureDto` when availability/provenance is needed. |
| `EvidenceDto` | Ledger cutoff, provider source/reference/observed/effective times, or manual action/update/effective times. No credentials. |
| `IssueDto` | Stable issue code, typed portfolio/account/position/asset scope, explanation and affected withheld figures. |
| `VerificationDto` | Unknown, verified-at-cutoff with evidence reference, or dated mismatch issues. Ledger query currently produces unknown only. |

Example quantity: `{"asset_id":"TOKEN","units":"123456789012345678901234567890000000000000000001","scale":18}` renders `123456789012345678901234567890.000000000000000001` exactly. A borrowing liability of `"-2500"` at scale 2 remains `-25.00`.

Money/rate/figure/evidence types establish the shared transport vocabulary; stage 3 now supplies separate persisted rate/price and observation records using the shared core exact decimal. The original ledger query remains independent of these calculations. [Stage-4 read models](portfolio-queries.md) now calculate exact rational current values/rates and select dated independent observations; full host editing commands and generated TypeScript bindings remain later stages. Existing historical floating-point price rows are not relabelled as exact.

## Workspace and checks

[Cargo workspace](../../Cargo.toml) lists its ten Rust packages explicitly, including `lib-app`. TypeScript directories cannot accidentally become Rust members. The unused external `typed-ids` dependency was removed, not vendored. A future Tauri crate must be added explicitly when created. `Cargo.lock` is now tracked for reproducible application builds.

The [initial CI workflow](../../.github/workflows/rust.yml) runs on Linux: locked metadata, formatting, deterministic production-workspace tests and CLI help. It excludes the Polars demo and the pre-existing private cache-first Binance test. Ignored live/private tests are not enabled, and no financial credentials are supplied. The workflow is added locally; a hosted CI run and the later macOS/Windows/Tauri lanes are not claimed as completed.

Validation covers exact JSON round-trips above JavaScript and 64-bit integer limits, malformed/numeric input rejection, known-zero versus unknown, account/portfolio isolation, signed debt, exclusions/counterparties, cutoff selection and repeated file-backed opening preserving legacy Nexo/Binance IDs and balances. Existing provider synthetic mapping/reconciliation/cache tests and CLI configuration tests accompany these new service tests.

Checked locally: 129 deterministic production-workspace tests passed (private/live tests excluded), formatting passed, CLI help ran successfully, and locked offline metadata passed in an isolated copy containing all ten Rust packages plus a dummy TypeScript-only directory. The full suite required permission to bind local mock HTTP sockets; it did not fetch private provider accounts. Hosted CI execution remains pending.

## Stage-3 metadata and mutations

[UI metadata](ui-metadata.md) extends the scoped managed worker with `ui_metadata`, `manual_actions`, low-level `apply_manual_change` and `undo_manual_action`. Version-1 metadata retains original account labels/references and separately exposes source hierarchy, effective parents, local state, held subscriptions and revisions. Amounts/rates still serialize exactly; `ExactDecimal` is reexported from core at its previous DTO path. Host editing controls remain later stages; [current value/earnings projections](portfolio-queries.md) are implemented in stage 4.
