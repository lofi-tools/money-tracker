# CI and native build lanes

Configured 12 October 2026 as the first part of [stage 9](../todo/first-ui-implementation-plan.md) (cross-platform acceptance and build delivery) and the master todo's [CI section](../todo/master-todo.md#6-near-term-ci-and-reproducible-builds-on-three-systems). Source of truth is the workflows themselves: [`.github/workflows/rust.yml`](../../.github/workflows/rust.yml), [`desktop.yml`](../../.github/workflows/desktop.yml) and [`design-system.yml`](../../.github/workflows/design-system.yml).

## What each lane runs

| Workflow | Job / systems | Commands |
| --- | --- | --- |
| Rust foundation | `rust`, matrix `ubuntu-latest`, `macos-latest`, `windows-latest` | `cargo metadata --locked --no-deps`, `cargo fmt --all --check`, deterministic workspace tests (`--exclude try-polars --exclude money-tracker-desktop`, skipping the legacy cache-first Binance test), `cargo run -p cli -- --help` |
| Desktop app | `service-tests`, same three systems | `cargo test --locked -p money-tracker-desktop`: real `Session`/DTO tests over temporary databases — read-only navigation, exact/partial figures, manual edits with Undo, dust/spam/grouping and the refresh lifecycle — plus the headless startup smoke described below |
| Desktop app | `frontend`, Ubuntu | frozen install, `pnpm test:desktop` — builds the design system and desktop frontend, regenerates the fixture from real Rust Session output, runs the Chromium suite including the axe scan |
| Desktop app | `native-build`, same three systems | frozen install, `pnpm build:desktop` — a debug-free (`--no-bundle`) Tauri build that compiles the host and embeds production frontend assets |
| Design system | `design-system`, Ubuntu | frozen install, formatting, codegen/typecheck, unit tests, production build, design-system browser suite |

Linux jobs install the Tauri prerequisites explicitly (`libwebkit2gtk-4.1-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`, `libxdo-dev`, `libssl-dev`, build tools). macOS and Windows runners need no extra system packages: WebView2 and the platform toolchain are present.

## Headless startup smoke

[`examples/startup_smoke.rs`](../../apps/desktop/src-tauri/examples/startup_smoke.rs) runs the real host startup path on one system and exits: platform path resolution, the missing-database chooser state, explicit creation at the resolved destination, an empty snapshot, an idle refresh plan and a clean close. It redirects the database, config and cache paths to a temporary directory, clears any configured portfolio or Nexo export, and only reads the platform config for path precedence parity — so it cannot touch a developer's or runner's real portfolio.

`cargo run --locked -p money-tracker-desktop --example startup_smoke -- <directory>` fails with a non-zero status and a plain message if any step regresses. It does not open the window: runners have no display.

## Decisions

- **The desktop crate has its own lane rather than joining the foundation matrix.** Its tests need Tauri's native dependencies on Linux, so they run in a lane that installs them instead of slowing the dependency-light foundation checks on all three systems.
- **Deterministic data only.** No lane takes provider credentials, private caches or imported financial databases. The frontend lane regenerates its fixture from Rust Session output over a synthetic seed; the desktop tests build their own temporary databases. Private/live provider tests stay opt-in and local.
- **`--no-bundle` for the build smoke.** Compiling the host and embedding production assets is the acceptance criterion here; packaging formats, signing and notarization remain the master todo's release tasks and are not implied by a passing build lane.
- **Two jobs share the OS matrix.** Service tests and native builds fail independently per system, so a Windows-only regression does not hide a passing macOS build.

## Verified locally

macOS (this checkout, 12 October 2026):

- `cargo metadata --locked --no-deps`, `cargo fmt --all --check`: pass.
- `cargo test --locked --workspace --exclude try-polars --exclude money-tracker-desktop -- --skip adapters::binance::tests::cached_binance_account_reconciles`: all suites pass.
- `cargo test --locked -p money-tracker-desktop`: pass, including the refresh-lifecycle and coalescing tests.
- `pnpm test:desktop`: 6 browser tests pass, including the axe scan.
- `pnpm build:desktop` (the exact `native-build` command): passes, producing `target/release/money-tracker-desktop` (59 MB) with embedded production assets; the release profile finished in about 2m 20s on a 14-core macOS machine. The workflow YAML of all three files parses.
- `cargo run --locked -p money-tracker-desktop --example startup_smoke -- /tmp/mt-smoke-check`: passes (`missing → new → empty snapshot → close`, contract 1, reporting USD), so the smoke's assertions hold on at least one system.

## Not verified

- **Hosted execution.** None of these lanes has run on GitHub-hosted runners; the Linux and Windows dependency lists, timings and cache behavior are unproven there.
- **The GUI window itself.** The startup smoke covers the host's startup path headlessly, but nothing opens the window or drives the native file picker; that still needs a desktop session on each system, so it remains a manual or self-hosted step.
- **Packaging, signing and notarization,** installer smoke tests and supported-architecture choices remain the master todo's release tasks.
- **Windows/macOS provider paths.** The two new systems only run the deterministic offline tests; Binance/Kraken/EVM acquisition stays opt-in and unverified across platforms.
