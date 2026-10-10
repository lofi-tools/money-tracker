# Design: Persistence Layer

## Database Schema

We use the following tables:

### `accounts`
| Column | Type | Notes |
|---|---|---|
| `id` | `TEXT` | Primary Key |
| `name` | `TEXT` | |
| `kind` | `TEXT` | e.g., "wallet", "exchange" |

### `transactions`
| Column | Type | Notes |
|---|---|---|
| `id` | `TEXT` | Primary Key |
| `date` | `TIMESTAMP` | |
| `description` | `TEXT` | |

### `transaction_effects`
| Column | Type | Notes |
|---|---|---|
| `transaction_id` | `TEXT` | Foreign Key -> transactions.id |
| `account_id` | `TEXT` | Foreign Key -> accounts.id |
| `amount` | `DECIMAL` | |
| `asset` | `TEXT` | |

### `asset_prices`
| Column | Type | Notes |
|---|---|---|
| `datetime` | `TIMESTAMP` | Price observation time |
| `asset_id` | `VARCHAR` | Currency being priced |
| `vs_asset_id` | `VARCHAR` | Quote currency |
| `price` | `DOUBLE` | Quote-currency units per asset |

The implemented ledger schema stores each effect with its transaction, position, signed smallest-unit amount, and stable effect order. `positions` maps position IDs to asset IDs and ownership, while `asset_scales` records the decimal places for each asset. Inputs persist as negative balance changes and outputs as positive balance changes. Valuation selects the most recent price at or before each balance timestamp.

## Architecture
- `libs/lib-core/src/storage.rs` encapsulates normal database reads and writes.
- `libs/lib-core/src/migrations.rs` contains the ordered migration registry and all schema/data migration code, clearly separated from normal storage operations.
- `Store` struct will hold the `duckdb::Connection`.
- Every `Store::open` and `Store::in_memory` runs all migrations in registry order before returning a usable store. There is no applied-version gate: each step is idempotent and checks the current schema/data. New steps are appended to the registry; existing steps keep their order.
- Each step runs in its own transaction. Failure rolls back that step and stops startup with the migration name in the error; completed earlier steps remain safe to rerun on retry.
