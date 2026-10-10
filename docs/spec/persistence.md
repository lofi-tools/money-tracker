# Persistence and startup migrations

[Store](../../libs/lib-core/src/storage.rs) encapsulates DuckDB reads and writes. [migrations.rs](../../libs/lib-core/src/migrations.rs) owns the ordered migration registry and schema/data changes.

## Implemented tables

| Table | Stored information |
| --- | --- |
| `transactions` | Stable ID and UTC datetime |
| `positions` | Position ID, asset ID, explicit economic role, legacy ownership flag |
| `asset_scales` | Decimal places per asset |
| `transaction_effects` | Transaction ID, position ID, signed `BIGINT` amount, stable effect order |
| `asset_prices` | Timestamp, asset/quote IDs, floating-point quote price; unique per timestamp/pair |
| `price_coverage` | Successfully fetched half-open intervals per asset/quote pair |
| `unavailable_price_periods` | Confirmed unavailable provider/pair intervals |
| `balance_observations` | Independent source/hash/position/timestamp/amount evidence |
| `imported_files` | Source/content hash, transaction count, completion time; unique per source/hash |

There is no full `accounts` metadata table. Account/product/position metadata persistence and retrieval remain [future work](../todo/implementation-gaps.md). Transactions and effects support stable-ID replacement and retrieval. Inputs persist negatively and outputs positively. An import marker must commit atomically with its ledger and related registrations/observations; a failed import leaves no successful marker. Independent balance observations are comparison evidence, never synthetic ledger effects.

## Ordered idempotent startup migrations

Every `Store::open` and `Store::in_memory` must run **all** migrations in explicit registry order before returning a usable store. There is no applied-version gate. Append new steps without reordering existing ones; make every step idempotent against current schema/data.

1. `001_create_tables`: create baseline tables if missing.
2. `002_position_roles`: upgrade ownership-only positions. Owned positions become assets; known `NEXO-CREDIT:` positions become liabilities; other non-owned positions become external counterparties. Preserve existing explicit roles, effects, and import markers.
3. `003_legacy_price_coverage`: seed conservative coverage from dense legacy price runs with gaps at most 48 hours, only for pairs without coverage. Sparse prices do not imply fetched history.

Each step runs in its own transaction. Failure rolls back that step, stops startup, and reports its name. Earlier committed steps must remain safe to rerun on retry.

Acceptance scenarios: repeated startup preserves data and adds no duplicate backfills; newly added qualifying legacy prices gain coverage at the next open; malformed legacy data fails with the migration name and rolls back partial changes; corrected data permits retry. Explicit roles must not be reclassified when reopened. These scenarios are covered by core storage tests.
