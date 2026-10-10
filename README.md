# Money Tracker

Track crypto across accounts, chains, apps, platforms.
Estimate historical value and current revenue.


## Developer quickstart

Setup deps using `direnv allow` (needs Nix and nix-direnv), then:
- Run unit tests: `utest`
- Run program: `run`

## Nexo amounts and historical net worth

From the repository root, import `.cache/imports/nexo_transactions_05-10-2026_10-36-53.csv` and fetch prices into a local DuckDB file:

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

See [Exporting historical Binance transactions](docs/binance-history.md) for the website steps, date ranges, cache location and current importer status.

To load the cached account and display exact asset balances with progress logs, run `binance` in the Nix dev shell (or `nix run .#binance` from the repository). This runs the ignored `latest_amount_after_cached_statement` test with output visible, like `nexo`. It reads a Binance ZIP or CSV from `.cache/imports`, caches decompressed CSVs under `.cache/test-data/binance/statements`, and imports once into `.cache/test-data/binance/test-binance-statement-v1.duckdb`. Set `BINANCE_STATEMENT=PATH` to select a particular file. It prints transaction-derived totals and any reconciliation differences without failing the display command. The Nexo test DB is also under `.cache/test-data`.

The Binance adapter maps API-accessible Spot, Funding and Simple Earn activity to separate owned accounts per asset, with external deposit, withdrawal, market, reward and fee counterparties. It fetches deposits, withdrawals, spot trades (including fees in another asset), conversions, dust conversions, dividends, fiat orders/payments, Funding transfers and Earn subscriptions/redemptions/rewards. Decimal strings are converted exactly; per-asset precision is at least eight decimal places and increases when the snapshot requires it. Locked Earn positions are aggregated by asset.

Set `BINANCE_API_KEY_ID` and `BINANCE_API_KEY_SECRET` in your direnv environment, then run (the earlier `BINANCE_API_KEY` and `BINANCE_SECRET_KEY` names remain supported as fallbacks):

```sh
cargo test -p cli cached_binance_account_reconciles
```

The first run backfills history from Binance's launch and independently fetches current balances. This can take a long time: trade history requires a request for every known pair, and other endpoints impose rate limits. A complete raw snapshot is saved to `.cache/binance/account-v1.json` before reconciliation, so a discrepancy can be investigated and replayed offline. Successful historical requests are also cached beneath `.cache/api_responses/binance/`, scoped by a credential fingerprint, allowing interrupted backfills to resume. Each successful history response is saved atomically as JSON, including empty pages; errors are not cached. Cache keys include the endpoint and exact query parameters, so a changed end time creates a new request. Exchange metadata and current balances are fetched fresh until the complete test snapshot is frozen. The snapshot is mapped and imported once into `.cache/test-data/test-binance-api-v1.duckdb`, following the Nexo test’s persistent DB pattern. Transactions, account ownership, asset precision, independent API balance observations and the completed-import marker commit together. Subsequent runs reconcile balances queried from DuckDB against those saved observations without reading the snapshot, remapping or calling the API. API observations are reference data and never create ledger entries. Cache files are ignored by Git. Delete the DB to remap the saved snapshot; delete both the DB and complete snapshot to refresh the fixture (historical request caches remain reusable).

The existing ignored command also uses this persistent cache:

```sh
cargo test -p cli live_binance_account_reconciles -- --ignored
```

To explicitly fetch fresh data without the test cache:

```sh
cargo test -p cli uncached_binance_account_reconciles -- --ignored
```

Run against a quiet account: history and current positions cannot be captured atomically. Reconciliation checks every owned account, including historical assets absent from current positions, and reports exact differences without adding artificial opening balances. HTTP 429 responses pause requests for the complete `Retry-After` duration; missing or invalid headers use exponential backoff. All Binance clients and real-account tests in a process share one HTTP connection pool and one limiter, regardless of base URL or credentials. The limiter records a rolling minute of weighted requests, starts at 80% of documented budgets, spaces heavy calls, and accounts for usage reported in Binance response headers. Spot’s minute budget is refreshed from `exchangeInfo`. Throttling lowers the estimated budget using observed load; successful calls recover it gradually. Parallel Cargo processes and test binaries in this repository also share the budget via an OS file lock and persisted state under `.cache/binance/limiter`. Requests are retried up to eight attempts, with cooldown progress printed by the cached tests. HTTP 418 bans stop the fetch and retain the cooldown. Other API errors stop the fetch. Binance's API does not guarantee lifetime history for all products (for example, universal transfers expose only the last six months, and dust records before December 2020 are unavailable); derivatives, margin, loans, P2P and retired Earn products are not implemented. Such missing activity can cause reconciliation failure and may require statement exports or additional adapters.

Delisted spot pairs may be absent from `exchangeInfo`. Supply their metadata with `BINANCE_ADDITIONAL_SYMBOLS`, for example:

```sh
export BINANCE_ADDITIONAL_SYMBOLS='[{"symbol":"BCCBTC","baseAsset":"BCC","quoteAsset":"BTC"}]'
```

To run the ordinary CLI tests before a real account snapshot is available:

```sh
cargo test -p cli -- --skip cached_binance_account_reconciles
```

Binance’s universal-transfer API retention limit cannot be bypassed with pagination. Older activity needs statement exports from Transaction History → Export Transaction Records; [Binance’s instructions](https://www.binance.com/en-GB/support/faq/how-to-generate-transaction-history-990afa0a0a9341f78e7a9298a9575163) describe generating multiple exports for periods longer than a year. The statement importer accepts CSV and ZIP, preserves repeated rows, pairs Spot/Funding transfers, and maps supported Spot/Funding/Earn operations. Unknown operations and incomplete transfer pairs fail before import. Matching API and statement movements share transaction IDs based on UTC second, asset, exact amount, both accounts and occurrence; records that differ in representation are not assumed equivalent. Accurate ledger totals require either complete movements or independently evidenced opening balances at a known cutoff plus all subsequent movements. Opening balances must be explicit provenance-backed checkpoints, not unexplained plugs calculated to force agreement with current positions. A checkpoint provides balances from that date onward, but does not reconstruct earlier performance or cost basis. Current API positions remain an independent source of current balances.

The statement fixture independently fetches Spot, Funding and Earn positions once into `.cache/test-data/binance/positions-v1.json`. It also caches API-only Flexible real-time interest, legacy fixed-term Savings maturities and their immediate reinvestments, Locked-to-Flexible redemptions and Locked rewards after the statement cutoff in `earn-supplement-v1.json`. These transactions are imported once; API balances are comparison data, never derived from the ledger. Subsequent runs need neither credentials nor network. Delete the statement DB to rebuild from the cached inputs; refresh the statement, positions and supplement together when advancing the cutoff.

```sh
# Independent positions only (four endpoint families; no full-history backfill):
cargo test -p cli fetch_cached_positions -- --ignored --nocapture
# Assert exact account reconciliation for the statement fixture:
cargo test -p cli cached_binance_statement_reconciles -- --ignored --nocapture
```

The supplied October 2026 export plus cached Earn history now matches every Spot, Funding, Flexible and Locked position, including a NEAR reward after the statement cutoff. The previous **0.07192 SOL** deficit was interest on a legacy 15-day Savings product: the API records show a 5 SOL subscription on 14 August 2022, a 5.07192 SOL maturity on 30 August, and immediate reinvestment of principal and interest into Flexible Earn. The CSV omits that maturity/reinvestment sequence. The mapper recognizes legacy `DAYSS` products and requires one matching successful principal subscription and one paid maturity before recognizing the interest; ambiguous or missing evidence fails import. The original subscription deduplicates against the CSV, in either import order. Four original records were recovered from three endpoint/query-verified response-cache files into the existing supplement, without fetching or modifying API positions; their provenance is saved in `.cache/test-data/binance/legacy-savings-reconciliation.json`, and the original supplement is preserved as `earn-supplement-v1.before-legacy-savings.json`. The strict cached statement test passes with SOL totaling **35.25865379**.

Rate-limit seeds follow [Binance’s limits documentation](https://developers.binance.com/en/docs/products/wallet/general-info): Spot 6,000 weight/minute; SAPI endpoint IP limits 12,000/minute and UID limits 180,000/minute. The shared limiter keeps separate endpoint counters inside its single state because SAPI limits are endpoint-specific. [Fiat history](https://developers.binance.com/en/docs/catalog/investment-and-services-fiat/api/rest-api/~) costs 45,000 UID weight/call; [withdrawal history](https://developers.binance.com/en/docs/catalog/core-trading-wallet/api/rest-api/capital) costs 18,000; [Convert history](https://developers.binance.com/en/docs/catalog/core-trading-convert/api/rest-api/trade) costs 3,000; [Earn history/positions](https://binance.github.io/binance-connector-js/classes/_binance_simple-earn.SimpleEarnRestAPI.FlexibleLockedApi.html) cost 150 IP weight. Unknown read endpoints start with the conservative Earn estimate; these are initial estimates, not a guarantee against external callers consuming the same IP/account budget. Cache hits consume no budget. Mock-server unit tests inject isolated limiters for their independent virtual clocks.

The limiter stores `state-v1.json` separately from the stable `state.lock` file. It saves each weighted reservation before sending a request and persists usage feedback, learned limits and cooldown deadlines before unlocking. The lock covers the HTTP response, so another process cannot send before a newly received 429 cooldown is recorded. Budget waits release the file lock and recheck the latest state on waking; cancelling a request or terminating its process releases the OS lock while saved reservations and cooldowns remain. Cache hits bypass the limiter entirely. State uses Unix timestamps to coordinate different processes and survives test restarts; malformed state or I/O errors stop the fetch with typed Snafu errors. No API credentials or signed URLs are stored. To explicitly reset limiter state, stop all Binance test/fetch processes and remove `state-v1.json`; preserve the lock file. Already-running older binaries need restarting to use the persisted limiter.
