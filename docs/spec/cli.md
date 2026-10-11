# CLI configuration

[Args and Config](../../apps/cli/src/cli.rs) use clap for flags, help, and version output. Without a subcommand the CLI runs the Nexo import and valuation path. `kraken --account ID` runs the account-scoped Kraken acquisition/replay and strict balance report in the same portfolio DB; global flags precede the subcommand. See [the Kraken guide](kraken.md) for cache/snapshot, portfolio selection and refresh options. `evm` registers/selects persisted public wallet accounts and runs discovery/replay with pinned quantity reconciliation; see [the EVM guide](evm-wallets.md) for chain defaults, endpoint overrides, offline replay and BNB key requirements. Binance commands and reconciliation are documented in [the repository README](../../README.md).

| Option | Behavior |
| --- | --- |
| `--cache-dir` | Override cache directory; environment fallback `APP_CACHE_DIR` |
| `--data-dir` / `-d` | Override data directory; environment fallback `APP_DATA_DIR` |
| `--nexo-csv` | Select Nexo export; default `.cache/imports/nexo_transactions_05-10-2026_10-36-53.csv` |
| `--historical-from` | Extend price history from a UTC `YYYY-MM-DD` date and print net worth |
| `--historical-to` | Historical end date; defaults to now |
| `--offline` | Import CSV and query cached prices without contacting price providers |
| `--config-path` / `-c` | Select a required JSON config file; otherwise load the native default config when present |

Directory precedence is explicit CLI flag → environment variable → config `data_dir` → native default. Unix data defaults to `~/.data/money-tracker`; Windows defaults to `%LOCALAPPDATA%/money-tracker`. DuckDB is `portfolio.duckdb` under that directory. [Shared startup/configuration](database-startup.md) specifies JSON format/location, settings, validation and previous-location detection. Cache resolution still uses `APP_CACHE_DIR`/OS cache defaults, independent of config `data_dir`; changing the data directory does not change the Nexo export path.

When the selected database is missing and the previous OS data-directory DB exists, the CLI stops with guidance to select it using `--data-dir` or copy it explicitly. It does not silently start an empty replacement. Without that conflict, existing CLI import/create behavior remains. Malformed/explicitly missing config files fail before provider work.

Acceptance scenarios: CLI paths override environment paths; environment paths override config/defaults; config loading succeeds; help/version work; `--data-dir` alone leaves the Nexo export default unchanged. Database onboarding controls and managed export selection in the UI remain later stages.

Saved EVM accounts support `--exclude-safe-promo` (Ethereum only) and `--restore-token CONTRACT`; both persist reviewed policy revisions. Combine with `--register-only` for a settings-only update, or `--offline` for cached strict replay. See [reviewed exclusions](tracking-exclusions.md).

The shared resolver is implemented. The managed desktop DB worker and copy/new actions are available in Rust; their Tauri UI remains deferred.
