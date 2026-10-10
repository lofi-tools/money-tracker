# CLI configuration

[Args and Config](../../apps/cli/src/cli.rs) use clap for flags, help, and version output. The current CLI runs the Nexo import and valuation path; Binance commands and reconciliation are documented in [the repository README](../../README.md).

| Option | Behavior |
| --- | --- |
| `--cache-dir` | Override cache directory; environment fallback `APP_CACHE_DIR` |
| `--data-dir` / `-d` | Override data directory; environment fallback `APP_DATA_DIR` |
| `--nexo-csv` | Select Nexo export; default `.cache/imports/nexo_transactions_05-10-2026_10-36-53.csv` |
| `--historical-from` | Extend price history from a UTC `YYYY-MM-DD` date and print net worth |
| `--historical-to` | Historical end date; defaults to now |
| `--offline` | Import CSV and query cached prices without contacting price providers |
| `--config-path` / `-c` | Parsed option; configuration-file loading is not wired up |

Directory precedence is explicit CLI flag, then the corresponding environment variable, then the OS directory from `dirs` joined with `money-tracker`. Cache and data have separate defaults. Fallbacks use home `.cache/money-tracker` and `.data/money-tracker`, then relative directories. DuckDB is `portfolio.duckdb` under the data directory. Changing the data directory does not change the Nexo export path.

Acceptance scenarios: CLI paths override environment paths; environment paths override defaults; help/version work; `--data-dir` alone leaves the Nexo export default unchanged. Config-file loading and managed export selection remain [backlog items](../todo/implementation-gaps.md).
