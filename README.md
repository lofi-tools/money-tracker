# Money Tracker

Track crypto across accounts, chains, apps, platforms.
Estimate historical value and current revenue.


## Developer quickstart

Setup deps using `direnv allow` (needs Nix and nix-direnv), then:
- Run unit tests: `utest`
- Run program: `run`

## Nexo amounts and historical net worth

From the repository root, import `.cache/nexo_transactions.csv` and fetch prices into a local DuckDB file:

```sh
cargo run -p cli -- --data-dir ./data
```

On the first online run, the CLI backfills USD prices from one day before each asset's first owned transaction. It downloads older daily closes from Binance's public [spot archives](https://data.binance.vision/) and recent USD observations from CoinGecko. Later runs fetch only uncovered dates, including earlier dates introduced by another import. To extend the requested history further back and print historical net worth, pass a start date:

```sh
cargo run -p cli -- --data-dir ./data --historical-from 2020-07-26
```

The database is `./data/portfolio.duckdb`. The CLI hashes the exact Nexo CSV bytes and skips a file already imported successfully; a changed export updates transactions by stable ID. The hash is committed with the transaction import, so failed imports are retried. Use `--offline` to import and query without contacting price providers. Set `COINGECKO_DEMO_API_KEY` or `COINGECKO_PRO_API_KEY` for the corresponding API plan.

Pass `--nexo-csv PATH` to import an export from another location. The Nexo CSV path is independent of `--data-dir`, which controls the DuckDB location.

Price observations and fetched date ranges are stored in DuckDB per asset and quote currency. Binance archive candles and confirmed absent pairs are cached by traded pair and month; USD-valued results are cached separately. Archive USDT prices are treated as a USD proxy. GBP and EUR values use direct Binance quotes where available, then a BTC cross rate, then ECB reference rates. ETHW and other assets absent from Binance use public KuCoin daily candles when available. Historical POL falls back to MATIC archive pairs. NEXO before Binance listing uses the export's USD Equivalent divided by its NEXO amount and carries the last known rate across gaps. Those fallback rates are approximations, especially across months without NEXO rows. The CLI reports import and price-fetch progress as it runs.

Nexo's `USDX` and `xUSD` are valued at USD 1; `GBPX` and `EURX` follow GBP and EUR, with exchange rates derived from CoinGecko Bitcoin quotes. These are valuation assumptions, not exchange guarantees. The query rejects missing prices for nonzero holdings.
The Nexo CSV is imported as one aggregate position per asset because the export does not reliably identify individual wallets or products.
