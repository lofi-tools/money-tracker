# Project context

Money Tracker aims to provide a local-first view of investments, net worth, yield, revenue, and supporting financial evidence. The current implementation is a Rust CLI and libraries, with DuckDB storage and cached provider data. Desktop/mobile dashboards, broader bank/broker/pension coverage, yield comparisons, and jurisdiction-specific tax reports are future goals; Tauri, a TypeScript framework, and PandaCSS were earlier UI candidates, not an implemented frontend.

## Structure and conventions

- `libs/lib-core`: shared domain types, provider traits, balance history, DuckDB storage, and migrations.
- `apps/cli`: CLI configuration, provider adapters, imports, pricing orchestration, and reconciliation fixtures.
- `libs/binance-client`, `libs/coingecko-client`, `libs/nexo-csv`: source-specific acquisition and parsing; adapters own conversion into domain types.
- Rust, Tokio, Reqwest, Serde, Chrono, and DuckDB underpin the application. Nix and Direnv provide the development environment. Use rustfmt and scoped Cargo tests; consult the repository README for commands and cached/live integration workflows.

## Requirements and constraints

Store financial records locally and support offline replay. Preserve source evidence, avoid forced cloud storage of portfolio data, and keep credentials out of caches committed to Git, logs, and documentation. Network connectors and optional discovery services still reveal requested accounts/addresses to their providers.

Financial movements require stable identities, exact source amount handling, independent reconciliation, and explicit coverage limits. Reimports can replace effects under stable transaction IDs; the present ledger is not an immutable append-only audit log. Do not manufacture balancing entries from snapshots or hide missing prices. Net worth is assets minus liabilities, excluding external counterparties.

Binance and Nexo have account-history adapters. CoinGecko, Binance Vision, KuCoin candles, and ECB rates provide valuation inputs; pricing support does not imply account support. Coinbase, Kraken, other exchanges, Open Banking, and invoicing evidence (including the earlier ZUGFeRD idea) belong to future work in [the roadmap](../todo/provider-roadmap.md).
