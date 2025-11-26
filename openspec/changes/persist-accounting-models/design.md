# Design: Persistence Layer

## Database Schema

We will use the following tables:

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

## Architecture
- `libs/lib-core/src/store.rs` will encapsulate all DB logic.
- `Store` struct will hold the `duckdb::Connection`.
- Initialization will run `CREATE TABLE IF NOT EXISTS`.
