# Money Tracker desktop

Stage 6: a local Tauri 2 host and Solid frontend consuming `@money-tracker/design-system` and the shared Rust application worker. Displays already-imported current holdings, allocations and evidenced current annual earnings. Missing inputs are labelled unknown/partial. No account connection, CSV-import, transaction-history, financial-edit or provider-refresh commands are exposed yet.

From the repository root:

```sh
pnpm install --frozen-lockfile
pnpm dev:desktop
# Production assets + native binary (platform prerequisites required):
pnpm build:desktop
# Faster native smoke build, still embedding production assets:
pnpm --filter @money-tracker/desktop tauri build --debug --no-bundle
```

The GUI uses `APP_DATA_DIR`, then local JSON `data_dir`, then the OS default. Unix default: `~/.data/money-tracker/portfolio.duckdb`. See [startup and settings](../../docs/spec/database-startup.md) for native config paths. A GUI launched from Finder may not inherit terminal environment variables; use the JSON config or launch from a configured terminal. No provider credentials are needed to browse imported data.

A missing database prompts for Copy existing (native picker; preserves source) or Start new (explicit empty DB). Close any other process writing the selected source/destination first. Corrupt/locked/unrelated databases return an explicit error; they never become empty portfolios automatically.

`pnpm --filter @money-tracker/desktop dev` alone is a frontend preview, without access to Rust/portfolio data. It explicitly reports the missing desktop runtime. It does not load synthetic showcase data.

Validation:

```sh
pnpm build
cargo test --locked -p money-tracker-desktop
UPDATE_DESKTOP_FIXTURE="$PWD/.cache/desktop-fixture.json" cargo test --locked -p money-tracker-desktop --test portfolio real_wire_fixture
pnpm build
PLAYWRIGHT_BROWSERS_PATH=.cache/playwright pnpm test:desktop
# Opt-in offline check, only on temporary copies of private cached DBs:
cargo test --locked -p money-tracker-desktop --test portfolio cached_imported_portfolios_are_browsable -- --ignored
```

`pnpm test:desktop` builds assets, then regenerates the Rust fixture before browser checks; it requires the native Tauri prerequisites. The fixture-generation helper works across Node-supported platforms. Browser tests mock only Tauri IPC and consume serialized Rust Session results. Native lifecycle/build checks are separate; a browser test does not prove native picker or OS shutdown behavior. Snapshot IDs reject stale view requests; every allocation/detail retains the same cutoff and exact Rust calculation as its navigation summary. Close and Quit await worker shutdown and exit, with no tray/background process.
