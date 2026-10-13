# Nexo transaction inference investigation

Investigated 13 October 2026. This records the initial evidence review and a subsequent **implemented read-only candidate matcher**. Persistence, UI integration and current-rate inference remain proposed. The ordered follow-up is in [master todo section 0](../todo/master-todo.md#immediate-provider-wiring-fixes--ordered-proposed). Existing behavior is documented in [the Nexo guide](../spec/nexo.md).

## Result

| User question | What the available transactions establish | What stays unknown |
| --- | --- | --- |
| Which funds were moved into locked pots? | Lock/unlock events can reconstruct net term-wallet principal per asset at the export cutoff, subject to complete history and movement validation. | Independent current balances, unreported events and any later activity. |
| Which funds earn the flexible/default rate? | Total owned quantity minus reconstructed locked quantity gives a residual outside the term wallet, if both histories are usable. | Whether that residual is eligible savings, collateral or another bucket; current tier, payout choice, rate and conditions. Do not assign a default yield to all residual funds. |
| Can we recover individual subscriptions? | A lock records asset, quantity and event time. It can seed a candidate subscription with a stable local reference to that event. | A provider term ID and unambiguous closure/renewal linkage. Multiple equal-sized concurrent deposits defeat amount-only matching. |
| Can we recover maturity dates? | A safely linked unlock establishes an observed historical end event. | Scheduled maturity for an open term, term length, cancellation semantics and auto-renew setting. Observed end time is not proof of contractual maturity. |
| Can we infer APY from rewards? | Some reward descriptions identify the earning asset or a base-asset quantity, allowing partial observed historical yield calculations when principal and earning periods are known. | Current contractual APY, compounding policy, missing attribution and eligibility. Rewards alone do not identify these. |

The transaction export is useful for allocation and partial historical earning evidence. A reviewed rate/conditions catalogue plus account/subscription qualification evidence is still needed for current earnings; that is the next roadmap task. No default Nexo APY was invented in this investigation.

## Sources and present wiring

- [Parser](../../libs/nexo-csv/src/lib.rs): retains transaction ID, event type, input/output currency and exact decimal amounts, USD equivalent, `Details`, and UTC timestamp. There are no parsed term-ID, contractual-rate, term-length, maturity or auto-renew columns.
- [Adapter](../../libs/lib-app/src/adapters/nexo.rs): `LockTermDeposit` and `UnlockTermDeposit` create no aggregate effects. This preserves ownership, but loses the asset/quantity/type of those movements in the persisted ledger. Their transaction identity/time remains; source `Details` and reward classifications are not persisted. `CollectTxnData` supplies no products or positions, and the import registers aggregate asset positions rather than product/subscription rates.
- [Metadata](../../libs/lib-core/src/ui_metadata.rs): subscription lock/maturity/renewal fields and dated rate evidence exist. They are not populated from these Nexo rows. A subscription's positions represent ledger buckets; attaching the same aggregate position to several inferred subscriptions would duplicate principal.
- [Current queries](../spec/portfolio-queries.md): current earnings require applicable rate evidence. Missing investment rates can contribute assumed-zero aggregate estimates, but individual unknown rates remain unknown; this is not evidence of a real zero Nexo rate.

The cached CSV was inspected read-only. It contains 10,409 events through 5 October 2026: 753 locks, 733 unlocks, 737 fixed-term rewards and 7,467 `Interest` events. No balances, transaction IDs or private CSV rows are copied into this document. These counts describe this export, not a universal Nexo schema guarantee.

## Lock and term evidence

Every observed lock/unlock has the same input/output currency and equal nonzero amount magnitude. Locks have negative input, unlocks positive input. Their descriptions are generic Savings Wallet ↔ Term Wallet transfers, without term references or maturity information. Chronological exact-decimal summation of lock principal minus unlock principal never went negative in this export, and leaves positive net term principal in seven assets at its cutoff. This is a consistency check, **not independent reconciliation** or proof of current balances.

A diagnostic amount-matching heuristic maintained active lock candidates per asset and exact amount. It encountered 202 unlocks with multiple candidates. Using FIFO solely to continue that diagnostic produced 531 locally single-candidate unlocks; these are not 531 proven term matches, since previous ambiguous choices can affect later candidate sets. Historical elapsed times near a month do not establish a fixed 30-day contractual term or future maturity. There are also simultaneous lock/unlock timestamps, so ties must be treated as a group rather than relying on CSV row order.

Proposed rules:

- Validate signs, currencies, magnitudes, duplicate IDs, cutoff and history coverage before deriving allocations. Incomplete history, unmatched unlocks, inconsistent movements or derived amounts exceeding owned quantity must produce explicit unresolved evidence, never clamp amounts or add balancing entries.
- Preserve exact net locked principal per asset independently of candidate term matching. Label its export cutoff and evidence status. Preserve the remaining quantity as an unclassified residual until savings/collateral attribution is supported.
- A candidate lock can expose its observed opening event and amount. Keep ambiguous closure/renewal relationships unresolved; do not choose FIFO or a nearest timestamp as financial truth. A closed term needs sufficiently strong linkage or manual confirmation.
- Leave an open term's maturity and auto-renew nullable. Even a confidently linked historical unlock establishes only an observed closure; extrapolating a duration is a separately labelled estimate and cannot drive withdrawal guarantees.

## Reward and rate evidence

All 737 fixed-term rewards in this export have NEXO as both input and output currency. The reward currency therefore cannot identify the deposited asset. Of these, 417 use a generic term-interest description; others name an earning asset, with four fixed-term adjustment records. Of the fixed rewards, 576 share a timestamp with an unlock, but many timestamps have several unlock candidates. Timestamp coincidence alone cannot allocate the reward to a subscription or establish its earning interval.

There are 5,036 `Interest` descriptions matching a decimal quantity plus asset ticker, alongside generic interest and asset-named descriptions. That text is candidate base-denominated reward evidence, not a principal balance or verified conversion quote. Its meaning and precision need validation against export examples before using it for yields. There are 515 negative-input `Interest` records currently classified into the credit domain; borrowing costs must stay separate from investment rewards. Generic adjustments and unattributed rewards remain separate rather than being spread across eligible-looking assets.

Proposed calculation boundary:

1. Retain reward asset/amount, candidate earning asset, event timestamp and source detail independently. Preserve corrections, adjustments and debt classification without adding the reward twice to ownership.
2. Define an earning interval from supported source evidence; payout time alone is insufficient for fixed terms. Reconstruct changing eligible principal over that interval, excluding locked or collateral quantities where applicable. Unsupported principal coverage means no rate.
3. For known base-denominated rewards, a simple observed annualized rate can be `reward / time-weighted principal × year length / observed duration`. This is an observed APR-style statistic with an explicit convention. An equivalent APY needs an explicit reinvestment/compounding convention; it must not be relabelled contractual APY. Avoid annualizing an arbitrary interval between reward timestamps, especially for lump-sum term payouts.
4. Different-currency rewards require explicit conversion evidence at receipt for immediate historical yield. Today's conversion and actual disposals belong to the separate historical effective-return analysis. The CSV's USD equivalent and implied NEXO prices are approximate valuation inputs, not independent exact base-asset quotes.
5. Missing, ambiguous or adjustment-only evidence produces partial/unknown historical results. Neither sparse rewards nor this export's old cutoff justifies populating today's current `RateEvidence` with a guessed APY.

## Proposed implementation sequence

First persist versioned source event metadata keyed by stable account/provider/native transaction identity, then derive exact aggregate locked/residual allocations with provenance and unresolved term candidates. Add reward attribution separately. Keep the existing aggregate ownership ledger authoritative and unchanged: derived allocations must partition its quantity, not become additional assets counted alongside it. An eventual move to real ledger buckets needs a deliberate migration and tests; the current subscription model cannot split one aggregate position merely by assigning it to multiple subscriptions.

Keep opening, observed closure and scheduled maturity distinct. Do not force observed historical rates into the current APR/APY evidence type, which has no historical-estimate classification. Define that evidence/query contract before adding historical yield to the app. Preserve manual/pinned overrides and request review when new finer evidence overlaps an aggregate override.

A metadata backfill must replay the original CSV even if its ownership import hash is complete, under a separate versioned completion marker and atomic write. Do not delete old hashes or remap preserved float-derived legacy balances just to add metadata. Source files unavailable locally remain an explicit prerequisite. Normal reimports must preserve exact amounts, stable IDs and idempotence.

Implementation acceptance should cover overlapping equal-sized terms, ties, missing opening events, inconsistent movement amounts, cross-asset NEXO rewards, generic/adjustment/debt interest, incomplete eligible principal, pinned overrides and unchanged aggregate ownership across metadata replay. The initial investigation did not change runtime or migrations. The subsequent matcher below adds a read-only diagnostic without modifying imports, migrations or the app.

## Validation

- `cargo test -p nexo-csv`: five parser/hash/error tests passed; one private file test stayed ignored.
- `cargo test -p lib-app adapters::nexo::tests::`: eleven tests passed; one private print test stayed ignored. Coverage includes unchanged ownership across locks, exact amount rejection rather than rounding, reimport replacement, file-hash idempotence/retry/reopen, ledger sums and debt-aware valuation.
- `cargo test -p lib-app --test portfolio`: all thirteen tests passed, covering current unknown/partial rates, weighted subscriptions, liabilities, exact amounts, isolation and exclusions.
- Read-only CSV diagnostics used Python's `csv`, `Decimal`, UTC timestamp ordering and counters. They checked movement conservation and nonnegative net locked principal, counted competing exact-amount candidates, compared reward/unlock timestamps and inspected description families. They did not import data, contact Nexo, write the app database or persist heuristic term matches.

Existing tests establish current behavior, not acceptance of the proposed inference engine. Independent Nexo reconciliation remains unavailable.

## Implemented read-only matching follow-up (awaiting commit approval)

Source: [term inference](../../libs/lib-app/src/adapters/nexo/term_inference.rs), exported through the Nexo adapter, and [aggregate diagnostic](../../libs/lib-app/examples/nexo-term-inference.rs). The function consumes one account's parsed export without DB access. It returns candidate locks, inferred closes/rewards, ambiguity flags and unresolved event IDs; it is not part of automatic import or the desktop query path.

Matching is chronological and uses exact asset/principal keys. Unlocks at one timestamp are considered together, before new locks at that timestamp, so a same-time renewal is not a competing earlier opening. More than one active equal-principal lock taints the whole cohort: no FIFO identity is assigned, and a later smaller candidate count cannot resolve that earlier ambiguity. A cohort resets only after its represented releases are complete. Unmatched unlocks stay unresolved. Duplicate source IDs, invalid signs/currencies/magnitudes and negative windows fail explicitly. An open/unresolved lock is not proof of a current active subscription; completeness and later activity still require review.

Reward matching uses a caller-supplied symmetric time window, with zero seconds as the diagnostic default. Only explicit descriptions naming the earning asset constrain that asset; the payout currency alone does not. All unlocks within the window compete, including unresolved ones. A reward links only when it has one candidate unlock, that unlock has one competing usable reward, and its principal linkage is resolved. Negative-input fixed interest and adjustments remain unresolved. There is no nearest-event, amount-tolerance or rate-based tiebreaker. Links remain historical inferences, not provider-verified identities.

Observed elapsed duration is opening-to-unlocking, not scheduled maturity. With a linked same-asset reward, the matcher retains an exact rational period return and an approximate equivalent APY: `expm1(log1p(reward/principal) × 365 × 86400 / elapsed_seconds)`. This assumes that payout covers the whole inferred term and that the same return is repeated and reinvested throughout a fixed 365-day year. Missing earlier payouts or adjustments can invalidate that assumption. It does not establish current contractual APY. Fractional-power annualization is explicitly approximate; original amounts and period return remain exact. Cross-asset rewards remain linked but have unknown return/APY until conversion evidence is supplied. No approximate result is written as `RateEvidence`.

Read-only reproduction (prints counts only, not IDs, principal or returns):

```sh
cargo run -p lib-app --example nexo-term-inference -- PATH_TO_EXPORT.csv
# Explicit nearby window; zero/default demands the same timestamp:
cargo run -p lib-app --example nexo-term-inference -- PATH_TO_EXPORT.csv 60
cargo test -p lib-app adapters::nexo::
```

For the inspected export, both 0-second and 60-second windows gave 753 opening candidates, 617 inferred closes, 120 ambiguous opening candidates and 116 unresolved unlocks. Of 737 fixed rewards, 337 linked uniquely and 400 remained unresolved. Sixteen linked same-asset payouts allowed exact period returns and finite conditional annual equivalents; the others need cross-asset conversion evidence. These counts are not independent reconciliation or proof of the rate assumptions. They supersede the earlier FIFO diagnostic counts for this matcher, while preserving that diagnostic's rationale.

Eight synthetic matcher tests cover unique exact returns, overlapping equal amounts, huge principals differing in the eighth decimal, simultaneous renewals, competing rewards and unmatched unlocks, explicit nearby windows, earning-asset descriptions, cross-asset/adjustment/debt ambiguity, generic NEXO payouts, deterministic input ordering and malformed/duplicate source events. Validation passed: all 19 Nexo adapter/matcher tests (one private print test ignored) and all 13 portfolio-query tests. Formatting and relative documentation links also passed. No provider data was modified.

## Future expected-yield matching

A reviewed historical rate/conditions catalogue could score candidate links using expected earnings. It must identify rates effective during the term, qualified tier, principal/caps, payout choice, eligibility and payout schedule. Current advertised APY must not be used to explain old terms. Daily accrual is not necessarily a daily CSV payout: for a term-end payment compare the expected accumulated reward for its earning interval, with explicit compounding and conversion assumptions. NEXO payouts need supported conversion evidence and a documented rounding tolerance.

Expected amount is supporting evidence, not authority to invent a subscription identity. Equal principals/rates, unknown qualifications, reward adjustments or multiple fits remain unresolved. This rate matcher is not implemented and depends on the next catalogue task; do not silently turn an advertised APY into an observed fact or use a fitted APY to circularly prove its own reward attribution.
