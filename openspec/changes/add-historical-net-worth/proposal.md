# Change: Add historical net worth from Nexo and price archives

## Why
The CLI exits before importing data, and the existing value query does not convert ledger units to asset amounts or reject missing prices.

## What Changes
- Import Nexo CSV movements into DuckDB with stable transaction IDs and eight-decimal asset units.
- Fetch current and recent historical CoinGecko prices and older Binance Vision daily closes into DuckDB, with explicit asset mappings and cached pair months.
- Query asset amounts and USD net worth over time using the latest known price at each timestamp.
- Verify final asset amounts against the cached Nexo export.

## Impact
- Affected specs: historical-net-worth, persistence
- Affected code: `apps/cli`, `libs/binance-client`, `libs/coingecko-client`, `libs/lib-core`
