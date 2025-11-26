# Proposal: Persist Accounting Models

## Summary
Implement DuckDB persistence for `Account`, `Transaction`, and `TransactionEffect` types in `lib-core`.

## Motivation
To support the local-first architecture, we need to store financial data reliably on the user's device. DuckDB is the chosen database for this purpose.

## Scope
- Add `duckdb` dependency to `lib-core`.
- Define SQL schemas for accounts and transactions.
- Implement a `Store` module to handle database operations.
