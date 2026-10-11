# Kraken provider: account-scoped ledger and balance reconciliation

Implemented Spot REST foundation. Acquisition/signing/cache code lives in [kraken-client](../../libs/kraken-client/src/lib.rs); mapping, persistence, reconciliation and print tests live in [the CLI adapter](../../apps/cli/src/adapters/kraken.rs). Checked against official documentation on 2026-10-11. This release is verified with synthetic and mock-server fixtures; a private live-account result has not been established.

For user setup, OAuth/localhost evidence, historical coverage and manual-export instructions, see [Connecting Kraken](../findings/kraken-connection.md). API-key acquisition is implemented; Kraken Connect OAuth and CSV ingestion are proposed follow-ups.

## Scope and sources

Fetch the authenticated user's default Kraken wallet. Stable **local** account selection is implemented; selecting Kraken's remote `account_id`, multiple remote wallets/subaccounts and regional account variants needs further work.

| Endpoint | Use |
| --- | --- |
| GET `/0/public/Assets` | Provider asset identifiers, altname and decimals; currency class only. |
| POST `/0/private/Ledgers` | All ledger types from start 0 through a fixed whole-second UTC cutoff; 50-row offset pages, including trade and actual fee records. |
| POST `/0/private/BalanceEx` | Independent quantities after history acquisition: balance, credit, credit_used and hold_trade. |

Kraken documents `ofs` but now prefers cursor pagination; this first adapter uses the documented offset/count contract. It rejects repeated ledger IDs, early empty pages, inconsistent counts, oversized pages and count overruns. Ledger history is the monetary source; trade fills are not imported again. Kraken explains that ledger fees are actual deductions in the entry's asset, while trade-history fees can be estimates: [ledger versus trades](https://support.kraken.com/ca/articles/115000302707-differences-between-ledger-and-trades-history). See [Ledgers](https://docs.kraken.com/api-reference/account-data/get-ledgers-info), [Assets](https://docs.kraken.com/api-reference/market-data/get-asset-info) and [BalanceEx](https://docs.kraken.com/api-reference/account-data/get-extended-balance).

Fresh fetches require `KRAKEN_API_KEY` and `KRAKEN_API_SECRET` (base64-encoded secret). Grant Query ledger entries and Query funds, as required by those endpoints. Signing uses Kraken's documented HMAC-SHA512 construction with a SHA256 payload digest, encoded form body and increasing nonce. Tests use the [official authentication test vector](https://docs.kraken.com/exchange/guides/rest/authentication); no order/transfer execution endpoints are exposed. API-key action 2FA/OTP is not implemented.

Private calls share a process-wide serialization gate, monotonic millisecond nonces and four-second pacing. HTTP 429 honors integer Retry-After seconds; absent/invalid headers use exponential fallback. API rate-limit/throttle errors retry up to five attempts, honoring an explicit throttle timestamp where supplied. Other HTTP/API failures stop acquisition without printing raw provider error contents or signed request headers. This is not Binance's cross-process weighted limiter: avoid concurrently using the same Kraken key from other processes, and use a dedicated key where possible. Public metadata calls have ordinary timeout/status checking rather than private retry handling.

## Asset matching, exact amounts and positions

Resolve each source code against Assets keys or altnames, rejecting missing/conflicting metadata. Explicit aliases are XBT → BTC and XDG → DOGE; fiat such as ZUSD resolves through its USD altname. There is no generic removal of X/Z prefixes and no automatic guessing from a symbol. Unknown metadata, non-currency assets, KFEE fee credits and unsupported suffixes fail with no completed import marker. Registry-backed overrides/quarantine workflows remain future model work.

| Source suffix | Owned bucket | Economic asset |
| --- | --- | --- |
| None | spot | Resolved base asset |
| `.S` | staking | Same base asset |
| `.M` | opt-in | Same base asset |
| `.B` | earn | Same base asset |
| `.F` | rewards | Same base asset |

Each bucket has its own balance, but portfolio totals sum these distinct quantities only once. `.T` tokenized assets and other suffixes are rejected, not stripped and mistaken for crypto. Suffix meanings follow [Kraken's balance reference](https://docs.kraken.com/api-reference/account-data/get-extended-balance). Individual Earn strategies, allocations, maturity schedules, rates and underlying exposure metadata are not implemented by suffix recognition.

Amounts/fees/balances remain plain decimal strings converted directly to arbitrary-precision `Units`, without `f64` or bounded-decimal intermediates. Numeric BalanceEx fields retain their lexical decimal through serde_json arbitrary precision. New scales use the greatest of metadata decimals, significant source precision and 8; existing scales are respected when all source quantities fit, and insufficient precision fails rather than rescaling existing ledger effects. Epoch-second timestamps preserve up to nanosecond precision and fail outside the supported timestamp range.

Owned local position IDs are `<bucket>:<canonical asset>`; external positions are `external:<purpose>:<asset>`. `Store::save_account_import` namespaces **all** position IDs, ledger transaction IDs, import sources and observations by the stable local AccountId. Owned roles are Asset; external fee/market/funding/reward/transfer legs are ExternalCounterparty. Distinct accounts can import the same ledger IDs and identical bytes without collisions. Labels and API keys do not define economic identity. Accounts must belong to an existing portfolio; the default local portfolio exists automatically.

## Ledger and fee mapping

Each Kraken ledger ID produces one core transaction at the source timestamp. Its signed `amount` credits or debits the resolved owned bucket against an external purpose leg. A nonnegative `fee` separately debits that bucket and credits the external fee position in the **same asset**. Do not subtract the fee from `amount` and then debit it again. Kraken's documented balance relation is previous balance + amount − fee: [ledger fields](https://support.kraken.com/articles/360001169383-how-to-interpret-ledger-history-fields).

| Ledger type | Purpose / behavior |
| --- | --- |
| deposit / withdrawal | External deposit / withdrawal leg; source sign is retained. |
| trade / spend / receive | External market leg; related currency entries each keep their own ledger ID, asset and fee. |
| staking | External rewards leg. |
| earn subtype reward/rewards | External rewards leg. |
| other earn / transfer | External transfer leg; allocations represented by owned debit/credit source rows preserve aggregate ownership when both are present. |
| adjustment | External adjustment leg, retaining source movement. |
| margin, rollover, credit, settled and other unknown types | Unsupported: reject the snapshot before writing effects/observations/marker. |

Synthetic example at scale 8: `amount=1 BTC, fee=0.001 BTC` yields +100000000 units on Spot with an external deposit debit, plus −100000 Spot units with an external fee credit. Net owned BTC is 0.999. A trade debiting BTC and crediting USD maps each ledger row separately; a USD fee remains in USD. A 0.2 BTC allocation has a Spot −0.2 row and Earn +0.2 row, leaving combined BTC unchanged. No trade-fill supplement is added that could duplicate these movements.

Raw snapshots retain ledger refid, type/subtype, source asset, fee and reported post-entry balance. Core DB transactions currently retain effects/time/ledger identity, not those semantic metadata fields; refid grouping, earnings categories and a full audit/provenance model remain proposed. A ledger's reported post-entry balance is not used to manufacture an opening adjustment or replace independent observations.

## Independent reconciliation and liabilities

BalanceEx `balance` is independently fetched, mapped to owned buckets and persisted atomically with registrations, effects and the account-scoped completed marker. Holds remain part of balance; do not add or subtract `hold_trade` when calculating holdings. Preserve the actual API observation timestamp separately from the earlier history cutoff: these endpoints cannot be captured atomically.

Reconciliation queries only the selected account at the history cutoff and compares exact units with persisted observations over the union of historical/current positions; absent quantities are zero. Closed positions and current assets without history participate. Differences remain visible and cause the CLI/strict print test to fail; no balancing entries are generated. Structurally valid mismatching snapshots are retained for investigation and continue failing on replay.

Nonzero credit or credit_used, negative current balances and unsupported debt ledger types fail mapping. There is no current Kraken liability/credit/margin/derivatives integration. A successful supported Spot balance reconciliation does not imply full real-world net worth, Earn strategy coverage or historical performance accuracy. Activity between history cutoff and BalanceEx can cause a mismatch; rerun an explicit refresh against a quiet account rather than adjusting the ledger to the snapshot.

## Cached replay and ordinary CLI

Run from the repository root, with global flags before the subcommand:

```sh
# Fetch once if no frozen snapshot exists; persist in the normal portfolio DB:
cargo run -p cli -- --data-dir ./data --cache-dir .cache kraken --account kraken:primary
# Replay without credentials or provider HTTP calls:
cargo run -p cli -- --offline --data-dir ./data --cache-dir .cache kraken --account kraken:primary
# Explicitly refresh history + independent positions together:
cargo run -p cli -- --data-dir ./data --cache-dir .cache kraken --account kraken:primary --refresh
# Use a specific frozen snapshot, whose account_id must match:
cargo run -p cli -- --offline --data-dir ./data kraken --account kraken:primary --snapshot /path/to/snapshot.json
```

The command bypasses Nexo acquisition/price fetching and uses `${data_dir}/portfolio.duckdb`. `--portfolio ID` selects an existing portfolio. Output gives exact per-position and aggregate asset quantities, cutoff/observation dates and a strict reconciliation result. Account filtering works even when the DB contains Nexo, Binance and other Kraken accounts. This command does not run a new Kraken-specific pricing or yield pipeline.

Default cache path is `${cache_dir}/kraken/<SHA256(local AccountId)>.json`. Snapshots contain a local account binding, public-key fingerprint, asset metadata, ledger and independent balances, with no credentials/signatures. A mismatching local account cache is rejected. The binding is an explicit user assignment, not proof of Kraken account ownership: use a different local ID for each real account; rotate keys under the same ID only for that same account.

Completed snapshots are frozen and atomically written; cached replay creates no client. `<snapshot stem>.pending.json` preserves account/key fingerprint and cutoff for interrupted acquisition; `<snapshot stem>.pages/` stores validated successful ledger responses keyed by public-key fingerprint inputs, endpoint and exact query. A resumed fetch reuses pages but obtains fresh metadata/current balances. Failed requests are not cached. Refresh chooses a new cutoff and replaces the complete snapshot only after acquisition succeeds. New serialized snapshot hashes import new observations, while stable ledger IDs replace matching effects without double counting. Full snapshot hash/import marker includes source data, not a fixed fixture version string.

Schema migrations need no new step and remain ordered/idempotent on every Store open. They preserve existing Nexo/Binance data. Remapping after a future mapper change requires a deliberate version/rebuild plan: existing completed markers skip mapping. Missing/corrupt caches fail visibly offline. Cargo `--offline` alone only disables dependency fetching; use the application's `--offline` flag to prevent acquisition.

## Provider-only tests and acceptance

```sh
cargo test -p kraken-client
cargo test -p cli adapters::kraken::tests::
# Private cache-first, strict reconciliation + exact provider-only print:
nix run .#kraken
KRAKEN_ACCOUNT_ID=kraken:primary cargo test -p cli adapters::kraken::tests::latest_amount_after_cached_account -- --exact --ignored --nocapture
```

The ignored private test defaults to AccountId `kraken:primary` and uses `.cache/test-data/kraken/<account hash>.json` with `.cache/test-data/kraken/kraken.duckdb`. First run can fetch private data; subsequent runs use the frozen snapshot. It fails on discrepancies, unlike Binance's display-only tests. Ordinary deterministic tests never require Kraken credentials.

Covered acceptance cases: official signing vector; signed offset pagination and monotonically increasing nonces; cached ledger pages with independent balance fetches; HTTP throttle retry; missing/corrupt/wrong-account offline caches; exact large/numeric quantities; fee charged once; Earn transfers/rewards; repeated/changed snapshot identity; persisted discrepancy across DB reopen; historical positions absent from current references; protected scale/type/asset/debt failures leaving no effects/observations/completed marker; print isolation across two Kraken accounts plus Nexo and Binance in one DB. Loopback mock-server tests require local socket permission in restricted environments.

Follow-up scope lives in [the master todo](../todo/master-todo.md#3-third-implement-kraken): real-account fixtures, cursor pagination, CSV recovery, remote wallet selection, retired/unsupported products, allocation metadata and debt coverage. Do not treat these as implemented.
