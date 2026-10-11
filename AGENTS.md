# Repository instructions

For planning, new capabilities, architecture changes, or ambiguous behavior, read [the documentation index](docs/README.md), [project context](docs/spec/project.md), and relevant topic specs before editing.

- `docs/spec/`: implemented contracts, acceptance scenarios, and explicitly labeled proposed designs.
- `docs/findings/`: evidence, limitations, operational guides, and design decisions.
- `docs/todo/`: outstanding goals and work in priority order; reuse existing tasks and avoid duplication.

Ground documentation in current source. Do not present proposed capabilities as implemented. Preserve useful rationale when compacting completed work, and update links and task states with relevant changes. Follow the user's authorization; documentation maintenance does not impose an additional approval gate. See the maintenance workflow in `docs/README.md`.

## Commit messages

Commit after each completed implementation stage, once its relevant checks pass. Include the stage's documentation updates in the commit.

Follow the recent repository history: `<short affected area>: <extremely terse description>`.

- Use the affected component, provider, or topic as the prefix, such as `docs`, `workspace`, `cli`, `migrations`, `data model`, `nexo`, `kraken`, or `evm-chains`.
- Keep the description short and specific. Use lowercase by default and omit the final period.
- Prefer the affected area over generic prefixes like `feat` or `fix`. Put essential rationale in the body when needed, rather than lengthening the subject.

Examples from the history: `docs: implementation plan`, `kraken: balance reconciliation`, `migrations: run at start + idempotent`, and `evm-chains: exclude spam tokens`.
