# Persistence and startup migrations

[Store](../../libs/lib-core/src/storage.rs) encapsulates DuckDB reads and writes. [migrations.rs](../../libs/lib-core/src/migrations.rs) owns the ordered migration registry and schema/data changes.

[Managed application startup](database-startup.md) adds explicit missing/copy/new actions, source-preserving installation, single-portfolio selection and a serialized DB worker around this Store. Local preferences are stored in JSON configuration; stage 2 adds no database migration or financial-table changes.

## Implemented tables

| Table | Stored information |
| --- | --- |
| `portfolios` | Stable ID, label, reporting asset ID |
| `accounts` | Stable ID, economic portfolio ID, provider ID, label, optional source reference |
| `transactions` | Stable ID and UTC datetime |
| `positions` | Position ID, asset ID, explicit economic role, legacy ownership flag, optional owned account ID |
| `asset_scales` | Decimal places per asset |
| `transaction_effects` | Transaction ID, position ID, exact signed integer-text amount, stable effect order |
| `asset_prices` | Timestamp, asset/quote IDs, floating-point quote price; unique per timestamp/pair |
| `price_coverage` | Successfully fetched half-open intervals per asset/quote pair |
| `unavailable_price_periods` | Confirmed unavailable provider/pair intervals |
| `balance_observations` | Independent source/hash/position/timestamp/amount evidence |
| `asset_tracking_reviews` | Account/asset exclusion or restoration revisions, reason, evidence, review time; active exclusions derived from latest decisions |
| `account_source_snapshots` | Immutable complete EVM JSON evidence keyed by account/source/import hash, including unresolved raw history |
| `imported_files` | Source/content hash, transaction count, completion time; unique per source/hash |

Portfolio/account persistence and scoped queries are implemented as described in [the data model](data-model.md). Full provider/connection/product/held-investment metadata remains [future work](../todo/master-todo.md). Transactions and effects support stable-ID replacement and retrieval. Inputs persist negatively and outputs positively. An import marker must commit atomically with its ledger and related registrations/observations; a failed import leaves no successful marker. Independent balance observations are comparison evidence, never synthetic ledger effects.

The [data model proposal](data-model-proposal.md) records the implemented account/quantity foundation and proposed event/provenance revisions, matching registries and reconciliation extensions. The table inventory and migration registry below remain the implemented contract.

## Ordered idempotent startup migrations

Every `Store::open` and `Store::in_memory` must run **all** migrations in explicit registry order before returning a usable store. There is no applied-version gate. Append new steps without reordering existing ones; make every step idempotent against current schema/data.

1. `001_create_tables`: create baseline tables if missing.
2. `002_position_roles`: upgrade ownership-only positions. Owned positions become assets; known `NEXO-CREDIT:` positions become liabilities; other non-owned positions become external counterparties. Preserve existing explicit roles, effects, and import markers.
3. `003_legacy_price_coverage`: seed conservative coverage from dense legacy price runs with gaps at most 48 hours, only for pairs without coverage. Sparse prices do not imply fetched history.
4. `004_exact_units`: inspect the effects amount column and convert legacy signed `BIGINT` to `VARCHAR` losslessly. Skip an already converted column; refuse unknown legacy types rather than attempt a lossy conversion.
5. `005_portfolio_accounts`: create portfolio/account tables and add position membership if missing; insert the default portfolio and deterministic legacy accounts without overwriting metadata; backfill only user positions with absent membership. External counterparties remain outside owned accounts.

6. `006_tracking_reviews`: create review revisions, active-exclusion view and raw source snapshots if missing. Preserve all financial records and previous decisions. See [tracking exclusions](tracking-exclusions.md).

Each step runs in its own transaction. Failure rolls back that step, stops startup, and reports its name. Earlier committed steps must remain safe to rerun on retry.

Acceptance scenarios: repeated startup preserves data and adds no duplicate backfills; newly added qualifying legacy prices gain coverage at the next open; malformed legacy data fails with the migration name and rolls back partial changes; corrected data permits retry. Explicit roles must not be reclassified when reopened. These scenarios are covered by core storage tests.


## Foundation upgrade validation

Deterministic tests cover exact amounts beyond 64/128-bit ranges, sums of individually valid old effects exceeding signed `BIGINT`, rejected scale changes with atomic rollback, account-scoped ID/hash isolation, separate portfolio values, liabilities/counterparties, and preservation of user metadata on repeated opens. Atomic-import failure fixtures now use invalid negative effect magnitudes, since large valid quantities no longer imply failure.

An optional offline test copies the three existing private Nexo/Binance cache DBs into temporary directories and verifies financial records across three opens. It allows the already specified role/coverage upgrades, never modifies the original fixtures, and requires no credentials/network:

```sh
cargo test -p lib-core cached_provider_databases_upgrade_without_changing_financial_records -- --ignored
```

Old binaries that expect numeric effect columns must not write upgraded databases. Use the new core writers/readers together; no automatic downgrade is implemented.
