# Binance provider: sources, ledger mapping and diagnostics

Implemented behavior, checked against source on 2026-10-11. Acquisition is in [account.rs](../../libs/binance-client/src/account.rs), parsing in [statement.rs](../../libs/binance-client/src/statement.rs), semantics in [the adapter](../../libs/lib-app/src/adapters/binance.rs), and statement persistence/fixtures in [statements.rs](../../libs/lib-app/src/adapters/binance/statements.rs). [Account-history acceptance contracts](binance-account-history.md) cover pagination, caching and backpressure; [the export guide](../findings/binance-history.md) covers obtaining reports. [Shared conventions](../findings/provider-diagnostics.md) explain sums and test isolation.

## Coverage and acquisition

Supported owned buckets are Spot, Funding, Simple Earn Flexible and Simple Earn Locked, aggregated by asset. Loans, margin, derivatives, P2P and arbitrary retired Earn products are not implemented. The ordinary application CLI currently imports Nexo; Binance account acquisition and import are service/helper and test workflows, not a selectable production CLI connection. Binance Vision candles are a separate price source, not this account integration.

Private requests use `BINANCE_API_KEY_ID` / `BINANCE_API_KEY_SECRET`, with `BINANCE_API_KEY` / `BINANCE_SECRET_KEY` fallbacks. The code does not provision keys or configure their permissions. Supply read access to the supported endpoints; no transfer execution is part of this adapter.

### API history

`fetch_account_snapshot` attempts supported history from 2017-07-17 through a fixed UTC cutoff. Endpoint coverage below describes code, not a guarantee that Binance retains that entire history.

| Source | Endpoint and acquisition |
| --- | --- |
| Pair metadata | `/api/v3/exchangeInfo`; `BINANCE_ADDITIONAL_SYMBOLS` JSON can add delisted base/quote pairs. |
| Deposits / withdrawals | `/sapi/v1/capital/deposit/hisrec`, `/sapi/v1/capital/withdraw/history`; 89-day windows, offset pages of 1,000. |
| Dividends | `/sapi/v1/asset/assetDividend`; 179-day windows, split saturated 500-record windows. |
| Convert / dust | `/sapi/v1/convert/tradeFlow`, `/sapi/v1/asset/dribblet`; 30-day windows, split saturated 1,000 / 100-record windows. |
| Simple Earn | `/sapi/v1/simple-earn/{flexible,locked}/history/{subscriptionRecord,redemptionRecord,rewardsRecord}`; 30-day windows, pages of 100. Flexible rewards fetch BONUS, REALTIME and REWARDS. |
| Spot ↔ Funding | `/sapi/v1/asset/transfer`, MAIN_FUNDING / FUNDING_MAIN; last six calendar months, 30-day windows, pages of 100. |
| Fiat orders / payments | `/sapi/v1/fiat/{orders,payments}`; transactionType 0 / 1, 30-day windows, pages of 500. |
| Spot trades | `/api/v3/myTrades` per known pair; pages of 1,000, advancing `fromId`, filtered to cutoff. |

Repeated/nonadvancing pages fail rather than silently truncating; window splitting cannot overcome provider retention. Missing delisted pair metadata or unsupported account history can leave discrepancies. All client instances share weighted request budgets, transport and cooldown; repository processes coordinate through `.cache/binance/limiter/state-v1.json` and its separate `state.lock`. Cached successful historical responses bypass request budgets. HTTP 429 waits/retries, HTTP 418 fails with retained cooldown. See [the limiter contracts](binance-account-history.md).

### Independent current positions

| Bucket | Reference endpoint | Quantity |
| --- | --- | --- |
| Spot | `/api/v3/account` | free + locked |
| Funding | POST `/sapi/v1/asset/get-funding-asset` | free + locked + freeze + withdrawing |
| Flexible | `/sapi/v1/simple-earn/flexible/position`, paginated | totalAmount |
| Locked | `/sapi/v1/simple-earn/locked/position`, paginated | amount |

All four balance buckets must be present in a full mapped snapshot, even if empty; unknown buckets fail. Reference balances are independently fetched and never create ledger effects. API calls across history and current endpoints are not atomic. The full-history snapshot labels the history cutoff chosen before acquisition; balances are fetched afterward. A current-only snapshot records its cutoff after fetching positions. Activity during either sequence can cause a legitimate discrepancy.

### Transaction-history CSV / ZIP

`import_statement` accepts `Time` or `UTC_Time`, `Account`, `Operation`, `Coin`, `Change`, and optional `Remark`. `UTC_Time` is UTC; `Time` requires the original filename's `(UTC)` or `(UTC±offset)` metadata. Parsing trims fields, handles a BOM, normalizes timestamps to UTC seconds and parses `Change` exactly. Supported CSV account labels are Spot and Funding; Earn operations supply the other owned leg. ZIP CSV members are sorted and cached under generated safe filenames; member names are never filesystem paths.

File identity uses a fixed-key HMAC-SHA256 fingerprint over timezone metadata and framed CSV part bytes. This key is a public identity constant, not an account credential. Equivalent single-part ZIP/CSV payloads with the same timezone share identity. Renaming a `Time` report to a different timezone changes meaning and identity. Unknown operations and unmatched Spot/Funding transfer pairs fail before commit.

## Mapping to DB transactions

An adapter `Event` contains a UTC time and `Movement`s `(source bucket, target bucket, asset, exact amount)`. Each nonzero movement becomes a core transaction input on source and output on target of equal units. Owned positions are `BINANCE:<spot|funding|flexible|locked>:<ticker>`, role `Asset`. `BINANCE:external:<purpose>:<ticker>` positions have role `ExternalCounterparty`, with purposes including deposit, withdrawal, market, fees, rewards, fiat-payment and spending. They are balancing legs, never user liabilities.

| API source record | Normalized movement |
| --- | --- |
| Deposit, status 1 or 6 | external:deposit → wallet selected by walletType (Spot / Funding), coin/amount, insertTime. |
| Withdrawal, status 4 or 6 | Wallet → external:withdrawal for amount, and wallet → external:fees for transactionFee in coin, applyTime. |
| Dividend | external:rewards → Spot; direction 0 reverses it. asset/amount, divTime. |
| Convert SUCCESS | Spot sends fromAsset/fromAmount to market; market sends toAsset/toAmount to Spot, createTime. |
| Dust detail | Spot sends fromAsset/amount to market; market sends net transferedAmount to Spot in targetAsset (default BNB), and serviceChargeAmount to fees. Fee is not debited from Spot again. |
| Spot trade | Base qty and quote quoteQty exchange with market according to isBuyer; Spot pays commission in commissionAsset to fees, time. |
| Earn subscription | Spot and/or Funding → Flexible/Locked; explicit split quantities must sum to amount. Failed/pending records are skipped; unknown successful-status representations fail. |
| Earn redemption | Flexible/Locked → Spot/Funding/Flexible according to destination. `NEW_TRANSFERRED` Locked defaults to Flexible. lossAmount → fees; received + loss must equal originalAmount if supplied. Incomplete/pending records are skipped; deliverDate is preferred to time. |
| Earn reward | Flexible REALTIME → credit Flexible from rewards; other Flexible rewards and Locked rewards → credit Spot. |
| Confirmed MAIN_FUNDING / FUNDING_MAIN | Owned bucket → owned bucket, timestamp. |
| Successful fiat order | Fiat deposit credits Spot and debits totalFee; withdrawal debits amount plus totalFee. |
| Completed fiat payment | Crypto purchase/sale via external:fiat-payment; paymentMethod `Cash Balance` also records its Spot fiat source leg. Full card-fiat cost/provenance is not persisted. |

Most API times are epoch milliseconds; withdrawal applyTime also accepts UTC date/time strings. Exact identical JSON rows are deduplicated within each history kind before mapping. This is not cross-endpoint economic deduplication and can collapse distinct records if their complete JSON is identical.

CSV changes are signed: positive changes credit the named owned account from the classified counterparty, negative changes reverse direction. Operation groups map as follows:

| Statement operations | Other leg |
| --- | --- |
| Deposit / Withdraw | external:deposit / external:withdrawal |
| Fee / Transaction Fee | external:fees |
| Buy/Sell, Transaction Buy/Spend/Revenue/Sold/Related, Small Assets Exchange BNB, Binance Convert | external:market |
| Simple Earn Flexible Subscription / Redemption | Flexible |
| Staking Purchase / Redemption, Simple Earn Locked Subscription / Redemption | Locked |
| Spending / Merchant Payment | external:spending |
| Distribution, Commission History, Asset - Transfer, supported interest/reward/airdrop/cashback/voucher operations | external:rewards; exact supported labels are in `import_statement`. |
| Transfer Between Spot and Funding | Paired owned debit/credit with matching time, coin and absolute amount; counted once. |

Subscription changes must be negative and redemptions positive. Legacy fixed-term `DAYSS` Savings maturity interest is redemption minus principal, requiring one matching earlier successful subscription and one paid settlement with no loss. Missing/ambiguous evidence fails; current balances never supply the principal. Supported immediate reinvestments use the same owned movement identities as statements. This special case is not general support for every retired product.

Synthetic third-asset fee example: buy 0.01 BTC for 500 USDT with 0.001 BNB commission. DB effects are market → Spot `1000000` BTC units, Spot → market `50000000000` USDT units, and Spot → fees `100000` BNB units at scale 8. External legs make each asset's signed transaction total zero; owned changes are +0.01 BTC, −500 USDT and −0.001 BNB. An internal 1 ETH subscription reduces Spot and increases Flexible by equal units, leaving total owned ETH unchanged.

## Precision, persistence and identities

Source decimal parsing is exact but bounded by `rust_decimal`; ledger units are arbitrary-precision integers. Snapshot assets use at least eight decimal places, increasing to the greatest normalized precision needed by movements or observations. Statement/API-history imports respect already registered scales and fail if a new amount cannot be represented exactly. They do not silently rescale existing effects.

`import_statement` and `import_api_history` save registrations, effects and a completed marker atomically. Markers are `BINANCE-STATEMENT-v1` / file fingerprint and `BINANCE-API-HISTORY-v1` / serialized snapshot fingerprint. Each normalized one-movement transaction is serialized, fingerprinted and assigned `BINANCE-STATEMENT:<fingerprint>:<occurrence>`. API history normalizes to UTC seconds to share those IDs with matching CSV movements. Occurrence counts preserve identical repeated movements within an import. Matching requires identical mapped representation, timestamp, asset, units and both positions; differing fee/aggregation representations are not approximately merged. Partial overlaps with ambiguous identical occurrences remain a limitation, not a general provenance-aware matcher.

The full API test uses a different fixture scheme: `BINANCE-TEST:<index>` IDs and fixed marker `(binance-account-test, account-v1)`. Its registrations, effects, independent balance observations and marker commit together. Do not combine this fixture's ledger with the statement/API-history ledger: their event identities do not deduplicate against each other.

Known owned buckets migrate to `legacy:binance:<bucket>` accounts under the default local portfolio; external positions have no owned account. The adapter still uses provider-wide IDs. Multi-user/multi-exchange-account selection and the proposed asset registry are future work. Transient adapter Product values have placeholder APY 0 and are not verified product rates. Semantic source types/IDs/remarks are not a persisted earnings/provenance model.

## Asset and liability sums; reconciliation

Sum exact signed units across owned buckets at a cutoff, grouped by asset and protected scale. Exclude every external bucket; otherwise deposits, market legs or fees could be mistaken for holdings or debt. This integration currently produces no `Liability` positions; a missing loan adapter does not establish that the real account has zero debt.

`BinanceAccountData::reconcile` first checks per-asset effect conservation, then compares each owned position's transaction sum with independent API observations. The comparison uses the union of historical and observed positions, treating absent amounts as zero, and reports exact differences in smallest units. It does not add opening-balance plugs. Valid structure plus a balance mismatch can be committed to the API test fixture so the mismatch continues to fail on replay.

A statement import alone checks structure without an independent reference. The statement test adds recorded Earn movements and compares DB balances to separately cached API positions, rescaling exactly to DB scales. It rejects a ledger ending after the reference and a supplement/reference cutoff mismatch. A ledger ending before the reference can still miss intervening trades/transfers; the supplement is not a general bridge for all post-export activity. Reconciliation of one frozen account fixture proves that fixture's final balances, not universal historical coverage or correct performance classification.

## Provider-only print tests and cache layout

Run commands from the repository root; full names avoid selecting other tests accidentally:

```sh
# Default Nix wrapper: statement + Earn supplement display, reports mismatches:
nix run .#binance
BINANCE_STATEMENT='.cache/imports/Binance-Transaction-History-...(UTC).csv' nix run .#binance
cargo test -p lib-app adapters::binance::statements::tests::latest_amount_after_cached_statement -- --exact --ignored --nocapture
# Same workflow, but fail on reconciliation differences:
cargo test -p lib-app adapters::binance::statements::tests::cached_binance_statement_reconciles -- --exact --ignored --nocapture
# Fetch/display independent positions without full-history backfill:
cargo test -p lib-app adapters::binance::statements::tests::fetch_cached_positions -- --exact --ignored --nocapture
# Alternative full-history API fixture display and strict reconciliation:
cargo test -p lib-app adapters::binance::tests::latest_amount_after_cached_account -- --exact --ignored --nocapture
cargo test -p lib-app adapters::binance::tests::cached_binance_account_reconciles -- --exact --nocapture
# Explicit fresh API acquisition; ignored live_binance_account_reconciles is cache-first:
cargo test -p lib-app adapters::binance::tests::uncached_binance_account_reconciles -- --exact --ignored --nocapture
# Deterministic CLI regressions: skip the nonignored real-account test:
cargo test -p lib-app -- --skip cached_binance_account_reconciles
```

| Fixture/input | Path and reuse |
| --- | --- |
| Statement source | `BINANCE_STATEMENT` absolute or repository-relative path; otherwise sorted `.cache/imports/Binance-Transaction-History*.{csv,zip}`, preferring the last ZIP. Only one selected file per test run; successive explicit runs can add ranges. |
| Extracted statement CSVs | `.cache/test-data/binance/statements/`; fingerprint/index filenames. |
| Statement ledger | `.cache/test-data/binance/test-binance-statement-v1.duckdb`; completed input hashes skip remapping. |
| Independent statement reference | `.cache/test-data/binance/positions-v1.json`; fetch current positions once if missing. These observations remain JSON, not persisted DB observations. |
| Statement Earn supplement | `.cache/test-data/binance/earn-supplement-v1.json`; missing file triggers API collection. Flexible REALTIME interest, evidenced legacy maturities/reinvestments, Locked-to-Flexible transfers and post-statement Locked rewards. |
| Full API snapshot | `.cache/binance/account-v1.json`; fetch entire supported history and positions if missing. |
| Full API ledger and observations | `.cache/test-data/test-binance-api-v1.duckdb`; completed marker permits reuse without even reading the raw snapshot. |
| Historical query responses | `.cache/api_responses/binance/`; credential-fingerprint partition and exact endpoint/query identity, atomic successful-response writes. |

Statement display prints nonzero exact asset totals at the latest ledger timestamp and compares to the reference cutoff. A mismatch is printed but returns success; use the strict test for acceptance. API fixture display prints exact transaction-derived totals and a missing-history warning, without asserting balance agreement. Statement supplement movements and its skip marker are separate idempotent commits; the entire CSV + reference + supplement workflow is not one transaction.

First statement runs can call the API for references/supplement; first full API runs can take a long time for all pairs. Frozen complete inputs permit offline replay. Cargo `--offline` controls dependency fetching, not HTTP requests inside tests. To inspect a remap, preserve original fixtures and rebuild a disposable fixture DB from cached inputs. Schema upgrades alone preserve effects/import markers. Advancing a statement fixture requires a matching export, reference and supplement cutoff; swapping only a JSON file or reusing the fixed full-API marker does not refresh the ledger.

Regression coverage includes exact amount parsing, independent mismatch persistence, DB reuse without raw JSON, remapping from raw JSON, repeated/overlapping statement rows, unsupported operations and incomplete transfers committing nothing, API/CSV deposit deduplication in both orders, legacy Savings evidence, and owned Earn migration/redemption destinations. Client mock tests cover pagination, response caches and limiter behavior without private data. Product terms, account-aware diagnostics, complete source provenance, earnings classification and unsupported account families remain [outstanding work](../todo/master-todo.md).
