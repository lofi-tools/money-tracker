# Nexo capture for Brave

Development implementation approved 15 October 2026. **Live installed-extension capture remains unverified.** Firefox is a roadmap task, not a build target.

The floating **Export Nexo data** button reads the signed-in UK web interface, navigates the dashboard's savings breakdown and fixed-term views for each displayed earning asset, reads savings settings and sends JSON to Money Tracker through native messaging. No settings or financial controls are changed. Captured observations and unresolved matching reasons appear in the tracker under **Nexo browser captures**. Capture itself never creates ownership or reward movements.

## Nix package and Brave installation

The canonical runtime and package live in this repository:
`nix/nexo-capture-extension.nix`. `nix build .#nexo-capture-extension` builds
from exactly six public runtime files, without network access, signing keys,
local Cargo output or pairing secrets. The build checks the stable extension ID,
entry points, permissions and file inventory. Source changes require no CRX signing.

`my-nix` imports this derivation through a non-flake `money-tracker` source input
and forwards `pkgs.own.nexo-capture-extension`; nix-home only consumes it.
The user approved local source loading through `wrapBrave`. On macOS it wraps
the actual `.app/Contents/MacOS/Brave Browser` entry point; Linux wraps `bin/brave`.
It adds `--load-extension=<immutable package runtime>` and execs Brave with all
caller arguments intact. App metadata and URL/document registrations are unchanged.
Home Manager's `finalPackage` is also used for the Dock target and CLI environment.
A switch selects the new package; the next normal restart loads its source.
No Load unpacked, manual Reload or forced browser restart is needed.

Existing Web Store extensions remain on their normal installation path. The
archived local-CRX experiment was rejected by Brave 1.96.61 with
`DISABLE_NOT_VERIFIED` (256); its public artifact is retained only as a test fixture.
No CRX force-install policy or verification-disabling flag is used for deployment.

During development, use `--override-input my-nix git+file:///Users/me/src/scripts/my-nix`
and, in nix-home, `--override-input my-nix/money-tracker git+file:///Users/me/src/me/crypto-tracker`
with `--no-write-lock-file`. nix-home's `.envrc` supplies the source override when
that local repository exists. Git-backed inputs include staged new files and
exclude private ignored caches. Pin the published source revision when this
implementation has been reviewed, committed and pushed; the remote revision must contain the package files before the local source
override can be removed. No consumer lockfile is rewritten
by this development validation.

The static extension build is hermetic. The separate native-host registration
currently names the explicitly configured trusted local Rust development build;
it is not a claim that the native host/desktop are hermetically packaged yet.

### Stable identity

The manifest public key fixes the extension ID at
`agmnljgahcijdoahbejifmdcdpamijin`. Native registration and private pairing must
allow exactly this ID. The former `dhgo` development identity is superseded;
existing pairing must be reviewed/migrated before a real export. The archived
publisher private key is not used by source loading and stays outside Git/Nix.

## Build and pair locally

```sh
cargo build --locked -p cli --bin nexo-capture-host
cargo build --locked -p money-tracker-desktop
```

Use the manifest’s stable ID `agmnljgahcijdoahbejifmdcdpamijin` for the host allowlist.

Prepare a connection with an **existing** `portfolio.duckdb`, portfolio and Nexo account. For this repository's development portfolio, the command would be:

```sh
target/debug/nexo-capture-host prepare \
  .cache/nexo-native-host agmnljgahcijdoahbejifmdcdpamijin \
  .cache/data/portfolio.duckdb local:default legacy:nexo \
  target/debug/money-tracker-desktop
```

This command writes an owner-only pairing file and inbox beside the specified database and prepares a wrapper/host manifest in the chosen output directory. It does **not** register the host in Brave or load the extension. Existing pairing files are not overwritten. Save the printed connection ID for extension setup. Do not prepare a connection to a DB you did not intend to use. The bridge cannot create a new database; it does not open DuckDB.

On macOS, the usual Brave native-host directory is `~/Library/Application Support/BraveSoftware/Brave-Browser/NativeMessagingHosts/`; on Linux it is `~/.config/BraveSoftware/Brave-Browser/NativeMessagingHosts/`. Browser profiles/custom installations may use a different directory. Register the generated `dev.money_tracker.nexo_capture.json` in the correct Brave directory after reviewing its absolute executable path and exact extension allowlist. Windows registration is not implemented.

Install the package files through nix-home as described above. The approved wrapper loads the immutable source on Brave’s next normal restart. Open its toolbar setup, enter the connection ID and grant access to **platform.nexo.com only**. Reload Nexo. Manual unpacked loading remains an optional developer diagnostic, not the Nix deployment path. The extension uses `scripting`, `storage`, `nativeMessaging` and the granted site origin; it has no cookies, request interception, debugger or broad host permission.

Keep the paired tracker open for initial testing. The bridge can start the configured tracker binary with the explicit `APP_DATA_DIR` pointing to the paired database directory if its worker heartbeat is missing/stale. The tracker still uses its existing portfolio startup selection; custom portfolios must agree with that configuration. Multiple concurrent launches and stale heartbeat recovery need a live installation check. A missing/busy/wrong tracker returns a pending/rejected result rather than claiming success or opening another DB.

Press **Export Nexo data**. Progress and Cancel appear on Nexo. User clicks/input outside the overlay stop collection. Once JSON is sending, cancellation is disabled because local ingestion may already be atomic/in progress. On completion the extension returns to the initial Nexo view when the user did not intervene. The final status survives the page navigation. Failed/pending ingestion can be retried from extension setup using the original capture, without duplicate DB effects.

## Important current limits

- Inspected live Brave accessibility evidence confirms `platform.nexo.com`, dashboard savings breakdowns, `/fixed-terms/<asset>`, initial balances, due dates, calculation/payout labels and renewal controls, plus `/profile/settings`. DOM ancestry/selectors are implemented from those semantics and tested against synthetic pages, **not an installed live extension trial**.
- The inspected fixed-term list did not expose a start date or durable provider term ID. Displayed precision is not declared full ledger precision. The reader therefore exports unknown exactness/start/ID and **does not force transaction links**. This means current captures may all be unresolved while still recording useful actual due dates/rates/settings.
- Rates without an explicit APY/APR label retain `basis: unknown`, including rates labelled only with simple-interest calculation. Do not relabel them APY. Multiple rate bands remain unresolved rather than applying a maximum. Flexible/fixed breakdown percentages are separate raw account evidence, not broadcast rate defaults.
- The first collector deliberately marks every run **partial**: it reads currently rendered active cards and detects count/pagination mismatches, but does not yet traverse pagination/virtualized offscreen rows or perform an end-of-run list consistency pass. It never treats absent rows as closed. This remaining work is in the master todo.
- Private pairing migration/site grant and the end-to-end live Brave trial remain review steps. UI pairing/installer packaging is not implemented; the explicit local prepare command is the development setup.
- The extension keeps a failed payload in session memory. Retry enforces a 30-minute expiry; expiry is checked on use, and browser/extension restart clears session storage. There is no sync or external upload. Completed source evidence remains in the tracker DB. Result acknowledgements stay in the owner-only inbox for reliable retries; automatic retention cleanup is future work.
- Date-only maturities stay date-only; timezone is not guessed. Existing CSV balances, manual pins, source hashes and ownership remain unchanged. Captured raw rates and historical observed estimates are separate evidence.

## Checks

```sh
python3 tests/nix/extension-source.py
node --test apps/nexo-capture-extension/tests/background.test.mjs
CI=true PLAYWRIGHT_BROWSERS_PATH=.cache/playwright pnpm --config.verifyDepsBeforeRun=false exec playwright test --config playwright.nexo-capture.config.ts
cargo test --locked -p lib-app --test nexo_capture
cargo test --locked -p cli --test nexo_capture_host
```

The native process test sends framed JSON through a real helper process to an existing DB worker in a temporary synthetic portfolio, waits for committed acknowledgement and checks replay. Other cases cover exact one-to-one matching, equal-principal CSV cohorts, conflicting external IDs, missing/rounded data, pins, schema/account rejection, frame bounds, symlink/permission rejection and preserved balances. Browser fixtures are synthetic and contain no account data. The UI capture-view test runs in the review suite.

### Wrapper acceptance (15 October 2026)

The hermetic argument fixture checks empty/space/newline arguments, URL parameters,
macOS process flags, environment, working directory, exec PID and exit status.
Actual wrapped Brave loaded the extension in an isolated profile; the test harness
adds no extension-loading flag. The Home Manager final package evaluates to the
same derivation. Production Info.plist is byte-identical to base Brave.
A separate test App/profile verifies macOS App startup and Finder HTTP default-browser
URL delivery. HTTPS handler reassignment failed with macOS `permErr` and is not
claimed as a passing launch test. Actual Dock clicking is unverified: the UI tool
could not attach to Dock and this machine disables its keyboard focus shortcut.
The declared Dock path uses `finalPackage`; App launch itself passed.
Both original HTTP/HTTPS associations are restored after testing. No system switch
or live Nexo export was performed. The wrapper changes the entry executable to a
shell script; vendor bundle-signature preservation is not claimed (the base Nix
bundle also fails strict resource-signature verification).
