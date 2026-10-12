# Project context

Money Tracker aims to provide a local-first view of investments, net worth, yield, revenue, and supporting financial evidence. The current implementation is a Rust CLI and libraries, with DuckDB storage and cached provider data. [First-UI stage 1](application-services.md) adds shared application services and exact JSON DTOs; [stage 2](database-startup.md) adds shared configuration, safe database startup, local settings and a dedicated DB worker. [Stage 3](ui-metadata.md) adds persistent UI metadata, dated evidence, tracking policy and atomic manual-action/Undo primitives. [Stage 4](portfolio-queries.md) adds snapshot portfolio read models and exact current-value/annual-earnings queries with explicit incomplete coverage. [Stage 5](web-design-system.md) adds the separate SolidJS/Panda/Ark design system and an isolated synthetic showcase; the real portfolio/Tauri frontend remains unimplemented. Desktop/mobile dashboards, broader bank/broker/pension coverage, yield comparisons, and jurisdiction-specific tax reports are future goals.

## Current delivery priority

Build the first usable desktop app/UI next, ahead of further connector expansion. The [desktop scope interview](desktop-app.md) records confirmed scope and acceptance scenarios; the [ordered implementation plan](../todo/first-ui-implementation-plan.md) sequences the minimum model/query, design-system and desktop work; [master todo section 0](../todo/master-todo.md#0-now-first-usable-desktop-app--ui) owns immediate tasks. Existing coverage limitations remain visible. The component foundation and current query engine are implemented; a real desktop interface and historical earnings engine remain outstanding.

## Selected future interface direction

Confirmed in the [user needs interview](../findings/user-needs.md): the first desktop version will use Tauri, with a SolidJS, Panda CSS, and Ark UI frontend shared with the web version. A later desktop version will use Rust and GPUI. These are planned interfaces, not implemented behavior. Portfolio storage is local first, with a secondary cloud version for users who prefer hosted access. The public opportunity catalogue and paid risk-rating API are separate from personal portfolio storage.

The first desktop version will track holdings and suggest actions only. Executing transfers, staking, and renewal changes may be considered later; it is outside the first-version scope.

The product focuses on long-term investing. Initial portfolio coverage is crypto and related fiat balances. The user plans subsequent coverage of bank accounts, stocks, pensions, mortgages, and other assets/liabilities contributing to personal net worth, with bank and broker integrations through a paid cloud API plan. Those integrations and their data-handling boundaries are future work.

The app is intended to be open source from the start; license and service-component scope remain to be specified. Historical value/earnings graphs belong in the initial dashboard. Shared portfolios follow soon after single-user tracking. Tax reporting is later, through a cloud service that syncs local data; spending, community integrations, and mobile/sync are also later. Affiliate monetization follows live paid cloud/API plans. Invoicing is much later.

Local tracking and earnings analysis must remain usable without a subscription. Users can supply API keys for free provider/integration APIs. Paid APIs add risk ratings, provider dependency maps, and bank/broker integrations. The secondary cloud experience will offer one login, managed provider keys/sync, and OAuth connections where available. These are planned service boundaries; cloud pricing and credential-handling details remain to be designed.

## Structure and conventions

- `libs/lib-core`: shared domain types, provider traits, balance history, DuckDB storage, and migrations.
- `libs/lib-app`: shared application services, exact JSON DTOs, provider adapters and their mapping/reconciliation fixtures.
- `apps/cli`: CLI configuration and command/price orchestration consuming the shared library.
- `libs/binance-client`, `libs/kraken-client`, `libs/evm-client`, `libs/coingecko-client`, `libs/nexo-csv`: source-specific acquisition and parsing; adapters own conversion into domain types.
- Rust, Tokio, Reqwest, Serde, Chrono, and DuckDB underpin the application. Nix and Direnv provide the development environment. Use rustfmt and scoped Cargo tests; consult the repository README for commands and cached/live integration workflows.

## Requirements and constraints

Store financial records locally and support offline replay. Preserve source evidence, avoid forced cloud storage of portfolio data, and keep credentials out of caches committed to Git, logs, and documentation. Network connectors and optional discovery services still reveal requested accounts/addresses to their providers.

Financial movements require stable identities, exact source amount handling, independent reconciliation, and explicit coverage limits. Reimports can replace effects under stable transaction IDs; the present ledger is not an immutable append-only audit log. Do not manufacture balancing entries from snapshots or hide missing prices. Net worth is assets minus liabilities, excluding external counterparties.

Binance and Nexo have account-history adapters; Kraken now has an account-scoped Spot ledger/BalanceEx adapter and CLI replay/reconciliation command. EVM wallets have a native/ERC-20 quantity adapter with chain-qualified accounts, discovery, cached replay and pinned RPC reconciliation; see [coverage limits](evm-wallets.md). CoinGecko, Binance Vision, KuCoin candles, and ECB rates provide valuation inputs; pricing support does not imply account support. Broader Kraken account/product coverage, Coinbase, other exchanges, Open Banking, and invoicing evidence (including the earlier ZUGFeRD idea) belong to future work in [the master todo list](../todo/master-todo.md).
