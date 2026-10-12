# Desktop stage 8: refresh lifecycle

Stage 8 connects the [persisted per-source refresh state](../spec/ui-metadata.md) to a real acquisition path. The app refreshes configured sources at a configurable interval while it is open (daily by default), performs at most one catch-up pass after missed intervals, coalesces overlapping triggers and reports per-source freshness and failures without ever storing raw provider text.

Contract: [desktop app spec — implemented refresh lifecycle](../spec/desktop-app.md#implemented-refresh-lifecycle-stage-8). Ordered plan: [first-UI plan, stage 8](../todo/first-ui-implementation-plan.md).

## What the app does now

- **Sources come from the open portfolio.** `prices` covers the distinct non-reporting assets actually held; `account:<id>` covers each saved Kraken account; `provider:NEXO` covers the configured Nexo export. Binance and EVM accounts are listed with an explicit prerequisite instead of a permanent failure, and providers with no documented acquisition path are not offered as sources at all.
- **Scheduling is a pure decision.** [`refresh::plan`](../../libs/lib-app/src/refresh.rs) turns stored state plus a trigger into run/start-schedule/skip actions for an injected `now`. A never-scheduled source starts its interval without contacting anything, so an ordinary open shows stored data, and the first refresh of a fresh install happens one interval later. Refresh-on-open runs every ready source; a launch after a missed interval performs exactly one catch-up run; scheduled ticks refresh only sources whose `next_due` has passed.
- **Overlapping triggers coalesce.** The host holds one in-flight guard, so launch, catch-up, scheduled and manual triggers that arrive together produce a single provider pass. A trigger during one returns no update and the UI says so.
- **Provider work is off the database thread.** The worker plans, the async side performs the network acquisition (CoinGecko exact quotes, Kraken cached snapshot and page cache), and one further worker request commits exact evidence plus the new schedule. The Nexo re-read is a local file parse plus idempotent import at commit time, which keeps it inside the same serialized request as its evidence rather than adding a second file pass; it never blocks the UI thread. Navigation and details stay responsive during a slow provider call.
- **Failures stay redacted and scheduled.** A failed run keeps the previous success, records a code plus static explanation (`acquisition_failed`, `import_failed`, `reconciliation_failed`) and moves `next_due` forward, so a broken source is not retried on every tick. Provider messages never reach storage, the UI or logs.
- **Settings shows the schedule.** Each source lists its last success, next refresh, stored failure and prerequisite; `Refresh now` performs a manual pass over every ready source. Prerequisites are re-evaluated per run, so credentials appearing later (for example after starting the app from a terminal with direnv) unblock a source without a migration.
- **Closing ends refresh work.** There is no tray or background process; `Quit`/window close sets the exiting flag, which stops the scheduler and refuses further refreshes before the database worker closes.

## Decisions and deviations

- **Refresh-on-open stays off by default, and it is not the only trigger.** The confirmed daily behaviour is implemented as the interval itself: a first open starts the schedule, later opens refresh only when an interval was missed. This keeps "show stored data on open" true without disabling automatic refresh entirely.
- **`prices` is not a separate setting any more.** The previous `refresh_prices` command and its Settings button were replaced by a manual refresh over every ready source. Ordinary opening no longer performs price acquisition on its own.
- **Binance and EVM are documented as unavailable rather than half-wired.** Their acquisition paths (statements/API import, per-chain endpoints and caches) need UI connection setup that this stage deliberately does not add. Listing them as blocked prerequisites keeps the coverage gap visible instead of implying it was refreshed.
- **A blocked source neither runs nor schedules.** Its state is left untouched, so an unavailable provider cannot consume the catch-up rule or fabricate a freshness timestamp.
- **Empty price results are `unchanged`, not `updated`.** A run that matches no reviewed price mapping is not a failure and not a change; nothing is fabricated for unmapped assets.
- **Failures schedule the next attempt but do not clear the last success.** The UI therefore distinguishes "last updated 2 days ago" from "last attempt failed 5 minutes ago".
- **Kraken acquisition had to become `Send`.** The signed request builder held `form_urlencoded::Serializer` across the HTTP await, which made the whole future non-`Send` and would have forced the desktop refresh onto a blocking thread. [`kraken-client`](../../libs/kraken-client/src/lib.rs) now scopes the encoder before the request. Behaviour, pagination, signing and caching are unchanged.
- **The fixture export path helps the cache, not the ledger.** `cache_dir` defaults to the same platform cache the CLI uses, so cached Kraken pages and pending cutoffs are reused rather than re-fetched.

## Validation

- `cargo test --locked --workspace --exclude try-polars --exclude money-tracker-desktop -- --skip adapters::binance::tests::cached_binance_account_reconciles`: all green, including six new `refresh` unit tests and the two new offline integration tests in [tests/refresh.rs](../../libs/lib-app/tests/refresh.rs).
- Scheduling evidence is clock-controlled: launch starts an unscheduled interval without running, a launch after a missed interval runs exactly once, refresh-on-open runs everything, scheduled ticks refresh only due sources, and a prerequisite blocks runs and schedules while staying visible.
- Offline integration evidence: a configured Nexo export starts unscheduled, a manual run imports it exactly once (same bytes → `Unchanged`), the schedule and freshness survive close/reopen, and one catch-up run after the interval imports changed bytes. The exported unit is USD-denominated, so even the price source stays offline (no market lookup).
- `cargo test -p money-tracker-desktop`: `refresh_starts_unscheduled_imports_exactly_and_survives_restart` covers the same lifecycle through the real `Session` wire path, including the sources a seeded portfolio exposes.
- `cargo fmt --all --check` passes. `cargo clippy` reports nothing new in the changed code; the remaining warnings are pre-existing in untouched files (`utils`, `binance-client`, `lib-core`, the `Startup` enum size and one older nested `if` in the host).
- `pnpm typecheck`, `pnpm format:check`, `pnpm test:desktop` (6 passed) — the browser suite drives `Refresh now`, asserts the per-source failure notice, the prerequisite text in the status table and that cached navigation figures are unchanged; its axe scan still reports no violations. `.cache/desktop-fixture.json` was regenerated from the Rust wire output and now carries a real `refresh` status.

## Limitations

- Kraken's live path is not exercised in CI; it needs credentials and network. The offline tests cover planning, commit, state and one local provider path.
- Credentials must be in the app's environment. There is no in-app provider connection or CSV-import UI yet (deferred from this milestone), so Binance/EVM sources stay blocked and Nexo needs a configured export path.
- A scheduled refresh on the database worker can still be slow if a provider returns a large Ledger/progress set; the tick itself is cheap and idle ticks do no work.
- Cross-platform scheduler behaviour (sleep/timer accuracy on Windows, macOS App Nap-like throttling) remains part of stage 9 acceptance.
