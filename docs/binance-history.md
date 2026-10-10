# Exporting historical Binance transactions

Use Binance’s **transaction-history export** to recover movements older than the API retention window. A current account balance statement or a Spot trade-only report does not replace the transaction history: we need transfers, deposits, withdrawals, fees, rewards and Earn activity as well as trades.

## Generate the reports

1. Open the [Binance statement export page](https://www.binance.com/en/my/download-center?type=asset-account-statement) and sign in / complete 2FA if prompted.
2. Choose a **custom date range**, **all accounts** and **all coins**. Include all transaction types if the page offers that filter. Use UTC / UTC+0 if the export offers a timezone selector; otherwise record the timezone displayed on the report. Do not reinterpret local timestamps as UTC.
3. Start at your first Binance activity and cover the history through the cutoff you want to reconcile. Generate separate reports for each year, or smaller ranges if the page requires them. Check the date/time boundaries so there are no gaps; preserve any overlap for deduplication rather than deleting rows by hand.
4. Click **Generate** once for each range, wait until it is ready, then click **Download**. Keep a note of pending requests so you do not generate duplicates. Download links are available for seven days; check the export quota shown in your account. See [Binance’s export documentation](https://www.binance.com/en-GB/support/faq/how-to-generate-transaction-history-990afa0a0a9341f78e7a9298a9575163) for current limits.

## Import the downloaded report

Save the original Binance ZIP or CSV in `.cache/imports`. Keep Binance’s original filename, including `(UTC+1)` or another timezone suffix: the `Time` column uses that timezone. Older reports with an explicit `UTC_Time` header do not require a filename timezone. Do not resave the CSV in a spreadsheet editor.

```sh
# Automatically selects a Binance export in .cache/imports (prefers ZIP):
nix run .#binance
# Explicit file, including a directly downloaded CSV:
BINANCE_STATEMENT='.cache/imports/Binance-Transaction-History-...(UTC+1)-part1-of1.csv' nix run .#binance
```

The loader handles ZIP archives with multiple CSV parts and caches extracted bytes under `.cache/test-data/binance/statements`. Archive member names are never used as filesystem paths. The same single-part ZIP and CSV have the same import identity. Each completed file is imported once into `.cache/test-data/binance/test-binance-statement-v1.duckdb`; overlapping exact movements reuse IDs while distinct repeated rows remain distinct. API movements can share those IDs when their UTC second, asset, amount and both accounts match exactly. Different representations, such as aggregated withdrawals versus separately reported fees, require explicit mapping rather than approximate matching.

The first statement test fetches current Spot, Funding and Earn positions into `.cache/test-data/binance/positions-v1.json`, without the slow all-pairs trade backfill. Positions are independent reference data: deriving them from transactions would make the comparison meaningless. It also caches API Earn movements omitted by the Spot/Funding CSV in `earn-supplement-v1.json`. Successful history-query responses are also cached as JSON under `.cache/api_responses/binance`, partitioned by a credential fingerprint. Exact endpoint/query parameters identify each file. Changed date windows are new queries; failed requests are not cached. Subsequent fixture runs use the persistent DB and cached responses without credentials or network. The balance-display command reports discrepancies; the strict assertion is:

```sh
cargo test -p cli cached_binance_statement_reconciles -- --ignored --nocapture
```

Delete the statement DB to remap cached inputs. When advancing the fixture cutoff, refresh the export, position snapshot and Earn supplement together. Statements newer than the saved reference cannot be reconciled against that reference. Activity between export and API fetch can also produce differences; the current supplement covers post-export Locked rewards, not arbitrary post-export trades or transfers. `.cache` is ignored by Git and contains private financial records.

The supplied export currently reconciles all Spot, Funding and Locked positions after adding independently recorded Earn movements. Flexible SOL remains short by 0.07192 SOL, which the retained API records do not explain. Obtain the dedicated historical Earn report to resolve that gap. The importer does not manufacture an opening balance or an interest transaction from a current-balance difference.

Browser-assisted export remains unavailable because browser inspection was blocked by the browser tool’s policy. The local importer uses downloaded files and API credentials, without reading browser login storage.
