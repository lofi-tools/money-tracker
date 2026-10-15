# Nexo browser capture implementation

Implemented development slice approved 15 October 2026. The [approved design](nexo-browser-capture-proposal.md) remains the target. Brave is the only current browser target. Setup and precise remaining limits are in the [extension guide](../../apps/nexo-capture-extension/README.md).

## Delivered boundary

`apps/nexo-capture-extension` contains a Manifest V3 extension with a stable development identity, optional exact-origin site grant, an authenticated-page floating export control, read-only dashboard/breakdown/fixed-term/settings navigation, progress/Cancel, JSON collection and native messaging. Packaged scripts do not read credentials, intercept requests or submit financial/settings forms. Page text is untrusted and rendered as text.

`nexo-capture-host` (CLI package binary) validates the installed extension origin, pairing ID, payload/schema and size before atomically queuing JSON in an owner-only inbox beside the explicitly paired existing DB. Its `prepare` mode generates files for manual Brave registration, without silently installing browser access. The host uses bounded native-endian length-prefixed UTF-8 frames; stdout contains protocol messages only. No arbitrary SQL, paths or shell commands are accepted from the browser. The configured tracker executable is a trusted local setup choice, not page input.

The desktop polls the inbox while running and serializes ingestion through its existing `DatabaseWorker`. Only the tracker worker opens the DB. Durable committed/pending/rejected acknowledgements distinguish reception from ingestion. Hash and capture-ID checks preserve idempotency; replay after commit-before-ack does not create duplicate effects. Malformed queued requests receive a bounded rejection. The frontend adopts ingestion events and exposes captured term facts and unresolved reasons independently of portfolio ownership.

Migration `012_nexo_browser_captures` appends account/hash/capture-ID keyed immutable snapshots and receipts. Matched, explicitly labelled APY/APR can create dated subscription-specific `RateEvidence`. Existing validation and manual pin scopes are respected; pin conflicts retain captured evidence without applying its rate. No balances, positions, movements, scales, import markers or renewal settings are changed. Metadata revisions advance only for a new capture.

## Matching and dates

Shared Rust code owns one-to-one matching across all observations/candidate openings. Only a uniquely supported original principal, exact asset and source start instant (or a UTC-qualified source date) may bind an existing uniquely identified fixed subscription. Unknown-precision records compete conservatively but cannot auto-bind. Same-principal ambiguous CSV cohorts, duplicate observations and conflicting provider IDs remain unresolved. No FIFO, nearest date, APY or expected-yield tiebreaker is used. An external ID alone cannot identify an unbound CSV opening.

Dates retain source precision. A captured date-only maturity is displayed on a matched allocation without inventing a UTC instant or overwriting the subscription's manually recorded maturity. Actual existing subscription maturity retains display precedence. Captured payout and renewal facts remain separately scoped observations. Missing fields stay unknown; no missing-toggle-to-false default. Current rates start at observation time, not the original term opening. Historical observed APY estimates remain distinct from captured current rates and account-annotated expectations.

## Verified versus pending

Live read-only inspection in the user's Brave session established the current UK labels/routes and that the active list exposes initial balances and due dates but not start dates. No live account snapshot was ingested. The DOM reader and navigation coordinator were tested with synthetic fixtures; the Nix runtime/native files are installed, while the approved source wrapper is tested in an isolated Brave profile and live site access remains a setup step. A live installed-Brave trial is still required to verify its DOM selectors, host discovery and startup behavior.

Current collection is deliberately partial. Pagination/virtualized row traversal, a final list consistency pass, durable verified provider term IDs, manual binding/Undo UI, graphical pairing/installer, inbox retention cleanup and crash/duplicate-launch hardening remain outstanding in the existing capture todo. Partial captures cannot close missing terms. These capabilities must not be described as delivered from the approved design.

## Validation

Core/application/native desktop regression suites passed with private/live acquisition excluded. Eight capture/matcher/inbox tests and a real native-process delivery/replay test passed on synthetic databases. Browser tests cover read-only field extraction, repeated equal pots, unknown basis/start/precision, settings without writes, signed-out behavior, navigation delivery and the unresolved evidence panel. TypeScript/build and the existing desktop/review browser suites passed. The selected CLI portfolio was not opened/migrated or changed for this implementation's tests.

## Nix installation boundary

The canonical source and source-only package live in this repository at
`nix/nexo-capture-extension.nix`. my-nix forwards it through `pkgs.own`; nix-home
consumes it without a vendored copy. The hermetic build whitelists six public files
and validates identity, entry points and permissions. It reads no signing key,
pairing secret, local Rust build or network resource. Native registration still
names an explicit trusted local development executable, separately from this package.

The user explicitly approved `wrapBrave`: it wraps the actual macOS App entrypoint,
adds the immutable source path with `--load-extension`, and execs Brave preserving
caller arguments. App metadata is unchanged. nix-home uses `finalPackage` for the
Dock target and CLI environment. Updates apply on the next normal restart after
a switch; no manual unpacked load, reload or forced restart is required.
The rejected off-store CRX remains only a public regression fixture; existing
Web Store declarations are unchanged.

Hermetic argument checks and actual isolated Brave extension startup passed.
Home Manager evaluates to the tested wrapper. macOS App launch and Finder HTTP
URL delivery passed using a separate test App/profile. HTTPS reassignment returned
macOS `permErr`, and literal Dock clicking could not be verified through the UI
tool. The declared Dock target and unchanged URL/document metadata are checked;
those limitations are not represented as passing end-to-end tests. Original browser
associations are restored. No full switch, lockfile mutation or live ingestion
occurred. See the [extension guide](../../apps/nexo-capture-extension/README.md)
for setup, identity migration and acceptance details.
