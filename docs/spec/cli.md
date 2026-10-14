# CLI configuration

[Args and Config](../../apps/cli/src/cli.rs) use clap for flags, help, and version output. Without a subcommand the CLI replays available cached provider inputs into the selected shared DB and prints all provider/account balances and asset totals together. `providers` exposes the same operation with explicit source paths and `--no-import` for a stored-data-only report. Historical date flags retain the Nexo historical valuation path. `kraken --account ID` runs the account-scoped Kraken acquisition/replay and strict balance report in the same portfolio DB; global flags precede the subcommand. See [the Kraken guide](kraken.md) for cache/snapshot, portfolio selection and refresh options. `evm` registers/selects persisted public wallet accounts and runs discovery/replay with pinned quantity reconciliation; see [the EVM guide](evm-wallets.md) for chain defaults, endpoint overrides, offline replay and BNB key requirements. Binance commands and reconciliation are documented in [the repository README](../../README.md).

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

The shared resolver, managed desktop DB worker and Tauri copy/new startup UI are implemented. Provider connections and CSV-import controls in the UI remain deferred.

## Local development database override

The CLI and desktop use the same `portfolio.duckdb` filename and standard Unix default. From the repository root, select a disposable/local portfolio explicitly without changing the normal installed-app default:

```sh
APP_DATA_DIR="$PWD/.cache/data" cargo run -p cli -- --offline
APP_DATA_DIR="$PWD/.cache/data" pnpm dev:desktop
```

For a terminal session, `export APP_DATA_DIR="$PWD/.cache/data"` provides the same override. A local ignored `.env` may hold `APP_DATA_DIR="./.cache/data"` when loaded by the developer's environment/direnv setup; the CLI itself does not load dotenv files. Relative paths follow the launch working directory, so an absolute directory is safer if launching elsewhere. Unset `APP_DATA_DIR` to return to the configured/default directory. `--data-dir` remains the CLI's highest-priority override. No database is moved merely by changing the variable.

## Combined provider replay (14 October 2026)

The `h` development script forwards arguments to the CLI instead of printing an isolated test database. `h --offline` (or `cargo run -p cli -- --offline`) replays the configured Nexo CSV, a discovered Binance transaction CSV/ZIP with cached positions and Earn supplement, and the cached Kraken account snapshot into the selected `portfolio.duckdb`. Already saved EVM accounts are included in the report; this operation does not discover or refetch wallets. Existing separate provider print tests keep their isolated fixture databases.

Repository development defaults are `.cache/imports/Binance-Transaction-History*` (ZIP preferred, latest sorted path), `.cache/test-data/binance/positions-v1.json`, `.cache/test-data/binance/earn-supplement-v1.json`, and `.cache/test-data/kraken/<fingerprint>.json` for `KRAKEN_ACCOUNT_ID` or `kraken:primary`. These are optional local inputs, not databases to attach or synthetic holdings. Missing optional inputs are reported and existing stored holdings remain available. Explicit missing/corrupt inputs and reconciliation failures fail visibly. The Binance importer remains the single legacy account with distinct Spot/Funding/Earn buckets; the Kraken snapshot supplies its stable account identity. Existing account names/customizations survive replay.

```sh
APP_DATA_DIR="$PWD/.cache/data" cargo run -p cli -- --cache-dir .cache --offline
APP_DATA_DIR="$PWD/.cache/data" cargo run -p cli -- --offline providers --no-import
APP_DATA_DIR="$PWD/.cache/data" cargo run -p cli -- --offline providers \
  --binance-statement exports/Binance-Transaction-History.zip \
  --binance-positions snapshots/binance-positions.json \
  --binance-supplement snapshots/binance-earn.json \
  --kraken-snapshot snapshots/kraken.json
```

Cached replay never creates private API clients or refreshes snapshots. Without `--offline`, it additionally fills missing latest public quotes against the configured reporting currency using the existing reviewed CoinGecko mappings. Historical backfill remains an explicit historical-date operation; unsupported prices and missing APYs stay unknown. `--no-import` makes no provider/price requests. Global flags precede the subcommand.

The exact report lists owned positions per account and totals per provider/asset, keeping signed liabilities separately labelled and excluding counterparties/spam through the common account query. It describes transaction-derived amounts at the reporting cutoff; cached independent captures retain their separate timestamps and do not prove current balances today. Binance replay compares only its legacy account buckets, rescaling reference evidence to protected DB scales; cached reference quantities never become ledger opening adjustments. Kraken retains account-scoped strict reconciliation.

### Kraken precision prerequisite

The selected mixed-provider database originally stored DOT at eight decimals, while the Kraken snapshot includes ten-decimal movements. Before its first import, the reviewed workflow backs up the DB and runs `--offline providers --widen-scales`. This invokes the [explicit, transactional precision widening](data-model.md#explicit-precision-widening-14-october-2026), not rounding or relaxing ordinary imports. It is retryable; subsequent default runs need no flag. Scale-dependent immutable UI evidence blocks widening instead of being edited implicitly. Other providers’ exact decimal quantities, protected identities and account annotations are preserved.

SOL from the legacy Binance native asset namespace has an explicit CoinGecko Solana mapping. Arbitrary tokens claiming the ticker SOL are not matched. Unsupported legacy/delisted holdings remain unpriced in Others / dust and keep provider totals partial.
