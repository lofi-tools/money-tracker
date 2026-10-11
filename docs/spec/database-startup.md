# Database startup and local settings

Implemented first-UI stage 2 on 11 October 2026. Sources: [configuration](../../libs/lib-app/src/config.rs), [database startup](../../libs/lib-app/src/startup.rs), [managed worker](../../libs/lib-app/src/worker.rs), and [startup tests](../../libs/lib-app/tests/startup.rs). These are shared Rust services; the Tauri chooser/settings screens and refresh scheduler remain later stages.

## Paths and configuration

The database filename remains `portfolio.duckdb`. Shared resolution uses explicit CLI `--data-dir` → `APP_DATA_DIR` → JSON `data_dir` → native default. Desktop hosts omit the CLI override. Cache directories and the Nexo CSV selection retain their existing independent CLI behavior.

| Platform | Default database | Default configuration |
| --- | --- | --- |
| Linux | `~/.data/money-tracker/portfolio.duckdb` | OS config directory (`$XDG_CONFIG_HOME` or `~/.config`)/`money-tracker/config.json` |
| macOS | `~/.data/money-tracker/portfolio.duckdb` | `~/Library/Application Support/money-tracker/config.json` |
| Windows | `%LOCALAPPDATA%/money-tracker/portfolio.duckdb` | `%APPDATA%/money-tracker/config.json` |

`Locations::system` obtains these OS directories through `dirs`; `Locations::for_platform` allows deterministic host-independent tests. An explicit `--config-path` selects the file and must exist. A missing default config uses defaults without creating a file. Malformed/unreadable configs, unknown fields, unsupported versions and invalid settings fail explicitly. Relative directory/config paths are relative to the process working directory; `~` is not expanded. Prefer absolute paths in config files.

Example version-1 config (all omitted settings default):

```json
{
  "version": 1,
  "data_dir": "/absolute/path/to/money-tracker",
  "active_portfolio": "local:default",
  "settings": {
    "reporting_currency": "USD",
    "theme": "system",
    "privacy_mode": "normal",
    "privacy_scale": "0.1",
    "dust_threshold": "1",
    "refresh_interval_seconds": 86400,
    "refresh_on_open": false
  }
}
```

Settings are local preferences in this file, separate from provider credentials, source transactions and portfolio metadata. Theme accepts `system`, `light`, `dark`; privacy accepts `normal`, `hidden`, `scaled`. Privacy scale is an exact string strictly between zero and one. Dust threshold is a nonnegative exact string denominated in the reporting currency (USD by default). Reporting currency must be nonempty; this preference does not establish available pricing for an arbitrary asset. Refresh interval is a positive number of seconds. No rate/price selection, display formatting or refresh execution is implemented by these settings alone.

`AppConfig::save` validates first, writes and syncs a temporary file in the same directory, then replaces the configuration file. The worker updates its in-memory settings only after this succeeds. Existing `data_dir` and `active_portfolio` configuration survive a settings save. Environment/CLI directory overrides are not silently persisted as config values. Configuration is loaded at startup; external edits take effect on the next open. DB-backed manual-action/Undo and refresh scheduling records remain stage 3/8 work.

## Explicit database opening

`DatabaseWorker::start(resolved, OpenAction)` accepts three actions:

- `OpenExisting`: open only the selected existing file. If missing, return `StartupOutcome::Missing` with the destination and any detected previous OS data-directory database. Do not create an empty replacement or open the previous location automatically.
- `CopyExisting(source)`: copy a selected Money Tracker DB into the resolved destination, preserving the source. Refuse an existing destination, including a destination equal to the source.
- `StartNew`: explicitly create an empty Money Tracker DB at the resolved destination; refuse overwriting an existing file.

The previous location is `dirs::data_dir()/money-tracker/portfolio.duckdb`, when different from the new destination. The future UI can show that path as a suggested copy source. The CLI now loads shared configuration and uses the new default; if its destination is missing and the previous DB exists, it stops with instructions to select the old `--data-dir` or copy the DB. It retains its prior explicit command/import/create behavior where no previous DB would be abandoned. CLI provider portfolio flags remain unchanged; `active_portfolio` scopes the managed desktop worker, not provider command flags.

Existing/copy files must be nonempty regular DuckDB files containing the Money Tracker ledger tables. Unrelated DuckDB files, corrupt files, directories and inaccessible files produce errors rather than empty portfolios. Ordered idempotent migrations still run through `Store::open` on every accepted opening. A settings change does not rewrite existing portfolio reporting metadata.

Copy/new installation uses a temporary file in the destination directory, validates migrations and portfolio selection there, closes the DB, syncs the file, and installs with `persist_noclobber`. Read-only backup sources are supported; only the temporary copy is made writable (private permissions on Unix). Failed validation leaves no destination DB; a concurrent destination creator is protected from overwrite. Copy acquires a DuckDB read-only source connection for the physical copy, and does not run migrations or checkpoint on the source. Sources with a `.wal` file or an active writer are refused: close the source writer cleanly before retrying. A crashed source requiring WAL recovery must first be recovered and closed through its normal database owner; copying only its main file would be incomplete. Opening contention also returns an error explaining that other CLI/app processes should be closed. There is no automatic retry loop or live database backup claim.

## One portfolio and one database owner

Without `active_portfolio`, the managed worker requires the sole portfolio to be `local:default`. Multiple portfolios or a different sole identity require an explicitly configured existing ID. Unknown IDs fail; copying/new creation validates the ID before installation. Portfolios are never merged. Worker ledger queries construct `ReadContext` using the selected ID; callers cannot accidentally supply a different portfolio. The later switcher remains deferred.

One dedicated `money-tracker-db` thread owns the `Application`/DuckDB connection from startup through close. Cloneable handles enqueue typed async reads and settings writes; migrations, file operations and queries run off the caller/UI thread, serialized in order. Provider fetches are never triggered by opening or reading settings, including when `refresh_on_open` is true; stage 8 will interpret that setting.

Dropping a queued request skips its effects. Already-running operations finish atomically; cancellation does not roll back a completed settings write or DB installation. `close().await` rejects new requests, drops queued jobs, waits for in-flight work and joins the owner thread. Dropping the last handle also signals shutdown; hosts must await explicit close for a confirmed completed shutdown. No provider task or persistent background process is created. The existing synchronous `Application::open/store` boundary remains available to CLI orchestration.

## Validation

Deterministic tests cover Unix/Windows path defaults and precedence, explicit/malformed config errors, exact preference validation, missing/legacy discovery, new/empty startup, copy preservation/no overwrite, corrupt/empty/unrelated sources, WAL and active-writer refusal, selected/unknown/multiple portfolios, concurrent requests, settings persistence, queued-request cancellation and close waiting for in-flight work. A synthetic pre-exact-schema Nexo/Binance database is copied and migrated repeatedly: IDs, signed liabilities, large integer balances and import hashes survive, with original source bytes unchanged.

Run `cargo test -p lib-app --test startup`, `cargo test -p lib-app worker::tests`, and `cargo test -p cli`. Native Windows paths are tested through injected locations; Windows runtime/file-lock behavior remains for the later cross-platform build lane. The full deterministic production-workspace suite also retains migration rollback/retry and provider mapping/replay checks. No live account fetch is needed for this stage.

Checked locally: 138 deterministic production-workspace tests passed, plus the opt-in offline Nexo/Binance upgrade test using temporary copies of all three existing private cached DBs. Formatting, locked offline Cargo metadata, CLI help and changed documentation links passed. The copy tests also passed with a read-only source and retained its original permissions. No live provider request was made.
