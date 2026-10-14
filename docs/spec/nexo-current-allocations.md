# Nexo inferred current allocations

Implemented for review on 14 October 2026. This extends the [transaction investigation](../findings/nexo-transaction-inference.md) and [Nexo import contract](nexo.md). It does not replace aggregate ownership with reconstructed ledger positions.

## Evidence and partitions

The importer retains the exact validated CSV bytes in the existing immutable account source snapshot store after a completed import. An identical completed import can backfill this source and subscription metadata without replaying financial effects. Metadata backfill is a separate retryable stage after ledger completion. Source hash conflicts and provider/account mismatches fail explicitly.

The query uses the latest export no later than its requested cutoff. Exact lock/unlock movements are grouped by asset and principal. Unique surviving openings become fixed allocations; repeated principal remains an unresolved cohort until every ambiguous member is released. An unmatched release fails inference rather than inventing an opening. Partitioning rejects locked amounts exceeding the aggregate holding. Unallocated principal remains savings/collateral **unknown**, since transfers do not establish its complete eligibility lifecycle.

Inferred fixed subscriptions have empty ledger-position lists. They provide stable metadata/rate scopes, while the original aggregate remains the sole ownership record. Expanded holding rows show fixed partitions and the residual. Their values sum to the original holding when its price is known; they are never added again to portfolio totals. The redundant quantity list above the allocation table is removed.

“Current” means still open at the source cutoff, not independently verified today. For the selected export this is 5 October 2026. The supplied export yields 20 represented terms: 16 unique openings and two unresolved two-member cohorts. Scheduled maturity and auto-renew remain unknown, even when the user supplies a product duration.
