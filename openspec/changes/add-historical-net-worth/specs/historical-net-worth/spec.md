## ADDED Requirements

### Requirement: Nexo movement history
The system SHALL import Nexo CSV movements into a local ledger using stable transaction IDs and preserve asset amounts to eight decimal places. The system SHALL store a hash of the exact CSV file bytes only when its complete transaction import commits, and SHALL skip imports of files whose hash has already committed.

#### Scenario: Reimport
- **WHEN** the same Nexo CSV is imported twice
- **THEN** the final asset amounts remain unchanged

#### Scenario: Identical file
- **WHEN** a successfully imported export is run again, including from a different path
- **THEN** the transaction write is skipped

#### Scenario: Changed export
- **WHEN** a CSV's file bytes change
- **THEN** the new file is imported and stable transaction IDs update existing effects

#### Scenario: Failed import
- **WHEN** any transaction in a CSV fails to commit
- **THEN** its file hash is not recorded and a later retry can import the file

### Requirement: CoinGecko price persistence
The system SHALL persist current and recent historical CoinGecko price observations in DuckDB by asset, quote asset, and timestamp. The CLI SHALL fetch USD-valued prices only for assets that have at least one effect on an owned position, including assets no longer held. The system SHALL track successfully fetched historical intervals per asset and USD quote, and fetch only uncovered intervals on subsequent runs. For older periods outside CoinGecko's public historical window, the system SHALL fetch daily spot closes from Binance Vision monthly archives, use USDT as a USD proxy when no direct USD pair exists, and cache successfully fetched traded pair months in DuckDB.

#### Scenario: Historical price import
- **WHEN** a requested range returns price observations
- **THEN** each observation is available to valuation queries at or after its timestamp

#### Scenario: First import
- **WHEN** an owned asset has transactions and no cached history
- **THEN** the requested price interval begins one day before its earliest transaction

#### Scenario: Newer or older transactions
- **WHEN** later imports extend the wanted interval beyond previously fetched history
- **THEN** only the uncovered earlier or later intervals are fetched and stored

#### Scenario: Past holding
- **WHEN** an owned asset has been sold but has historical transaction effects
- **THEN** its USD prices remain eligible for historical valuation

#### Scenario: Older history beyond CoinGecko's public window
- **WHEN** an uncovered historical interval is older than CoinGecko's public limit
- **THEN** daily closes are downloaded from Binance Vision without querying CoinGecko for that interval

#### Scenario: Cached Binance pair month
- **WHEN** an archive pair month has already been fetched successfully
- **THEN** the stored candles are reused without downloading that archive again

#### Scenario: Known absent Binance pair month
- **WHEN** an archive pair month previously returned 404
- **THEN** its absence is reused without requesting that archive again

#### Scenario: Pair absent from Binance
- **WHEN** a required older Binance spot pair is unavailable
- **THEN** the system SHALL use KuCoin daily candles for a supported crypto pair, ECB reference rates for GBP/EUR, or Nexo export-implied USD rates for NEXO, and SHALL surface an error if no price is available

### Requirement: Historical net worth
The system SHALL calculate net worth as the sum of each owned asset amount multiplied by its latest available quote price at each historical timestamp.

#### Scenario: Price or balance changes
- **WHEN** an asset balance or its price changes
- **THEN** the corresponding net worth point reflects the new amount and applicable price

#### Scenario: Missing price
- **WHEN** a nonzero owned asset has no applicable price
- **THEN** the query fails rather than returning an incomplete total
