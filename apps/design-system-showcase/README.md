# Synthetic component showcase

An isolated Solid/Vite consumer of `@money-tracker/design-system` via `workspace:*`, using its public TSX and stylesheet exports. It has no Panda configuration. Run `pnpm dev:showcase` from the root.

The default is Overview, system theme and normal privacy. By-asset/provider tabs, search, names/details and independent arrows demonstrate the package. One level below the top-level account is initially expanded. The ETH example uses the [stage-4 synthetic fixture](../../docs/spec/fixtures/current-portfolio.json): 50% at 5%, 25% at 6%, 25% at 0% → USD 1,000 and USD 40/year. Provider selections use fixed synthetic scope figures, not live calculations. Missing-data/liability/control examples are separately labelled and excluded from fixture totals.

The editable specimen changes a demo field/pin, with one-step in-memory Undo. It does not recalculate the fixture or implement persistent application actions. Warning details, subscription terms and theme/privacy examples are synthetic. Closing/reloading resets everything. Stage 6 will bind a real Solid/Tauri app to Rust services; this showcase does not connect accounts, open/copy databases or read local portfolio data.
