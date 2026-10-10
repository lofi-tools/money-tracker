## ADDED Requirements

### Requirement: Binance ledger mapping
The provider SHALL fetch all API-accessible history from Binance's launch through a fixed cutoff, paginate supported endpoints, and map successful movements and fees to asset-specific owned and external positions. Decimal strings SHALL be converted without floating-point rounding. API errors and unsupported records SHALL be reported.

#### Scenario: Trade with a third-asset fee
- **WHEN** a spot trade exchanges BTC for USDT and charges BNB
- **THEN** BTC and USDT movements and the BNB fee have matching external counterparties

### Requirement: Independent balance reconciliation
The provider SHALL independently fetch current balances and compare every owned account's transaction sum with its current position, including zero positions and historical assets no longer held. It SHALL report differences without adding balancing transactions.

#### Scenario: Missing historical deposit
- **WHEN** transactions sum to less than a current API balance
- **THEN** reconciliation fails with the account and exact discrepancy

### Requirement: Real account test cache
The cached reconciliation test SHALL fetch real data on its first run using environment credentials and reuse a complete raw snapshot under repository `.cache` thereafter without credentials or network. The existing ignored `live_binance_account_reconciles` test SHALL use the same cache. A separately named ignored `uncached_binance_account_reconciles` test SHALL always fetch live data. Transactions, account metadata, independent API balance observations and a completed-import marker SHALL commit atomically into a persistent test DuckDB under repository `.cache`. Subsequent runs SHALL query that DB without remapping or importing the snapshot until the DB is deleted.

#### Scenario: Offline replay
- **WHEN** a complete cache exists and credentials are absent
- **THEN** the test imports the cached raw data if the test DB is absent and reconciles without API calls


#### Scenario: Reusing the imported ledger
- **WHEN** the test DB has a completed import, even if the raw snapshot is absent
- **THEN** the test compares persisted transaction sums with independent API observations without fetching, mapping or importing again

#### Scenario: Rebuilding the ledger
- **WHEN** the test DB is deleted and the raw snapshot remains
- **THEN** the test rebuilds the DB from the raw snapshot without API calls


### Requirement: Cached Binance balance display
An ignored test SHALL reuse the raw snapshot and persistent test DB, report fetch/cache and import progress, and display exact owned asset totals derived from transactions without asserting reconciliation. A Nix `binance` command SHALL run this test with output visible. The output SHALL indicate that missing history can affect these totals.

#### Scenario: Display imported totals
- **WHEN** the user runs `binance` with a completed test DB import
- **THEN** the test reports DB reuse and prints asset balances without fetching or reimporting transactions


### Requirement: Binance backpressure
The client SHALL honor the full Retry-After cooldown on HTTP 429, use bounded exponential fallback when the header is absent or invalid, and serialize network requests behind one limiter and cooldown shared by all client instances within the process regardless of base URL or credentials. Requests SHALL be signed after waiting. Cached responses SHALL bypass throttling. Repeated throttling SHALL reduce the subsequent request rate. Exhaustion after eight attempts and HTTP 418 bans SHALL return typed errors and preserve the shared cooldown. Rate-limit errors SHALL NOT be cached as successful responses.

#### Scenario: Long server cooldown
- **WHEN** Binance returns HTTP 429 with Retry-After of 120 seconds
- **THEN** the next network request waits at least 120 seconds and later requests use slower pacing

#### Scenario: Ban or retry exhaustion
- **WHEN** the retry limit is exhausted or HTTP 418 is returned
- **THEN** the client stops the fetch, reports the cooldown and prevents subsequent requests in that process from sending early

### Requirement: Historical statement fixtures
The importer SHALL accept original Binance transaction-history CSV and ZIP files under repository `.cache/imports`, cache extracted CSVs under `.cache/test-data`, and store Binance and Nexo test DBs under `.cache/test-data`. It SHALL honor the export timezone, preserve repeated rows, reject unsupported operations and incomplete transfer pairs before committing, and import each content hash once. Exact matching API and statement movements SHALL share IDs including occurrence counts. Different representations SHALL NOT be approximately matched.

#### Scenario: ZIP and CSV replay
- **WHEN** a single-part archive and its original CSV are imported
- **THEN** both identify the same file contents and the second import adds no transactions

#### Scenario: Independent positions and omitted Earn activity
- **WHEN** the statement fixture has no position reference or Earn supplement
- **THEN** the test fetches and caches independent API positions and supported omitted Earn movements once, without a full trade-history backfill

#### Scenario: Legacy Savings maturity omitted from the statement
- **WHEN** cached API records show one successful legacy fixed-term Savings subscription, one paid maturity and its immediate Flexible reinvestments
- **THEN** maturity interest is derived from the evidenced redemption minus principal, the original subscription deduplicates against the CSV in either import order, and replay imports the omitted movements once
- **AND** missing or ambiguous principal evidence fails import instead of deriving an adjustment from API positions
- **AND** subsequent runs reuse the persistent DB and frozen API fixtures without credentials or network
- **AND** unexplained differences fail the strict reconciliation test without generated balancing entries

### Requirement: Weighted adaptive request budgets
All Binance client instances SHALL share one HTTP transport within a process and a limiter whose budget, history and cooldown are shared across Cargo processes using the same repository cache. The limiter SHALL retain weighted call history over a rolling minute, seed budgets and request costs from documented limits with headroom, consume server usage headers and current Spot minute limits from exchangeInfo, and reduce the effective budget after throttling. Successful requests SHALL restore budget gradually without exceeding headroom. Cached responses SHALL consume no request budget. Independent mock-server tests MAY inject isolated limiters to control virtual time.

#### Scenario: Expensive fiat history
- **WHEN** fiat-history requests consume 45,000 UID weight each
- **THEN** the limiter spaces calls and waits for history expiry before exceeding its minute budget

#### Scenario: Several clients and hosts
- **WHEN** tests construct clients with different base URLs or credentials in the same process
- **THEN** they share one limiter and cooldown instead of independent URL budgets

#### Scenario: Server observes other callers
- **WHEN** response headers report usage near the effective limit
- **THEN** the next network request waits until sufficient budget is available

### Requirement: Limiter persistence across processes
The limiter SHALL coordinate parallel processes through an OS file lock and versioned state under repository `.cache/binance/limiter`. The lock inode SHALL remain separate from atomically replaced JSON state. Each network attempt SHALL persist its weighted reservation before sending; responses SHALL persist server usage, learned budgets and cooldowns before releasing the lock. Budget waits SHALL release the file lock and re-read state before admission. Malformed state or storage errors SHALL return typed errors rather than silently resetting the budget. Credentials SHALL NOT be persisted.

#### Scenario: Parallel processes
- **WHEN** separate test binaries request Binance data concurrently
- **THEN** their weighted reservations are combined without lost updates and they share pacing and cooldown

#### Scenario: Terminated lock owner
- **WHEN** a process holding the lock is terminated after saving a reservation and cooldown
- **THEN** the OS releases its lock and other processes still honor the saved history and cooldown

#### Scenario: Cached response with unavailable limiter state
- **WHEN** a historical response is cached and limiter state is malformed
- **THEN** the cache hit succeeds without acquiring the file lock or modifying the limiter state
