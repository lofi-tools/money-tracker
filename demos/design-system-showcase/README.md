# Full design-system showcase

A standalone pnpm package under `demos/`, consuming only the public exports of `@money-tracker/design-system` via `workspace:*`. It has no Panda configuration, Rust dependency, accounts, credentials or provider data.

Run `pnpm dev:showcase` from the repository root. The catalog at `/` defaults to the reference’s dark theme, with light/system overrides. `/portfolio` preserves the synthetic portfolio interaction walkthrough.

The catalog covers palette/type/radius/elevation tokens; every public component; primary/secondary/ghost/danger/disabled controls; icon buttons/tiles; inputs/search/editors/pins; segmented controls/chips; badges/warnings; exact quantities/money/rates; known/partial/unknown/not-applicable figures; summaries; navigation; expandable allocations/details/focus; empty/loading/offline/error states; card/pagination, SVG charts and composed dashboard specimens. Financial samples are explicitly synthetic. Theme/privacy and specimen actions work in memory and reset on reload.

The shared styling follows the user-supplied Sage Finance HTML reference: warm charcoal layers, pale sage accents, soft glows, rounded panels, and locally bundled Manrope. There are no CDN fonts or remote images. Chart paths are synthetic display geometry, not calculated historical returns; number animation never introduces fabricated intermediate money values. Bank-card/history specimens do not imply that those portfolio features exist.

The `/portfolio` ETH example uses the [stage-4 synthetic fixture](../../docs/spec/fixtures/current-portfolio.json): 50% at 5%, 25% at 6%, 25% at 0% → USD 1,000 and USD 40/year. Provider scopes are fixed examples. Missing-data/liability/control examples are excluded from fixture totals. The editable specimen changes a demo field/pin with one-step in-memory Undo; it does not recalculate fixture earnings or implement persistent actions. The real Tauri app separately consumes the same library and Rust services.

Checks: `pnpm typecheck`, `pnpm test`, `pnpm build`, and `PLAYWRIGHT_BROWSERS_PATH=.cache/playwright pnpm test:browser` after installing Chromium. Browser coverage includes all catalog themes, interactions, privacy, contrast, reduced motion, 390px overflow and the portfolio walkthrough. Review screenshots stay in ignored `.cache/design-system-review/`.
