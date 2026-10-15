# Nexo inferred current allocations

Implemented for review on 14 October 2026. This extends the [transaction investigation](../findings/nexo-transaction-inference.md) and [Nexo import contract](nexo.md). It does not replace aggregate ownership with reconstructed ledger positions.

## Evidence and partitions

The importer retains the exact validated CSV bytes in the existing immutable account source snapshot store after a completed import. An identical completed import can backfill this source and subscription metadata without replaying financial effects. Metadata backfill is a separate retryable stage after ledger completion. Source hash conflicts and provider/account mismatches fail explicitly.

The query uses the latest export no later than its requested cutoff. Exact lock/unlock movements are grouped by asset and principal. Unique surviving openings become fixed allocations; repeated principal remains an unresolved cohort until every ambiguous member is released. An unmatched release fails inference rather than inventing an opening. Partitioning rejects locked amounts exceeding the aggregate holding. The user has confirmed that principal outside fixed terms in this account is flexible savings. The residual partition is therefore labelled **Flexible savings**. This account confirmation supersedes the earlier savings/collateral uncertainty; it is not inferred from transfers alone. Inference failures still leave the aggregate unresolved.

Inferred fixed subscriptions have empty ledger-position lists. They provide stable metadata/rate scopes, while the original aggregate remains the sole ownership record. Expanded holding rows show fixed partitions and the residual. Their values sum to the original holding when its price is known; they are never added again to portfolio totals. The redundant quantity list above the allocation table is removed.

“Current” means still open at the source cutoff, not independently verified today. For the selected export this is 5 October 2026. The supplied export yields 20 represented terms: 16 unique openings and two unresolved two-member cohorts. Start dates are shown for unique openings. A product’s annotated `term_months` supplies an **expected maturity** by calendar-month addition; it is not a confirmed contractual due date. A saved subscription maturity takes precedence. CSV backfill preserves existing maturity, renewal and closure annotations. Renewal/live status is not established by the export. For the unique ETH opening on 3 October 2026, the one-month annotation gives expected maturity 3 November 2026. The equal-principal cohort’s individual start/due dates remain unresolved.

## Rates and earnings

A unique current opening can inherit an **observed APY estimate** from a uniquely matched preceding term of the same asset closing exactly at renewal. Principal must match exactly, or equal previous principal plus its same-asset reward. The existing reward matcher uses an explicit 300-second window because reward records precede unlock records by a few minutes. All candidates compete; there is no nearest-event tiebreaker. Ambiguous or different-currency rewards remain unresolved. Approximate annualization is historical evidence, not a contractual current rate and is not applied as confirmed annual earnings.

Expected rates use dated persistent metadata, with individual subscription overrides before a qualified cohort product and then the asset's fixed product. Missing rates remain unknown. Valid expected rates supply annual estimates on exact partition value; residual yields use an account-qualified flexible product rate when present, otherwise remain unknown, so the aggregate can display a partial known subtotal. Existing aggregate/manual override precedence is preserved. Price and rate provenance accompanies the estimate.

User-reported account rates recorded for this review are distinct from the [bundled public catalogue](nexo-rate-catalogue.md):

| Asset | Flexible reference | Fixed expected rate |
| --- | --- | --- |
| ETH | 5.25% | 1 month, 6.25% |
| NEXO | 3% | 12 months, 9% |
| NEAR | 7%, paid in NEXO | 1 month, 8%, paid in NEXO |
| BNB | 6%, paid in NEXO | 1 month, 7%, paid in NEXO |
| GBPX | 6.5%, paid in NEXO | 12 months: 4,963 and 4,200 at 10.5%; 3,600 at 11.5% |
| EURX | 6% base, +2 points when paid in NEXO | Old 12-month pots at 11% and 10% |

The two equal-principal EURX pots permit a **10.5% aggregate expected rate**. Their individual identities/rate assignments remain unresolved; these grandfathered rates are not offered defaults. DOT fixed rates remain unknown. GBPX rates are attached to the uniquely identified principal-specific subscriptions; they do not create a generic rate for all GBPX terms. The promotional/tier explanation in the supplied text is unverified. Confirmed flexible rates are now recorded in qualified, empty-position subscriptions and apply only to the matching account/asset residual. EURX uses the confirmed 6% base; the conditional +2 points are not applied without payout qualification. Conditional advertised maxima and pasted third-party assertions are not verified official rates or account defaults. No daily-yield-based disambiguation is implemented.

## Related portfolio behavior

Unpriced asset holdings join **Others**, including tokens with known metadata but unknown value. Their audit records and partial-total warnings remain visible.

Plain EVM native currency in a wallet has a zero-yield fallback with provenance. It does not apply to ERC-20 receipts or subscriptions, and explicit manual rates retain precedence. Users can annotate EVM account wallet software through the details editor; the local annotation is logged and undoable, preserves provider identity, and appears in account labels. The user-provided wallet's chain accounts are annotated MetaMask.

## Validation

Synthetic regression checks cover ambiguous releases, exact reinvested renewals, missing openings, unchanged aggregate ownership across reimports, expected-rate partial earnings, native-wallet zero versus unknown token-product yield, and wallet-software edit/Undo. The frozen browser export is regenerated from a copy of the selected CLI database. No principal or financial transaction is created by these annotations.

Review validation also caught and corrected the desktop `RateScope` contract: Rust serializes account/asset and provider/asset scopes under the tagged `id` object. Change history now reads that shape, so entering Settings after manual edits does not throw. The browser manual-edit/Undo regression uses the real Rust fixture and checks for runtime errors.

Completed checks for this review: 79 application unit tests, 18 portfolio tests, 2 refresh integration tests, 9 core metadata tests, 5 CSV tests and 5 native desktop portfolio tests passed; private/live tests were excluded. Desktop TypeScript/build and all 6 browser tests passed. The selected database comparison preserved account metadata/jurisdictions and five unrelated provider accounts, with zero Nexo balance differences against a fresh source map.

## Presentation and account settings follow-up

The [provider grouping contract](provider-layouts.md) gives Nexo an asset-view **Flexible savings** row and **Fixed terms** group, without a redundant ETH holding heading. Group summaries do not create ownership. A unique pot warning distinguishes unconfirmed maturity/renewal from an unresolved equal-principal identity cohort.

`cargo run -p lib-app --example nexo-savings-settings -- EXISTING_DB ASSET FLEX_PERCENT FIXED_MONTHS [REWARD_ASSET]` records account-specific flexible rates and fixed product duration. It requires an existing `legacy:nexo` account and fixed product, creates no financial positions, and preserves fixed rates and individual pot overrides. This review recorded ETH 5.25%/1 month, NEXO 3%/12 months, NEAR 7%/1 month paid in NEXO, BNB 6%/1 month paid in NEXO, GBPX 6.5%/12 months paid in NEXO, and EURX 6%/12 months. Before/after saved balance reports match exactly apart from their report timestamp.

### Proposed account capture, not implemented

Actual due dates, renewal settings and applicable rates should be collected from the account’s Active terms/Savings breakdown. Nexo’s [fixed-term guide](https://support.nexo.com/article/how-to-earn-interest-with-fixed-term-yield-us) describes the account interface; its US eligibility/rates are not applied to this UK account. No documented retail fixed-term API was established in this investigation; the payment gateway API does not establish savings access.

The revised [Chromium capture proposal](nexo-browser-capture-proposal.md) uses a floating **Export Nexo data** button on signed-in Nexo pages. One click automatically navigates read-only account views and sends JSON through a local native bridge for tracker DB ingestion. Site-specific access is granted during setup. Capture includes term principal, current rate/basis, payout, dates and distinct renewal settings; missing/rounded fields and ambiguous matches stay unresolved. No credentials or Nexo setting changes are collected/performed. A [Brave development implementation](nexo-browser-capture.md) was approved on 15 October 2026, with conservative partial coverage and unresolved matches; live installed-Brave verification and remaining acceptance work are owned by the existing master-todo capture task.
