# Web design system — proposed package plan

Status: **planning only; no TypeScript workspace, design-system package or app frontend is implemented**. Confirmed scope on 11 October 2026. Delivery tasks remain in [master todo section 0](../todo/master-todo.md#0-now-first-usable-desktop-app--ui) and [section 4](../todo/master-todo.md#4-near-term-typescript-design-system-package).

## Package boundary

Create a separate pnpm workspace package at `libs/web-design-system`, provisionally named `@money-tracker/design-system`. Build it with SolidJS components, Panda CSS tokens/styles and Ark UI interaction primitives. The app consumes this package through a `workspace:*` dependency; it does not duplicate its tokens, recipes or component implementations. Later the public homepage/docs and web app consume the same package.

Use a root pnpm workspace manifest, shared TypeScript defaults and a committed pnpm lockfile. Select and pin compatible tool versions when implementing; the current Rust workspace remains independent. Generated Panda utilities, dependencies and build output are ignored and reproducible from source. Remove the current lockfile ignore rule only as part of actual workspace creation.

Proposed layout:

```text
pnpm-workspace.yaml
package.json
pnpm-lock.yaml
tsconfig.base.json
libs/web-design-system/
  package.json
  panda.config.ts
  src/
    tokens/       # neutral palette, accent, semantic colors, type/spacing
    components/   # styled Solid components backed by Ark where applicable
    formatting/   # display helpers, not portfolio calculations
    index.ts      # documented public component exports
apps/desktop/     # eventual shared Solid frontend and Tauri host
```

The desktop frontend owns portfolio queries, navigation state, account grouping, settings persistence and Rust/Tauri calls. The design system accepts props/events; it does not open databases, fetch providers, store API keys, reconcile accounts or calculate financial yield. Keep exact values at the application boundary; formatting must not introduce floating-point conversion of ledger quantities.

## First app components

Start with the components needed by the [desktop contract](desktop-app.md):

- Buttons, icon buttons, inputs, labels and inline editing affordances, including pin state.
- Asset/provider grouping controls, searchable navigation, selected/hover/focus states.
- Expandable table rows with separate name-to-details and arrow-to-expand actions.
- Summary/value displays, unknown/partial states, liabilities and signed earnings.
- Warning/status indicators and a right-side details panel retaining the visible table.
- Theme controls, Undo affordances and empty/loading/offline/error states. The app owns change history and restoration; the design system exposes presentation/actions only.

Use Ark's headless primitives where they provide interaction/accessibility behavior, such as tabs, editable fields and panels. Wrap them in a coherent styled API rather than asking each app to restyle raw primitives. Native elements remain appropriate for simple semantic controls/tables. Component props must preserve accessibility attributes and keyboard interaction.

## Styling and distribution

Use neutral surfaces/text, one accent color and separate semantic status cues. Balance compact rows with comfortable padding. Support system/light/dark themes, accessible contrast/focus states and reduced-motion transitions. Theme persistence belongs to the app; components share the same theme contract.

The initial consumer can import Solid component sources and a generated stylesheet from documented package exports. Panda scanning/codegen belongs to the design-system build, with styles for all supported component variants. Do not require application imports of private generated files or duplicated Panda configuration. Use a package-specific class/token prefix. If applications later need their own Panda styles, add a documented preset/design-system integration after verifying compatible tooling; it is not a requirement for the first consumer.

Official references for implementation: [Panda stylesheet distribution](https://panda-css.com/docs/design-systems/consume-without-panda), [Panda presets](https://panda-css.com/docs/theming/presets), and [Ark UI components](https://ark-ui.com/docs/components/tabs). The [Taskstream structure](../findings/taskstream-reference.md) is a local reference, not a runtime sibling dependency.

## Display formatting

Currency values use full amounts with thousands separators, never automatic `k`/`m` abbreviations. Confirmed USD formatting uses two decimals for absolute amounts below $100 and whole dollars at $100 or more. Positive nonzero amounts below one cent display as “<$0.01”; retain a negative sign for tiny costs/losses and distinguish actual zero ($0.00). Examples: $12.34, $100 (from $100.25), $12,346 (from $12,345.67), and <$0.01 (from $0.004). Select precision before rounding; in scaled privacy mode use the scaled display amount. Other reporting currencies need currency-aware formatting rather than blindly copying USD symbols/minor units. Display rounding never changes stored amounts, financial calculations, reconciliation or dust classification. Nonzero tiny values must remain distinguishable from actual zero.

Token quantity displays trim trailing zeros and retain enough decimal precision for small nonzero balances to remain visible. Details provide full exact decimal quantities from integer units and protected asset scales; never pass them through floating-point conversion. Hidden/scaled privacy modes also apply to details, so exact original quantities appear only in normal mode.

Support unknown values, partial subtotals, signed costs and reporting-currency labels. Privacy mode is an app setting: components consistently display normal values, masked amounts/quantities, or proportionally scaled amounts. Scaled mode defaults to 10%, configurable by the user, and leaves percentage rates unchanged. Do not leak original amounts in hover/accessibility text while privacy mode is active.

## Implementation sequence and acceptance

1. Add the pnpm/TypeScript package boundary, lockfile, documented exports and reproducible Panda codegen/build commands.
2. Define theme tokens and implement the minimum controls/navigation/table/details primitives above.
3. Add an isolated showcase with explicitly synthetic examples covering themes, missing values, warnings, editing/pinning and privacy presentation.
4. Wire the shared Solid app frontend to the package, then connect its typed application services to the existing portfolio DB. A showcase or empty shell alone is not the first portfolio app deliverable.
5. Verify clean install, codegen, typecheck and production consumer build. Check actual interaction behavior: keyboard navigation, independent expansion/details controls, focus restoration, inline validation and theme switching. Inspect light/dark layouts and narrow-window behavior.

No implementation is requested by this plan. Public-site components, charts and broader component coverage follow the first app's needs through the existing todo list.
