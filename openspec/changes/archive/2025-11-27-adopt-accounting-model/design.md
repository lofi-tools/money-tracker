# Design: Accounting Transaction Model

## Data Model

We are adopting the following data model from `nmrshll/accounting`:

```rust
pub struct Transaction {
    pub outputs: Vec<TxEffect>,
    pub datetime: DateTime<Utc>,
}

pub struct TxEffect {
    pub account_id: AccountId, // or AssetId depending on integration
    pub amount: Decimal,       // Can be negative (credit) or positive (debit)
    pub datetime: DateTime<Utc>,
}
```

### Key Concepts

1.  **Double Entry**: Every transaction should ideally balance to zero (sum of amounts = 0), although for a simple tracker we might have unbalanced transactions (e.g. airdrops, or just tracking one side).
2.  **Unified Inputs/Outputs**: Instead of separate lists, we use a single list of "outputs" or "effects".
    -   **Outflow**: Negative amount.
    -   **Inflow**: Positive amount.
3.  **Granularity**: Each output has its own timestamp, although usually they match the transaction timestamp.

## Integration with Crypto Tracker

In `crypto-tracker`, we currently focus on Assets.
- `account_id` in the accounting model maps to `AccountId` in our system. We might need to ensure `AccountId` can represent "Asset held at Provider".
- Alternatively, if we are tracking *Asset* movements, we might replace `account_id` with `AssetId` or a tuple `(ProviderId, AssetId)`.
- **Decision**: The user asked to find the data model for transactions and use it. The found model uses `AccountId`. I will propose using `AccountId` but we might need to alias it or ensure it fits. For now, I will strictly follow the struct structure found.

## Struct Mapping

| Accounting Repo | Crypto Tracker (Proposed) |
| :--- | :--- |
| `Transaction2` | `Transaction` |
| `TxEffect` | `TxEffect` |
| `outputs` | `outputs` |
| `datetime` | `datetime` |
| `account_id` | `account_id` |
| `amount_diff` | `amount` |

