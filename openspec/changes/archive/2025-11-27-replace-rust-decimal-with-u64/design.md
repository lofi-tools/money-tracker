# Design: Integer-based Financial Arithmetic

## Data Model

We will represent financial amounts as `u64` integers, representing the smallest unit of the asset (e.g., satoshis for Bitcoin, wei for Ethereum). The `Asset` definition will hold the `decimals` field, which indicates how many decimal places the asset uses.

### `Asset`

```rust
pub struct Asset {
    pub id: AssetId,
    pub chain_id: String,
    pub decimals: u8, // Added
    pub external_ids: HashMap<ProviderId, ExternalAssetId>,
}
```

### `Position`

```rust
pub struct Position {
    pub id: PositionId,
    pub product_id: ProductId,
    pub amount: u64, // Changed from f64
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
}
```

### `TxInputOutput`

```rust
pub struct TxInputOutput {
    pub asset: AssetId,
    pub amount: u64, // Changed from f64
}
```

## Conversion Logic

When ingesting data from external APIs (which often provide `f64` or string representations of decimals), we will convert to `u64` using the asset's decimal precision.

Formula: `internal_amount = (external_amount * 10^decimals).round() as u64`

Note: We need to handle potential overflow if `u64` is not large enough, but for most crypto assets (even with 18 decimals), `u64` is sufficient for reasonable amounts (up to ~18 quintillion units). For extremely large amounts or high precision, `u128` might be considered, but the requirement specifically asked for `u64`.

## Trade-offs

- **Pros**:
    - Simple, standard integer arithmetic.
    - No external dependencies for basic types.
    - Exact representation of values (no floating point errors).
- **Cons**:
    - Manual management of decimal places.
    - Potential for overflow if not careful (though `u64` is quite large).
    - Need to look up `Asset` to know the real value of an amount.
