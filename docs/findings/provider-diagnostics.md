# Provider documentation and diagnostic conventions

Source review on 2026-10-11. Provider guides: [Nexo](../spec/nexo.md), [Binance](../spec/binance.md), [Kraken](../spec/kraken.md). These describe implemented behavior; [the master todo](../todo/master-todo.md#2-second-document-every-existing-provider-and-acquisition-source) owns remaining diagnostic work and price-source guides.

## Guide structure for current and future connectors

Use the same topics for Kraken and later integrations: supported account/product families; acquisition endpoints or file fields; authentication and retention; timezone/cutoff; exact amount/scale rules; source identity and import markers; record → movement → DB effect mappings; position roles; asset/liability sums; independent references and reconciliation; cache layout; provider-only print/strict test commands; meaningful synthetic tests and limitations. State separately what a provider exposes and what our code supports. Price feeds belong in source guides and never imply account coverage.

## Shared DB arithmetic

[Core transactions](../../libs/lib-core/src/types.rs) contain timestamped nonnegative input/output magnitudes. [Storage](../../libs/lib-core/src/storage.rs) persists signed effects: inputs negative, outputs positive. `Units` is arbitrary precision, stored as integer text; aggregate in Rust rather than applying an ordinary SQL numeric sum to the text column. For scale `s`, display quantity is units / 10^s. `decimal_string(s)` preserves exact digits; converting for price multiplication or approximate display is a separate boundary.

Position roles are authoritative:

| Role | Sum and valuation |
| --- | --- |
| Asset | Included in owned asset totals. An unexpectedly negative balance is a coverage/mapping warning, not automatically a loan. |
| Liability | Included separately as a signed balance; current Nexo debt is negative. Net worth adds signed debt to assets, subtracting it once. |
| ExternalCounterparty | Excluded from holdings, liabilities and net worth. Binance fee/market/reward legs are not money the user owes. |

`asset_units_at` / `credit_units_at` group exact amounts by asset across the database at the inclusive transaction cutoff. `portfolio_units_at` adds portfolio selection; `portfolio_balances_at` and `account_balances_at` return scoped asset/liability positions. Prices must cover nonzero assets **and** liabilities; fiat/debt proxies and fallback prices are documented in [historical net worth](../spec/historical-net-worth.md). Accounting role does not imply that a provider's full real-world debt or holdings are covered.

Nexo/Binance provider-only print tests achieve isolation through **separate fixture databases**, not provider filters in their queries. Adding other providers to those DBs would include their owned totals. Kraken uses account-scoped imports/queries and exact print output, with a mixed-database isolation test covering Nexo, Binance and two Kraken accounts. Extending that workflow to legacy adapters remains open work.

## What a passing test establishes

| Check | Evidence and limit |
| --- | --- |
| Parsing/mapping/import test | Supported source records produce expected effects and safe reimports. Does not prove the source is complete. |
| Nexo CSV/frozen expected totals | Consistency against the export or historical fixture expectation. No independent Nexo balance source. |
| Binance structural conservation | Each asset's signed effects sum to zero including counterparties. Does not prove correct account classification or complete history. |
| Binance strict balance reconciliation | Owned ledger positions match separately fetched API reference quantities, including closed/zero positions. Does not by itself prove historical earnings, fees or cost basis are correct. |
| Kraken strict account reconciliation | Selected account's exact ledger quantities match persisted independently fetched BalanceEx observations. Cutoff/fetch timing and unsupported account/product families remain explicit coverage limits. |
| Binance print tests | Show balances; statement display reports differences without failing, API display directs users to the strict check. Print success is not reconciliation success. |

No implemented workflow yet enforces the interview requirement to withhold unreconciled historical/yield totals across the application. Source event semantics and independent observations need a fuller model before received/net/expected earnings can be reported reliably. Asset balances alone are not an earnings calculation.

## Repeatable verification

Keep private exports, keys, cached API responses and account balances out of committed documentation/fixtures. Commands in provider guides assume the repository root and existing Rust/Nix build environment. Synthetic fixtures should test economically distinct cases: internal transfers preserving owned sums, fees in a third currency, liabilities reducing net worth, independent mismatches staying visible, exact decimals and failed imports leaving no completed marker. Preserve source files and use disposable DBs for remapping experiments; do not refresh frozen observations independently from their matching history/cutoff.

Provider guides were checked against functions, tests and Nix wrapper definitions, not a fresh private live fetch. Kraken additionally has synthetic mapping/storage and loopback HTTP acquisition tests. Existing historical reconciliation results remain in [the Binance findings](binance-history.md); they must not be reinterpreted as a new verification run.
