# Desktop packaging: macOS bundle evidence

Recorded 12 October 2026 as the packaging slice of [stage 9](../todo/first-ui-implementation-plan.md) (cross-platform acceptance and build delivery). The user selected this scope explicitly: **build and smoke-test the macOS bundles locally, keep signing and CI bundle lanes out of scope for now.** Nothing here is published or signed, and no lane uploads an artifact.

## What is produced

`pnpm bundle:desktop` (`pnpm --filter @money-tracker/desktop tauri build`) runs the production frontend build, the release Rust build and Tauri's own bundlers against [`tauri.conf.json`](../../apps/desktop/src-tauri/tauri.conf.json) (`bundle.targets = "all"`, identifier `dev.money-tracker.desktop`, version `0.1.0`):

| Artifact | Path | Observed size |
| --- | --- | --- |
| Application bundle | `target/release/bundle/macos/Money Tracker.app` | 57.06 MiB |
| Disk image | `target/release/bundle/dmg/Money Tracker_0.1.0_aarch64.dmg` | 16.74 MiB |

Inside `Money Tracker.app`: `Contents/MacOS/money-tracker-desktop` (the release host, 59,826,048 bytes), `Contents/Resources/Money Tracker.icns` and an `Info.plist` carrying `CFBundleIdentifier dev.money-tracker.desktop`, `CFBundleShortVersionString`/`CFBundleVersion 0.1.0`, `LSMinimumSystemVersion 10.13` and `NSHighResolutionCapable`. The DMG is created by Tauri's bundled `bundle_dmg.sh` and mounts read-only as `/Volumes/Money Tracker` containing the same `.app`.

These are build outputs under `target/`, which is ignored by git; no artifact or installer is committed.

## Acceptance

`pnpm test:desktop-bundle` runs the same native launch acceptance as `pnpm test:desktop-native` ([launch acceptance](ci-lanes.md#native-launch-acceptance)) against the bundle's executable instead of the bare release binary. On macOS (this checkout, arm64):

- The bundle's executable opens the real window, the bundled frontend reaches the host, a fresh temporary location shows the missing-database chooser, explicit creation opens contract 1 with no holdings, the same location reopens directly, and a corrupt database fails with exit 1 while staying byte-identical — identical results to the bare binary.
- The DMG mounts read-only, contains `Money Tracker.app` with the same executable, and detaches cleanly.

The script fails when the bundle is absent (`build the bundle first (macOS: pnpm bundle:desktop)`) and rejects unknown arguments, so a missing or misbuilt artifact cannot pass silently.

## Decisions and boundaries

- **macOS bundles are verified locally; there is no CI bundle lane and no artifact publication.** Bundled, signed artifacts for wider distribution stay behind the master todo's release tasks, together with release tagging, checksums and update delivery.
- **Unsigned and unnotarized.** No signing identity, notarization or Gatekeeper quarantine path is exercised, and the app is never downloaded from a remote host. Signing needs credentials that are not available here.
- **The smoke runs the bundle's executable directly, not `open`.** A GUI launched through LaunchServices does not inherit a terminal's environment, which is also the documented configuration caveat in the [desktop guide](../../apps/desktop/README.md); the temporary-location redirection therefore uses the executable path.
- **The icon is a placeholder.** `icons/icon.png` is a 32×32 PNG, so the generated `.icns` is placeholder quality; a real icon set is a design task, not a packaging decision, and `pnpm --filter @money-tracker/desktop tauri icon <source>` is the documented way to produce it.

## Not verified

- **Only macOS (arm64) is built here.** Windows (`.msi`/NSIS) and Linux (`.deb`/AppImage/RPM) bundles, their dependency lists and their architectures are untouched; `bundle:desktop` and `test:desktop-bundle` are macOS-oriented commands and CI deliberately does not run them.
- **No install-path smoke test.** The bundle is launched from the build directory; copying it to `/Applications`, first-launch quarantine prompts and removal are not exercised.
- **No universal or x86_64 macOS build**, no per-architecture artifact matrix and no reproducibility claim across machines.
