# Project Context

## Purpose
**Money Tracker**: A unified, open-source, local-first desktop and mobile application for tracking all investments (crypto, banks, stocks, pensions, debt) in one place.
**Goals**:
- **Unified Dashboard**: Track net worth, yield, and revenue across all accounts.
- **Local-First & Privacy**: Store data locally (DuckDB) to ensure privacy and longevity. Avoid reliance on cloud services where possible.
- **Automation**: Connect to accounts (via API keys or open banking) to automate data retrieval.
- **Yield Optimization**: Suggest better risk-adjusted yields and flag underperforming assets.
- **Tax Compliance**: Collect transaction proofs, generate accounts, and help file tax returns.

## Tech Stack
- **Language**: Rust (Core logic, Data fetching)
- **Desktop UI**: Tauri + TypeScript + Frontend Framework (e.g., React/Solid) + PandaCSS
- **Database**: DuckDB (Local-first data storage)
- **Build/Env**: Nix, Direnv
- **Async Runtime**: Tokio
- **HTTP Client**: Reqwest
- **Data Processing**: Polars, Serde, Serde JSON, CSV
- **Date/Time**: Chrono
- **Error Handling**: Anyhow, Thiserror, Miette
- **Crypto**: HMAC-SHA256

## Project Conventions

### Code Style
- Standard Rust formatting (rustfmt).
- Idiomatic Rust patterns.

### Architecture Patterns
- **Local-First**: Data is stored locally in DuckDB.
- **Core + UI**:
    - `rust-core`: Library for business logic, data fetching, and storage.
    - `ui`: Tauri-based frontend for interaction.
- **Offline Capable**: Cache data locally and fetch when online.
- **Extensible**: Designed to allow adding support for new providers easily.

### Testing Strategy
- **Unit Tests**: Run via `utest` script.
- **Integration Tests**: Run via `itest` script.

### Git Workflow
- Standard feature branching workflow.

## Domain Context
- **Transactions**: Immutable, append-only records of financial movements.
- **Assets**: Crypto, Fiat, Stocks, etc.
- **Yield/Revenue**: Earnings from staking, interest, dividends.
- **Net Worth**: Total value of all assets minus liabilities.
- **Privacy**: User data stays on their device; API keys are stored locally.

## Important Constraints
- **Privacy**: No user asset data should be forced to a cloud server.
- **Security**: API keys must be handled securely (local storage).
- **Accuracy**: Historical data and yield calculations must be precise.
- **Connectivity**: Must handle offline states and sync when online.

## External Dependencies
- **Crypto Exchanges**: Binance, Coinbase, Kraken, Nexo (CSV/API).
- **Open Banking APIs**: For bank account connections.
- **ZUGFeRD**: For invoicing support.
- **CoinGecko API**: For historical price data.
