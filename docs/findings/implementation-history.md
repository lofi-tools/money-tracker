# Consolidated implementation decisions

Documentation reconciled with source on 10 October 2026. Earlier proposal/checklist copies have been folded into current topic specifications and the implementation backlog. Git history retains their original wording.

| Earlier work | Retained decision and destination |
| --- | --- |
| Move core types to library (26 November 2025) | Shared types/traits belong in `libs/lib-core`; conversions belong in adapters. See [data model](../spec/data-model.md). Old `bin/` and `libs/core` paths are superseded. |
| Adopt accounting model (27 November 2025) | Model dated position effects and cumulative balances. Earlier single signed Decimal output list was replaced by unsigned input/output lists and signed persisted effects. See [data model](../spec/data-model.md). |
| Replace Decimal with u64 (27 November 2025) | Integer effects with per-asset scale avoid float arithmetic in the core. Storage range and source parsing still require care; `u64` is not sufficient for every possible 18-decimal holding. See [precision backlog](../todo/implementation-gaps.md). |
| CLI arguments and cache (28 November 2025) | clap and OS directory resolution, with CLI precedence over environment/defaults. Correct current names are `APP_CACHE_DIR`, `APP_DATA_DIR`, and `money-tracker`. See [CLI](../spec/cli.md). |
| Persist accounting models | Ledger, observations, prices, and migration registry implemented. Proposed full account metadata remains future work. See [persistence](../spec/persistence.md). |
| Historical net worth | Atomic Nexo file imports, cached price coverage, older-price fallbacks, and signed asset/liability valuation. See [Nexo](../spec/nexo.md) and [valuation](../spec/historical-net-worth.md). |
| Improve Nexo file handling | Configurable paths and hash deduplication implemented; managed uploads/content abstraction/setup remain future choices. See [remaining gaps](../todo/implementation-gaps.md). |
| Complete Binance account history | Exact ledger mapping, independent balances, statement/API replay, legacy Savings evidence, and shared adaptive rate limiting. See [Binance](../spec/binance-account-history.md). |

## Corrections made during consolidation

The old core specification described enum asset IDs, active external-ID maps, and position amounts that no longer match the structs. Old persistence examples described an unimplemented accounts table and Decimal effects. Historical valuation now includes liabilities. The Binance guide's previous 0.07192 SOL deficit was resolved by evidenced legacy Savings maturity/reinvestment, not a balancing adjustment; [the README](../../README.md) records the recovery. Older API-only attempts encountered retention gaps and throttling, which remain distinct from the successfully reconciled frozen statement fixture.

Completed/archived checklists with stale unchecked tasks and repeated proposal scaffolding were removed after their requirements or remaining work were consolidated. The old documentation tool's commands, archive ceremony, placeholder purpose text, and automatic approval gate have been replaced by [the documentation workflow](../README.md). No missing account metadata, managed-upload implementation, complete generic loan lifecycle, or future UI is presented as delivered.
