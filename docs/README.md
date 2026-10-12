# Documentation

Documentation is organized by purpose. Specifications describe implemented contracts; proposed extensions must be explicitly labeled and linked to a todo. Findings record evidence, limitations, and design rationale. Todos own outstanding work and its priority.

## Specifications

- [Product questions, pain points, and proposed solutions](spec/product-goals.md)
- [First desktop app: delivered slice, remaining scope and UI interview](spec/desktop-app.md)
- [Web design-system package: Sage styles, Solid/Panda/Ark components and full catalog](spec/web-design-system.md)
- [Project context and conventions](spec/project.md)
- [Core data model](spec/data-model.md)
- [Shared Rust application services and exact DTOs](spec/application-services.md)
- [Proposed data model evolution and implementation phases](spec/data-model-proposal.md)
- [Persistence and ordered startup migrations](spec/persistence.md)
- [Persistent UI metadata and manual actions](spec/ui-metadata.md)
- [Portfolio read models and exact current earnings](spec/portfolio-queries.md)
- [Database startup and local settings](spec/database-startup.md)
- [CLI configuration](spec/cli.md)
- [Nexo sources, ledger mapping, balance sums and diagnostics](spec/nexo.md)
- [Binance sources, ledger mapping, reconciliation and diagnostics](spec/binance.md)
- [Kraken account-scoped ledger, exact fees, cached replay and reconciliation](spec/kraken.md)
- [EVM wallet setup, exact ledger mapping and pinned reconciliation](spec/evm-wallets.md)
- [Reviewed asset exclusions and preserved audit history](spec/tracking-exclusions.md)
- [Historical net worth and pricing](spec/historical-net-worth.md)
- [Binance account history and rate limiting](spec/binance-account-history.md)

## Findings

- [Desktop stage 6: real-data browser, typed IPC and validation](findings/desktop-stage-6.md)

- [User needs interview and confirmed portfolio questions](findings/user-needs.md)
- [Older product draft: comparison and follow-up decisions](findings/older-spec-comparison.md)
- [Taskstream design-system and homepage/docs structure reference](findings/taskstream-reference.md)
- [Risk measurement research and proposed opportunity comparisons](findings/risk-comparison.md)
- [Airtable data model comparison](findings/airtable-model.md)
- [Binance export and replay guide](findings/binance-history.md)
- [Kraken connection setup, OAuth evidence and history/export strategy](findings/kraken-connection.md)
- [Ethereum contract resolution, public indexer access and remaining blockers](findings/evm-contract-resolution.md)
- [EVM export inventory and live acquisition evidence](findings/evm-wallet-inventory.md)
- [Shared provider-guide and diagnostic conventions](findings/provider-diagnostics.md)
- [Consolidated implementation decisions and documentation corrections](findings/implementation-history.md)

## Todos

- [Provisional master todo list and delivery priorities](todo/master-todo.md)
- [First UI: ordered implementation plan and dependencies](todo/first-ui-implementation-plan.md)
- [Provider discovery inventory and source research](todo/provider-roadmap.md)
- [Implementation design notes from earlier proposals](todo/implementation-gaps.md)

The master todo list owns delivery status. Provider discovery tables and implementation notes are supporting references, not competing ranked checklists.

## Maintenance workflow

Read the relevant specs and source before planning. Put concrete requirements, acceptance scenarios, and necessary tradeoffs in a topic document under `spec/`; clearly distinguish proposed from implemented behavior. Put ordered, verifiable work under `todo/`, reusing existing tasks rather than creating duplicate checklists. Record supporting evidence and uncertainties under `findings/` without copying private exports or credentials.

Implement within the user's authorized scope. Update specs and todo status only after checking actual behavior and appropriate tests. Keep useful decisions in findings when completed plans are compacted; use Git history for obsolete drafts. Validate relative links and remove stale paths or commands when moving documentation. There is no separate documentation CLI or mandatory proposal/archive approval ceremony.
