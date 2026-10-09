# Complete Binance account history

## Why
The Binance provider has unfinished transaction mapping and cannot reconcile its ledger against live account positions.

## What Changes
- Fetch paginated, time-windowed account history and independent current Spot, Funding and Simple Earn balances.
- Map movements, exchanges and fees to owned and external positions using exact decimal amounts.
- Cache raw API responses under the ignored repository `.cache` directory for repeatable reconciliation; provide an ignored uncached live test.
- Surface discrepancies and API retention limits rather than inventing opening balances.

## Impact
- Affected specs: binance-account-history
- Affected code: libs/binance-client, apps/cli/src/adapters/binance.rs

The user's request to finish this implementation authorizes this scope.
