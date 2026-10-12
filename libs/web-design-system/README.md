# Money Tracker design system

SolidJS/Panda CSS/Ark UI package with the Sage visual refresh, consumed by the [synthetic showcase](../../demos/design-system-showcase). It accepts props/events and performs display formatting only. It has no DB, provider, credential, persistence, valuation or earnings engine. See the [implemented contract](../../docs/spec/web-design-system.md).

From the repository root with Node 24.21.0 and pnpm 11.27.0:

```sh
pnpm install --frozen-lockfile
pnpm typecheck
pnpm test
pnpm build
pnpm dev:showcase
```

`dev:showcase` first builds the library stylesheet, then starts Vite on localhost. The terminal prints its URL. The full catalog opens at `/`, and `/portfolio` contains the synthetic portfolio flow. A Solid/Tauri/site package adds `"@money-tracker/design-system": "workspace:*"` and imports only public entry points:

```tsx
import {
  ThemeRoot,
  PrivacyProvider,
  Button,
  Money,
} from "@money-tracker/design-system";
import "@money-tracker/design-system/fonts.css";
import "@money-tracker/design-system/styles.css";

export function Screen() {
  return (
    <ThemeRoot mode="system">
      <PrivacyProvider value={{ mode: "normal" }}>
        <Contents />
      </PrivacyProvider>
    </ThemeRoot>
  );
}
function Contents() {
  return (
    <>
      <Money
        value={{ numerator: "100050", denominator: "100" }}
        currency="USD"
      />
      <Button>Options</Button>
    </>
  );
}
```

Construct components that use privacy **under** the provider, including JSX stored in row arrays/memos; do not create financial cell elements before mounting the provider. `ThemeRoot` follows system preference changes unless given `light`/`dark`; hosts persist preferences. Details panels mount in this root, preserving theme variables without a body-level portal.

Public exports:

- `.`: theme/privacy providers; buttons, icon/pin controls, text/search fields, grouping tabs, inline editable fields; money/quantity/rate/figure/summary displays; navigation entries, allocation table, nonmodal details panel, badges/warnings and state messages. Public prop/domain presentation types accompany them.
- `./formatting`: pure exact formatting and privacy types, without Solid/Ark imports.
- `./styles.css`: generated complete reset/token/utility/recipe stylesheet. Consumers need no Panda config or scans. `--mt-*` variables and `mt-` classes are package-prefixed; use semantic variables for consumer layout where useful.

Component sources are exported as TypeScript/TSX for Solid-aware consumers (ES2022 BigInt). Generated `styled-system` runtime helpers and `dist/styles.css` are intentionally ignored; `pnpm codegen`/`pnpm build` regenerate them. Direct versions and the workspace lockfile are committed. This private workspace package is not published; release/licensing and a compiled distribution remain later decisions. Nothing was copied from the external Taskstream repository.

Install the browser once, then test the production consumer:

```sh
pnpm exec playwright install chromium --only-shell
pnpm test:browser
```

For a workspace-local browser cache, prefix **both** commands with `PLAYWRIGHT_BROWSERS_PATH=.cache/playwright`. Linux CI additionally installs browser system dependencies with `--with-deps`. Playwright manages and shuts down its localhost preview server. Screenshots are synthetic and written under ignored `.cache/design-system-review/`; traces/reports are ignored too.

Ark owns tab/edit/dialog behavior; the panel manages focus explicitly because it is nonmodal. Name/detail and expansion controls are independent. Inline editors accept exact text and host validation, retain invalid drafts, and emit only valid commits; hosts own applying changes/Undo. Callers supply already-filtered/ordered visible rows, rates and summaries. They never derive financial totals by adding table nodes or Dust entries. Hidden/scaled privacy changes presentation only; host settings, actual allocations, sort order and rates stay unchanged. Arbitrary child text or custom attributes must also be sanitized by the host.

The locally bundled Manrope font has a separate `fonts.css` export. `Surface`, `IconTile`, `ScreenHeader`, `ChipGroup`, `StatRow`, `ListRow`, `TimePill`, `TrendChart` and `PaginationDots` provide reference-style presentation primitives; chart inputs are display geometry rather than portfolio data.
