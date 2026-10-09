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

## Binance account reconciliation

To load the cached account and display exact asset balances with progress logs, run `binance` in the Nix dev shell (or `nix run .#binance` from the repository). This runs the ignored `latest_amount_after_cached_account` test with output visible, like `nexo`. It uses the same raw snapshot and persistent DB as reconciliation, prints transaction-derived totals without asserting reconciliation, and labels possible history gaps.

The Binance adapter maps API-accessible Spot, Funding and Simple Earn activity to separate owned accounts per asset, with external deposit, withdrawal, market, reward and fee counterparties. It fetches deposits, withdrawals, spot trades (including fees in another asset), conversions, dust conversions, dividends, fiat orders/payments, Funding transfers and Earn subscriptions/redemptions/rewards. Decimal strings are converted exactly; per-asset precision is at least eight decimal places and increases when the snapshot requires it. Locked Earn positions are aggregated by asset.

Set `BINANCE_API_KEY_ID` and `BINANCE_API_KEY_SECRET` in your direnv environment, then run (the earlier `BINANCE_API_KEY` and `BINANCE_SECRET_KEY` names remain supported as fallbacks):

```sh
cargo test -p cli cached_binance_account_reconciles
```

The first run backfills history from Binance's launch and independently fetches current balances. This can take a long time: trade history requires a request for every known pair, and other endpoints impose rate limits. A complete raw snapshot is saved to `.cache/binance/account-v1.json` before reconciliation, so a discrepancy can be investigated and replayed offline. Successful historical requests are also cached beneath `.cache/binance/requests/`, scoped by a credential fingerprint, allowing interrupted backfills to resume. The snapshot is mapped and imported once into `.cache/test-binance-v1.duckdb`, following the Nexo test’s persistent DB pattern. Transactions, account ownership, asset precision, independent API balance observations and the completed-import marker commit together. Subsequent runs reconcile balances queried from DuckDB against those saved observations without reading the snapshot, remapping or calling the API. API observations are reference data and never create ledger entries. Cache files are ignored by Git. Delete the DB to remap the saved snapshot; delete both the DB and complete snapshot to refresh the fixture (historical request caches remain reusable).

The existing ignored command also uses this persistent cache:

```sh
cargo test -p cli live_binance_account_reconciles -- --ignored
```

To explicitly fetch fresh data without the test cache:

```sh
cargo test -p cli uncached_binance_account_reconciles -- --ignored
```

Run against a quiet account: history and current positions cannot be captured atomically. Reconciliation checks every owned account, including historical assets absent from current positions, and reports exact differences without adding artificial opening balances. API errors stop the fetch. Binance's API does not guarantee lifetime history for all products (for example, universal transfers expose only the last six months, and dust records before December 2020 are unavailable); derivatives, margin, loans, P2P and retired Earn products are not implemented. Such missing activity can cause reconciliation failure and may require statement exports or additional adapters.

Delisted spot pairs may be absent from `exchangeInfo`. Supply their metadata with `BINANCE_ADDITIONAL_SYMBOLS`, for example:

```sh
export BINANCE_ADDITIONAL_SYMBOLS='[{"symbol":"BCCBTC","baseAsset":"BCC","quoteAsset":"BTC"}]'
```

To run the ordinary CLI tests before a real account snapshot is available:

```sh
cargo test -p cli -- --skip cached_binance_account_reconciles
```

Binance’s universal-transfer API retention limit cannot be bypassed with pagination. Older activity needs statement exports from Transaction History → Export Transaction Records; [Binance’s instructions](https://www.binance.com/en-GB/support/faq/how-to-generate-transaction-history-990afa0a0a9341f78e7a9298a9575163) describe generating multiple exports for periods longer than a year. A Binance statement importer is not implemented yet. Accurate ledger totals require either complete movements or independently evidenced opening balances at a known cutoff plus all subsequent movements. Opening balances must be explicit provenance-backed checkpoints, not unexplained plugs calculated to force agreement with current positions. A checkpoint provides balances from that date onward, but does not reconstruct earlier performance or cost basis. Current API positions remain an independent source of current balances.
