# Documentation

Documentation is organized by purpose. Specifications describe implemented contracts; proposed extensions must be explicitly labeled and linked to a todo. Findings record evidence, limitations, and design rationale. Todos own outstanding work and its priority.

## Specifications

- [Project context and conventions](spec/project.md)
- [Core data model](spec/data-model.md)
- [Persistence and ordered startup migrations](spec/persistence.md)
- [CLI configuration](spec/cli.md)
- [Nexo CSV ingestion](spec/nexo.md)
- [Historical net worth and pricing](spec/historical-net-worth.md)
- [Binance account history and rate limiting](spec/binance-account-history.md)

## Findings

- [Airtable data model comparison](findings/airtable-model.md)
- [Binance export and replay guide](findings/binance-history.md)
- [Consolidated implementation decisions and documentation corrections](findings/implementation-history.md)

## Todos

- [Goals and provider roadmap, in descending priority](todo/provider-roadmap.md)
- [Remaining implementation details from earlier proposals](todo/implementation-gaps.md)

## Maintenance workflow

Read the relevant specs and source before planning. Put concrete requirements, acceptance scenarios, and necessary tradeoffs in a topic document under `spec/`; clearly distinguish proposed from implemented behavior. Put ordered, verifiable work under `todo/`, reusing existing tasks rather than creating duplicate checklists. Record supporting evidence and uncertainties under `findings/` without copying private exports or credentials.

Implement within the user's authorized scope. Update specs and todo status only after checking actual behavior and appropriate tests. Keep useful decisions in findings when completed plans are compacted; use Git history for obsolete drafts. Validate relative links and remove stale paths or commands when moving documentation. There is no separate documentation CLI or mandatory proposal/archive approval ceremony.
