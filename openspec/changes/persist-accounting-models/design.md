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
| `asset_id` | `JSON` | Currency being priced |
| `vs_asset_id` | `JSON` | Quote currency |
| `price` | `DOUBLE` | Quote-currency units per asset |

The implemented ledger schema stores each effect with its transaction, account, asset/currency, signed smallest-unit amount, and stable effect order. Inputs persist as negative balance changes and outputs as positive balance changes. `ASOF JOIN` valuation selects the most recent price at or before each balance timestamp.

## Architecture
- `libs/lib-core/src/storage.rs` will encapsulate all DB logic.
- `Store` struct will hold the `duckdb::Connection`.
- Initialization will run `CREATE TABLE IF NOT EXISTS`.
