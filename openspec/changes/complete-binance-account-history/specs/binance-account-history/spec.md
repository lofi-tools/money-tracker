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
