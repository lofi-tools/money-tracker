# Nexo provider: sources, ledger mapping and diagnostics

Implemented behavior, checked against source on 2026-10-11. See [shared provider conventions](../findings/provider-diagnostics.md), [persistence](persistence.md) and [outstanding provider work](../todo/master-todo.md#2-second-document-every-existing-provider-and-acquisition-source).

## Data source and import identity

[CSV reading](../../libs/nexo-csv/src/lib.rs) owns parsing and exact-byte SHA-256 file identity. [The adapter](../../libs/lib-app/src/adapters/nexo.rs) owns transaction semantics, conversion to domain types, position registration, and import orchestration.

The adapter accepts a configurable CSV path via [CLI configuration](cli.md). It checks a completed file hash before parsing/mapping. Register assets, effects, and the hash atomically; skip identical successfully imported bytes even at a different path. Changed bytes reuse stable `NEXO:` transaction IDs to replace effects. A failed import records no completed hash and can be retried.

The source is a user-supplied transaction-history CSV, not a Nexo API connection or independent balance snapshot. Parsed fields are `Transaction`, `Type`, `Input Currency`, `Input Amount`, `Output Currency`, `Output Amount`, `USD Equivalent`, `Details`, and `Date / Time (UTC)`. Amounts use exact `rust_decimal` parsing; time is UTC. Unknown enum values or malformed rows fail parsing with the CSV row number. `Fee` and `Fee Currency` columns are not parsed. `Details`, source type and USD equivalent are not persisted as transaction metadata; USD equivalent is used separately for implied pricing.

Production entry point: `NexoSvc::import_into_with_progress`, called by [the CLI](../../apps/cli/src/main.rs), writes `${data_dir}/portfolio.duckdb`. `--nexo-csv PATH` selects the source. The import marker is `(NEXO, SHA-256 of exact file bytes)` and transaction IDs are `NEXO:<Transaction>`. IDs identify records, not their filename or row index: a changed row under the same ID replaces effects. Duplicate source IDs therefore do not represent independently counted occurrences. Empty-effect rows still have a persisted transaction.

`import_nexo_rows` is a lower-level helper used by tests: it registers metadata before saving transactions and has no completed-file marker. Its whole operation does not have the production file import's atomicity guarantee.

## Mapping to transactions and positions

Movements use eight-decimal integer units. Source amounts are parsed exactly into bounded decimal values and converted to arbitrary-precision integer units without rounding. Unsupported range or fractional precision fails; prices remain approximate. Existing float-derived ledger results and completed import hashes are preserved during schema upgrades, rather than silently remapped. Register one aggregate `NEXO:<ticker>` asset position per ticker. The export cannot reliably distinguish individual product/wallet positions, so individual lookup is unsupported and products are not fabricated. Internal wallet transfers and term locks have no aggregate asset effect. Exchanges reduce input assets and increase output assets.

Credit mapping uses separate `NEXO-CREDIT:` liability positions, including the current USD proxy. Loan draw, mapped fees/interest, and duplicate merchant/FX draw handling must preserve signed debt without treating external counterparties as liabilities. The role migration upgrades legacy databases without reimport. Generic loan lifecycle, repayment coverage, collateral terms, and claim metadata still need fuller source-backed modeling; the existing liability fix does not imply every Nexo event type is supported.

The following table describes `convert_row`, `is_credit_domain` and `xusd_debt_accrued`. Credit classification takes precedence over the savings mapping. An input effect subtracts a nonnegative magnitude; an output adds it.

| CSV record | Persisted effect |
| --- | --- |
| Interest, Fixed Term Interest, Cashback, Exchange Cashback, Referral Bonus, Dividend, Top up Crypto, Transfer From Pro Wallet; input nonnegative | Credit `NEXO:<Output Currency>` by output amount; if output is zero, use input amount. The selected amount's sign determines input/output. |
| Deposit To Exchange | Signed input amount on `NEXO:<Input Currency>`. |
| Exchange, Exchange Deposited On, Exchange To Withdraw, Credit Card Fiatx Exchange To Withdraw | Debit absolute input amount from input currency; credit output amount in output currency. Zero output uses absolute input amount as a legacy 1:1 fallback. |
| Withdrawal, Withdraw Exchanged, Transfer To Pro Wallet | Debit absolute input amount from input currency. |
| Locking Term Deposit, Unlocking Term Deposit, Transfer Out | No aggregate effect: ownership remains with the user, and reliable wallet IDs are absent. |
| Exchange Credit with input currency USDX or xUSD | Debit `NEXO-CREDIT:xUSD` by absolute input amount: new negative liability. |
| Nexo Card Transaction Fee with input currency USDX or xUSD | Debit that same liability by absolute input amount. |
| First table row's types with negative input | Credit-domain row: debit `NEXO-CREDIT:xUSD` by output amount. The current code does not validate its currency as USDX/xUSD; this is a mapping assumption, not a generic loan-interest rule. |
| Other Exchange Credit legs, Credit Card Withdrawal Credit, Nexo Card Purchase, other credit-domain fees | No effect: duplicate draw records and merchant/FX legs must not count a second borrowing or a savings withdrawal. |

Synthetic examples: a 2 USDC savings reward produces an output of `200000000` units on `NEXO:USDC`. A USDX card draw of 100 followed by a USDX fee of 1 produces input effects of `10000000000` and `100000000` on `NEXO-CREDIT:xUSD`, for a signed balance of −101 xUSD. Duplicate merchant/draw rows do not subtract it again. Nexo does not create Binance-style external balancing legs, so per-asset double-entry conservation is not a Nexo acceptance test.

Asset IDs currently use the source ticker directly; the proposed registry/alias resolution is not implemented here. Both savings and credit positions belong to the migrated `legacy:nexo` account in the default local portfolio. Provider-wide transaction/position IDs remain in use: selecting separate real Nexo accounts through the application is future work.

## Asset sums, liabilities and reconciliation

For a cutoff, sum output units minus input units separately for positions with role `Asset` and `Liability`. `asset_units_at` and `credit_units_at` are exact; display divides each sum by its protected scale. Liabilities remain signed negative amounts. Net worth adds valued assets and signed liabilities, so it subtracts debt once. A synthetic 200 USD asset and −101 USD liability give 99 USD net worth. Counterparties never enter either sum. See [historical valuation](historical-net-worth.md) for proxy prices and missing-price behavior.

Current Nexo tests compare mappings with synthetic expectations, a CSV movement calculation, and frozen export-specific expected balances. These establish mapping consistency and regressions; they do **not** reconcile with independently obtained Nexo balances. There is no current provider balance endpoint/import, persisted independent Nexo observation, or reconciliation status that gates dashboard totals. A successful import means the file mapped and committed, not that all account history is present.

## Provider-only print and verification tests

Run from the repository root:

```sh
# Nix wrapper, also available as `nexo` in the dev shell:
nix run .#nexo
# Exact test selected by that wrapper:
cargo test -p lib-app adapters::nexo::tests::latest_amount_after_cached_export -- --exact --ignored --nocapture
# Synthetic/import regression tests; ignored private print test stays skipped:
cargo test -p lib-app adapters::nexo::tests::
cargo test -p nexo-csv
```

The ignored print test uses `.cache/imports/nexo_transactions_05-10-2026_10-36-53.csv` and `.cache/test-data/test-nexo_05-10-2026.duckdb`. It reads the CSV even when the completed hash permits DB reuse, takes the greatest CSV timestamp as its cutoff, and prints assets and credit separately. It does **not** use the ordinary CLI's `--nexo-csv` override. Its printed values use approximate floating-point formatting to eight places, while its exact-unit assertions and hardcoded frozen-export expectations are fixture-specific. Preserved historical float-derived effects may differ from a fresh exact remap; this test is not a generic print command for arbitrary exports.

Meaningful regression coverage includes `nexo_card_debt_reduces_net_worth_without_including_binance_counterparties`, `internal_transfer_does_not_change_amount`, `reimport_replaces_effects`, `file_hash_skips_identical_exports_and_imports_changed_bytes`, `invalid_export_does_not_store_its_hash`, `imported_file_hash_survives_database_reopen`, and `exact_import_retains_large_decimal_units_and_rejects_rounding`. These exercise debt roles, import identity, failure/retry and precision without requiring the private export.

## Valuation limitations

An incomplete export can produce negative aggregate assets if funding or transfers are omitted. Nexo USD/fiat proxies and export-implied NEXO prices are valuation assumptions documented in [historical valuation](historical-net-worth.md). No wallet/subscription detail or yield rate should be inferred from aggregate movements alone.

The general fee columns, earnings categories, cost basis, rate/renewal/term metadata, independent reconciliation, and full repayment/collateral lifecycle are not implemented. The zero-output exchange fallback assumes equal quantities across currencies and needs source-specific review rather than being treated as a universal conversion rule. Completed hashes prevent automatic remapping after mapper changes; startup schema migrations preserve existing effects and hashes.

Managed file upload/storage, a setup command, environment content input, and automatic discovery were proposed but are not implemented. They are retained as [future design choices](../todo/implementation-gaps.md), rather than requirements describing current behavior.
