# Proposed data model evolution

Status: **proposed; no schema, adapter, or calculation changes implemented by this review**. Reviewed against source on 11 October 2026. Delivery status belongs to [the master todo list](../todo/master-todo.md#1-first-data-model-review-and-minimum-supporting-changes). This document is the design deliverable for that review, not approval to implement every phase.

## Recommendation

Keep the signed movement ledger and explicit asset/liability/external roles. Add account-scoped identity, exact amounts, economic event meaning, source evidence, and reconciliation boundaries first. Persist actual held positions separately from public products and rate observations. Earnings analysis needs these foundations before opportunity optimization or withdrawal planning can be reliable.

Implement this incrementally. Kraken should use the same ingestion contract as Nexo/Binance after the minimum foundation changes; it need not wait for shared accounts, the risk service, or a complete DeFi ontology. Do not translate all Airtable tables into database tables. The [interview](../findings/user-needs.md) and [product questions](product-goals.md) determine requirements; [Airtable findings](../findings/airtable-model.md) identify evidence and candidate holdings.

## Current source and gaps

| Area | Implemented evidence | Gap / consequence |
| --- | --- | --- |
| Domain identity | String IDs in [types.rs](../../libs/lib-core/src/types.rs); `Asset` has ticker/symbol/scale. | Tickers do not identify chain contracts; no durable account/portfolio hierarchy. `ExternalAssetId` equality ignores issuer while its hash includes issuer. |
| Positions/products | `UserPosition` has product and optional dates; `Product` has one asset and `apy: f64`; `CollectTxnData` collects them. | These relationships are transient. Binance constructs per-position products with `apy: 0.0`, which cannot mean a verified zero rate. |
| Ledger | [Store](../../libs/lib-core/src/storage.rs) persists transaction ID/time and ordered signed effects; stable-ID imports replace effects atomically. | No persisted event kind, source record, earning-position attribution, fee purpose, or connection namespace. Core collection also lacks transaction IDs. |
| Amounts | Effects are `u64` in domain, signed `BIGINT` in DB; balance result type is `i128`. | Writes reject amounts above `i64::MAX`; exact balance queries cast sums to `BIGINT` and read `i64` before widening. Several individually valid effects can overflow a query. |
| Precision | `asset_scales` registers an asset scale. Binance parses decimal strings; [Nexo parser](../../libs/nexo-csv/src/lib.rs) uses floats and its adapter rounds to eight places. | Scale upserts can reinterpret old quantities; eight places cannot retain all EVM units. Decimal source evidence is lost through float conversion. |
| Economic roles | Asset, signed liability, and external counterparty roles persist. | Keep this contract. Collateral ownership, earning allocation, locked terms, and underlying exposure need separate relationships, not extra liabilities. |
| Observations/imports | `balance_observations` stores source/hash/time/position/string amount; `imported_files` stores source/hash/completion. | No observation scope/completeness, connection, reconciliation result, or account-history coverage. Price coverage is not transaction coverage. |
| Valuation | [history.rs](../../libs/lib-core/src/history.rs), prices and coverage tables, [historical valuation](historical-net-worth.md). | One floating-point value per pair/time loses competing source evidence and quality/provenance. Current total queries do not enforce future reconciliation/exclusion policy. |
| Migrations | [migrations.rs](../../libs/lib-core/src/migrations.rs): all three steps rerun in order on every open, each in a transaction. | Extend this existing mechanism. Existing insert statements often depend on whole-table column order and must be changed when expanding tables. |

`Db`/`AllProducts` in storage are in-memory collections, not additional persistent metadata tables. `PositionHistory` contains position descriptions, not balance history. Nexo currently aggregates each asset into `NEXO:<ticker>` and credit into `NEXO-CREDIT:<ticker>`; it cannot establish individual fixed terms from those aggregate balances alone. Binance distinguishes account buckets such as spot/funding/Earn in position IDs, but those are not separate persisted connection/account entities.

## Feature-to-model mapping

| User question / feature | Required records and query boundary | Phase |
| --- | --- | --- |
| What do I have across providers/wallets? What is my net worth? | Portfolio → account → balance position; canonical asset; signed role; ledger and independent observations; reporting currency. | A |
| Which balances/history can I trust? | Observation scope and cutoff; endpoint/account coverage; reconciliation runs/issues bound to ledger revision. Current verified snapshot and historical replay are separate outputs. | A |
| What should be hidden/excluded? | Portfolio-scoped tracking policy for positions/assets with explicit historical application. Filter every totals/earnings/exposure query. | A |
| How much interest/staking/cashback/bonus have I received, after costs? | Classified events, reward receipts, costs, earning-position links, source revisions; receipt/disposal valuation. | A → B |
| What are realized/unrealized price changes? | Acquisition lots, disposals and selected attribution rule; fee/proceeds evidence and price provenance. | B |
| How much am I earning per year now? | Current held earning position/balance, verified applicable rate, reward denomination, capital basis, tiers/caps and historical reward-relative-value assumption. | B |
| Did holding a different reward coin reduce effective yield? | Dated principal flows, receipts, earning/base asset, reward/base quotes at receipt and now; hypothetical retention distinct from actual disposals. | B |
| Did my strategy beat holding the same assets? | Portfolio-boundary contributions/withdrawals, transfer links, starting holdings, price history, fee policy and versioned benchmark parameters. | B |
| What underlies this DeFi position? | Receipt/share position with dated underlying conversions and exposure edges; one selected valuation route. | C |
| Could the same assets earn more elsewhere? | Public products/offers, dated rates/fees/terms/qualification evidence; private eligibility inputs and scenario allocations/costs. | D |
| What risks and concentration do I have? | Common reviewed assessments, provider/protocol/chain exposure, methodology versions, paid dependency edges; private tolerance/limits. | D |
| When is 50%, 100%, or an amount withdrawable? | Held term instances, renewal policy, unlock/unbonding rules, debt/collateral constraints, fixed valuation snapshot and scenario. | E |
| What was lost/recovered from a failed platform? | Dated valuation override with reason; claim evidence and linked distributions, preserving quantity and original movements. | C |
| What should I be notified about? | Opt-in rules, deadlines, comparison thresholds and deduplicated notification state; no movement effects for notifications. | E |
| Can I manage/invite family or clients? | Stable portfolio identity now; later principals, access grants, invitation/handoff manifest and source-data identity across clones. | A identity; F sharing |
| Can it sync or support tax/spending/TradFi later? | Provenance and stable IDs now; later sync revisions/tombstones, tax-specific accounting and instrument/debt schedules. No speculative tax fields on every crypto event. | F |
| Can I use free local tracking and optional paid APIs? | Local personal ledger/settings; separately versioned public catalogue and entitled paid assessments. Credential references separate from financial records. | A boundary; D/F services |
| Desktop/web/GPUI, docs, CI, open source? | Stable core/query DTOs and versions. These delivery concerns do not require new financial entities. | Throughout |

## A. Minimum foundation before earnings and Kraken

### Ownership, accounts and identities

Recommended logical entities (names provisional, not SQL DDL):

| Entity | Minimum fields and relationships |
| --- | --- |
| `Portfolio` | Stable opaque ID, label, reporting currency (USD default). Represents a tracked economic perimeter, not the logged-in user. |
| `Provider` | Stable catalogue ID, display name, kind; separate from credentialed instances. Price issuers may be registered without implying account integration. |
| `Connection` | ID, provider ID, label, acquisition configuration, credential-reference ID, sync policy/status. No API secrets in this table or copied into events. |
| `Account` | ID, connection ID where applicable, source account reference, account kind, optional chain/address, label. A connection may expose several provider subaccounts. Manual accounts are supported without fabricated API access. |
| `PortfolioAccount` | Portfolio/account membership, initially exactly one economic portfolio per account. Future permissions and fractional ownership are separate extensions. |
| `Asset` | Opaque ID, display metadata, immutable ledger scale, kind; optional fiat code or chain-qualified native/token locator. |
| `SourceAssetMapping` | Issuer/namespace/external ID → asset ID; source evidence and validity dates. Unknown IDs remain distinct until resolved. |
| `BalancePosition` | Stable ID, account ID for user positions, asset ID, economic role, optional held-investment ID, source reference, lifecycle dates. External counterparties have source context but no owned-account membership. |
| `TrackingPolicy` | Portfolio, position or asset scope, included/excluded state, reason, revision. Spam hiding invokes exclusion, not merely presentation hiding. |

A balance position is one asset-denominated ledger bucket. A held investment groups those buckets around the product actually earning returns. A subscription to an available offer is not interchangeable with an account balance. This avoids making every Binance spot balance a verified yield product.

Generate durable IDs independent of labels and credentials. Scope provider record keys by connection/account and endpoint namespace, not just provider name or ticker. For chain events use chain + transaction hash + event/instruction index + affected account; transaction hash alone cannot identify every effect. Fix `ExternalAssetId` equality and hashing to use the same issuer/namespace/id tuple.

Custodial tickers may map to a canonical economic asset. On-chain representations retain chain/contract identity; wrapped, bridged, migrated and receipt tokens are distinct assets with explicit relationships. Similar tickers or price correlations do not justify merging balances. An economic exposure grouping can group representations without erasing provenance. Normalize locators according to their chain rules; do not lowercase every chain address.

### Matching provider asset identifiers

Use an explicit asset mapping registry; names and tickers only suggest candidates. Recommended resolution order:

1. Look up an active, evidenced `SourceAssetMapping` by issuer + namespace + external identifier, within its validity period. Include chain/account context where the source namespace requires it; a provider-wide ticker is sufficient only if the provider defines it unambiguously.
2. If the source supplies a blockchain locator, resolve the chain-qualified contract/mint or native-asset identifier to an existing asset. A name match on another chain is not equivalent evidence.
3. For an unknown custodial identifier, use source metadata and curated mappings to propose candidates. Provider names, ticker aliases and descriptive names do not themselves authorize merging assets.
4. Accept an unambiguous mapping supported by a trusted identifier or reviewed registry entry. Otherwise keep a stable unresolved source asset and create a review issue.

For example, two custodial providers' different Bitcoin identifiers can map to the same canonical Bitcoin asset after verification. Two tokens called `USDC` on different chains retain distinct representation IDs; an evidenced economic-exposure relationship can group them for analysis without merging their ledger identity. These examples illustrate the contract, not a shipped mapping catalogue.

Store mapping state (`unresolved`, `proposed`, `confirmed`, `conflicting`), candidate targets, evidence, resolver/mapping version, reviewer or acceptance mechanism, validity dates and decision history. A confirmed mapping must be unique within its applicable source context and interval. Conflicting evidence opens an issue; it must not silently replace the active mapping. A human correction records its scope and reason and preserves the original source identifier.

Unknown and ambiguous assets remain visible with exact source quantities and warnings. Do not inherit a candidate's prices, scale, rewards or underlying exposure. If their value cannot be established independently, portfolio totals must be unavailable or explicitly labelled partial with omitted holdings; unresolved does not mean zero. A verified independent price may support valuation while canonical matching remains unresolved, but its source and identity limitation must remain visible. Unsupported reward/base matching also withholds the affected yield comparison.

Resolving or correcting a mapping produces a new normalization revision and deliberately reprocesses affected records. Convert source units exactly into the target scale, preserve event/source identities, and invalidate affected reconciliation and derived reports. Do not edit the asset ID on historical effects in isolation or merge unresolved records by ticker. Unknown spam can still be explicitly excluded under the normal tracking policy.

Legacy positions retain their IDs and roles. Backfill a default local portfolio and explicitly labelled legacy Nexo/Binance accounts where source evidence permits. Do not guess account ownership from arbitrary strings. If two real accounts were previously merged under one provider-wide ID, the ledger cannot be split from that ID alone: require original-source replay/account assignment and show unresolved history until then.

### Exact quantities and prices

Recommended representation: a validated arbitrary-precision signed integer coefficient plus an asset's fixed ledger scale. In Rust use an exact integer abstraction; persist canonical integer text in DuckDB initially, with exact aggregation in core. This removes both the effect and aggregate `BIGINT` limit and supports large chain quantities. SQL floating-point casts must never feed reconciliation or disposal accounting. The storage/performance tradeoff is explicit: initially filter/order in DuckDB and aggregate exact values in core; benchmark before adding materialized exact aggregates.

Alternatives: `HUGEINT`/`i128` would widen today's limit but still impose a fixed range; a single fixed-scale SQL decimal does not represent every asset's scale. Neither is the preferred general contract. The concrete Rust integer/decimal library remains an implementation choice, with bounded input lengths and checked resource use.

Parse original source decimal strings exactly; reject unsupported precision instead of rounding silently. Keep original amount text and source scale as evidence. Asset ledger scale may exceed a provider's reported precision; zero padding expresses unit conversion, not extra source accuracy. Reject a conflicting scale upsert. A deliberate scale increase must rescale all affected effects/observations/lots atomically or replay source data with verification. A token redenomination is an evidenced conversion event/asset relationship, not a metadata edit that changes historical balances.

Use exact decimal strings for new quoted prices, fees, rates and realized proceeds; define rounding only at display/output boundaries. Market prices can be estimates despite exact decimal storage. Legacy floating-point prices get a labelled legacy origin; converting their representation does not recover missing precision. Nexo float-derived ledger amounts also cannot be repaired by widening a column: preserve legacy evidence, replay original files when available, compare changes, and invalidate affected reports/reconciliation.

### Economic events and evidence

Keep one canonical economic event with ordered balance effects, supported by one or more source records. Add these logical records:

| Record | Purpose |
| --- | --- |
| `ImportBatch` | Connection/account/source, exact file/response hash or page identity, fetched time, requested coverage, parser/mapping version, completion state. Completion commits with events/registrations/observations. |
| `SourceRecord` | Batch reference, source key/namespace, original timestamp/timezone and amount fields, payload hash and restricted local evidence reference. Credentials/request auth headers are never evidence. |
| `EconomicEvent` | Stable event ID, occurrence time, account context, kind, normalization revision, status; links to all supporting source records. |
| `EventEffect` | Event ID + deterministic effect index, position ID, exact signed coefficient, effective time; semantic component such as principal, reward, fee or borrowing cost. |
| `EventRelation` | Links between events: transfer legs, reversal/correction, reward payout, recovery; evidence, review status and confidence where inference is used. |
| `AttributionOverride` | User classification/lot-attribution correction with revision/reason; does not overwrite original source records. |

Kinds include external deposit/withdrawal, internal transfer, trade/swap, interest, staking reward, cashback, signup bonus, promotion, fee, loan draw/repayment/interest, recovery and unknown. A trade may contain principal exchange and fees in a third asset. Fees carry purpose, paid asset and related event so reports can distinguish trading/switching fees, earning costs and borrowing costs without counting a fee twice.

An unknown classification may still have valid balance effects. It blocks affected earnings/benchmark interpretation rather than being silently treated as zero income. Do not require numeric sums across unlike currencies to balance. When counterparties are represented, per-asset movement consistency is useful; a universal same-number double-entry assertion is invalid across exchange rates.

Do not infer an internal transfer merely from equal nearby amounts. Persist confirmed or proposed links and the accounted network fee. Portfolio-boundary contribution queries use account membership and transfer links: a movement into an untracked account is external to that selected portfolio even if it belongs to the same person elsewhere.

Source record uniqueness and economic event uniqueness are different. The same operation in API and CSV can have two evidence links and one event. Two legitimate identical rewards need two event identities. Use native stable IDs where available; otherwise use file occurrence identity and an explicit overlap matching process that preserves multiplicity. Payload hash alone is insufficient. Ambiguous overlap produces an issue, not an automatic deletion.

Proposed revision policy: retain source evidence and normalized event revisions locally, with one active projection into the balance ledger. Replacing effects is still allowed, but no longer destroys the explanation of a correction. This is an audit history, not a promise of tamper-proof storage. A mapping-version upgrade must replay previously completed files deliberately; exact-byte dedup must not suppress remapping. Preserve existing success markers and create a new normalization run linked to the same evidence. Keep the former active projection until a complete replacement commits.

### Reconciliation and current versus historical data

Extend observations with account/source/batch IDs, observation time and fetched time, quantity basis (total/available/locked/debt), covered asset/position scope, and completeness. Missing from a partial endpoint does not mean zero. Evidence derived from the same movement export is labelled a self-consistency check, not independent reconciliation.

Add history coverage per account and stream (trades, rewards, transfers, etc.): interval, retrieval completeness, retention limitations, unresolved mappings and provenance. Endpoint pagination success alone does not prove coverage back to first activity. Snapshot equality at one cutoff is necessary but insufficient to prove complete earning history: missing deposit/withdrawal pairs can cancel.

Persist `ReconciliationRun` and issues with cutoff, included scope, observation IDs, active ledger revision, expected/actual exact amounts, and outcome. States distinguish unknown, incomplete, mismatch, verified-at-cutoff and stale. Reports assess completeness for their selected interval and purpose. A new import, remapping, exclusion-policy change or lot override invalidates affected derived results rather than leaving a timeless verified boolean.

Show independently verified current balances even when history is incomplete, retaining provider warnings. Do not mix snapshots from unrelated timestamps and call the aggregate a verified common-cutoff balance. Withhold affected historical totals/earnings; offer explicitly labelled reconciled-provider-only subtotals with included/omitted accounts. Snapshot observations never manufacture ledger movements.

Proposed exclusion behavior: current tracking policy applies to all displayed periods, including historical totals, earnings and benchmark inputs. Record policy revisions so a saved analysis is reproducible. Exclusion suppresses the holding and associated earnings/exposure, not the source evidence. Transfers across the included/excluded perimeter must be reflected in that report's cash flows. A policy must not exclude a receipt position but retain its underlying value through a second route.

### Core ingestion and query APIs

Replace anonymous `Vec<Transaction>` ingestion with an identified, account-scoped normalized batch containing asset mappings, account/position registrations, events/effects, provenance, observations and coverage. Keep acquisition/parsing in provider libraries, economic conversion in adapters, invariants/reconciliation/storage in core. Incrementally adapt existing helpers; no provider fetch should fabricate product rate zero to satisfy a required field.

Queries take `PortfolioId`, cutoff/interval, reporting currency, tracking-policy revision and a completeness requirement. Return values plus coverage, as-of timestamps, source/methodology references and warnings. Provide exact account/position/asset sums, net worth, independent current observations, eligible historical subtotals and event detail. Do not force callers to parse provider names from position IDs. Credentials live in a dedicated secure configuration boundary referenced by ID; the chosen secret-storage backend is a separate implementation task.

## B. Held investments and earnings

Add `HeldInvestment` with account, product reference when known, opened/closed dates, principal/base asset, reward policy and source references. Opening date means evidenced position opening; oldest imported record alone is a lower bound when history is incomplete. A product reference may remain unknown while holdings are valid. A held investment can have multiple balance buckets and repeated term instances. Its balance is derived, not another independently counted asset.

Persist products as reusable definitions; rates are dated observations, not an `apy` field on the position. A `RateObservation` needs observed/effective dates, source, review/freshness state, input asset, reward asset, rate basis (APR/APY/variable/unknown), compounding convention, tiers/caps and qualification/fee/term references. Unknown differs from zero. Store personal effective terms separately from advertised offer terms. Existing generic `Product` should evolve or be replaced; do not preserve fabricated products as verified offers.

Add reward receipt attribution to the earning investment/base asset and affected payout effects. Aggregate platform payouts may require explicit allocation evidence; if attribution is unavailable, show provider-level earnings and withhold unsupported per-position effective yield. Record gross reward and fee components independently, including fees in another asset.

Actual realized/unrealized analysis needs acquisition lots and disposal allocations (including reward lots), realized consideration, fee components and transfer continuity. Oldest-first is the default selected rule; alternatives and overrides are revisioned. Account-to-account transfers move lots without realizing them; swaps realize for this product report. A composite conversion needs explicit continuity treatment rather than classifying every receipt-token movement as profit. Tax-specific realization rules remain later and separate.

Derived earnings are projections from receipts, lots/disposals, exact effects and quotes. Persist a calculation version and input revisions for saved results; do not store a single mutable `earned_total` as truth. Show receipt-time economic value, retained/realized reward value and price change as separate explanatory measures. They overlap: adding today's reward value to its full price gain would double count. Report net proceeds and costs with clear allocation to the selected period; do not subtract a disposal fee already included in net proceeds again.

### Proposed calculation conventions and unresolved choices

For different-currency rewards, define quantities `q_i`, receipt times `t_i`, and reward/base quote `P(t)` in units of base per reward:

- Hypothetical retained reward value now: `sum(q_i * P(now))`, with explicit comparable conversion costs if enabled. Actual sold rewards do not disappear from this hypothetical history.
- Immediate-conversion baseline: `sum(q_i * P(t_i))`, using receipt-time quotes and a declared fee policy. No extra lending/staking return is added to this reward baseline.
- Actual received-earnings view: retained reward quantities valued today plus realized proceeds for disposed quantities, after allocated costs. This is a different result from hypothetical retention.

Use dated principal exposure `C(t)` and investable exposure duration, not just the original deposit. Recommended first historical summary: capital-time-normalized annual earning rate `effective_base_rewards / integral(C(t) dt in years)`, plus cumulative reward value and explicitly defined cumulative percentage. This is a simple annualized earning rate, **not automatically compounded APY**. To satisfy the requested equivalent APY, define a separate cash-flow-aware annualization with worked examples before implementing it. Time-weighted return and money-weighted return answer different questions; neither should be selected silently. Zero/negative principal, short durations and missing allocation history need unavailable/unstable states rather than infinity.

Current immediate APY uses the verified applicable current product rate with conversion into the earning asset on receipt, preserving the stated APR/APY convention. Current annual earnings use the current principal and applicable tiers/caps/costs, not future contributions or assumed renewal changes. The requested adjustment by past reward performance needs an explicit historical retention factor and observation window. Proposed candidate: retained/immediate reward value ratio since position opening, shown alongside the unadjusted annual earnings; do not multiply unrelated APYs or annualize that ratio again. Positions without sufficient receipts show the immediate figure with the adjusted figure unavailable. This factor/window and equivalent-APY convention are open methodology choices, not invented confirmed requirements.

Benchmark projections start from evidenced holdings and replay the selected portfolio's external contributions/withdrawals on their actual dates, earn no staking/lending yield, and use versioned valuation/fee assumptions. Preserve contributed asset quantities. Internal transfers are excluded. Asset selection for cash contributions, withdrawal depletion, starting history and baseline fee treatment still needs worked examples before calculation code.

## C. Wallets, composites, losses and exposures

Add dated `PositionExposure` edges from a held share/receipt position to underlying assets/positions, with quantities/conversion ratios, chain/protocol/provider identifiers, evidence and freshness. Underlying exposures are analytical children, not additional owned balances. Choose one valuation route per root holding (direct receipt price or underlying sum); never add both. Prevent graph cycles and count wallet-held receipt tokens only once when a protocol adapter discovers the same claim. Unknown underlying composition keeps a warning; use an independent direct valuation when available, not invented allocation.

### Matching DeFi products and resolving underlying assets

Use protocol-specific position resolvers alongside the asset registry. Keep these identities separate:

| Identity | Matching evidence |
| --- | --- |
| Product/market/vault/pool | Chain + protocol + contract/market/pool identifier; an NFT or account-specific position reference where needed. A display name or advertised yield is not a product identifier. |
| Ownership representation | Chain-qualified receipt/share token, NFT, or protocol account-position identifier. Some positions have no transferable receipt token. |
| Underlying assets | Asset registry resolution of the actual underlying contract/mint/native locators returned by supported protocol data. |

An adapter first identifies the supported product, then reads the user's ownership quantity and the protocol-specific redemption/composition data. Record the observed block height/hash where available, observation time, source, resolver version and freshness with the exposure edges. Resolve every underlying asset through the same asset matching rules; never infer composition from a product name such as “ETH yield vault.” Product-to-token relationships need evidence: one share token address alone may not distinguish all markets or user positions.

Synthetic example: 10 vault shares represent 12 units of asset A at block B. Store the holding as 10 shares and the dated conversion as 1.2 A per share. A later observation may change this ratio without creating another holding. A pool can have several underlying assets; a concentrated or account-specific position needs its actual position state rather than a generic pool percentage.

Nested exposures can be expanded through supported resolvers, retaining the path and observation references. Detect cycles and bound traversal. Separate confirmed, partial, stale, unsupported and conflicting resolution states. Missing resolver support, unknown underlying identities or inconsistent snapshots produce warnings and withhold unsupported exposure/yield attribution. Partial composition must not be normalized to 100% by assuming the unknown remainder is zero. A trustworthy direct holding valuation can remain available independently of the underlying breakdown; if neither route is valid, use the missing-value/partial-total policy above.

### Single-count ownership and valuation

Wallet/indexer and protocol observations can describe the same ownership claim. Link their evidence to one canonical holding using chain + owner/account + product + ownership representation/position reference. Keep genuinely distinct lots/positions separate. If equivalence is ambiguous, create a duplicate-claim issue and withhold an unqualified combined total rather than counting both or discarding one arbitrarily.

For each root holding, select and record one valuation route at the report cutoff: direct receipt/share valuation **or** underlying composition valuation. The underlying tree is a drill-down, not additional owned inventory. Nested conversions replace a parent's value representation; they are never added on top of it. Independently held units of an underlying asset elsewhere remain real separate holdings and are counted normally.

For example, shares worth USD 100 with underlying exposure of USD 100 contribute USD 100 total. If the wallet also independently holds USD 30 of that underlying asset, total is USD 130. Excluding the share holding removes its valuation and derived underlying exposure/earnings, without removing those independent USD 30 holdings. Common observation cutoffs and exposure provenance are required to avoid mixing incompatible conversion ratios. Do not use stale or partial underlying composition to claim a complete current exposure breakdown.

Keep principal/yield-bearing share exchange-rate changes distinct from price-only changes: some rewards accrue through growing redeemable value without a payout event. Such products need dated accrual/conversion evidence and a specified accounting method before their earnings are called verified.

Collateral restrictions link asset positions to liability positions with provider evidence, valuation haircuts, limits, interest and release rules. They do not transfer economic ownership or add a second loan. Historical rates/constraints matter; a current liquidation threshold cannot establish historical safety.

For failed platforms, retain claim quantity and movements but apply a dated zero valuation/loss status until recovery. Claim-linked distributions create actual recovery events and reduce/settle the claim only with evidence. Do not report ordinary market value for an inaccessible failed claim or duplicate a distribution as both new income and the full restored claim. Record the valuation policy/reason separately from liquidity restrictions: an ordinary locked position is not automatically worth zero.

## D. Public catalogue and paid risk boundary

Catalogue schema, separate from private portfolio rows: providers, protocols, chains, product offers, supported assets, versioned rate/fee/term/qualification observations, evidence and review/freshness states. Personal eligibility, membership/token holdings, liquid reserves, tolerance and concentration limits stay local/private. Offer IDs can be referenced without importing every public offer into the personal ledger.

Paid records: common risk assessments with dimensions/overall ratings, methodology version, evidence/reasoning, reviewer, validity and review dates; provider dependency edges with direction, relationship type and uncertainty. The same assessment serves everyone; preferences affect use, not the rating itself. Public descriptive risk summaries are separate publishable fields. Subscription absence must not prevent local balance/earnings queries. Locally cached paid responses need source/version/entitlement metadata; retention policy is a later service decision.

Allocation scenarios reference actual included holdings, comparison horizon, current rates/fees, eligibility, risk-data versions and exposure limits. Store assumptions and cost components if saved; never mutate current holdings to represent a suggestion. Known risks above limits warn by default; unknown/stale rates or assessments exclude suggestions until verified. Concentration aggregates provider, asset, protocol and chain exposure without summing overlapping dimensions into more than portfolio value. Dependency links inform correlated risk; do not invent failure probabilities from a graph.

## E. Liquidity planning and notifications

Add held `TermInstance` records: principal allocation, start/maturity, recurring offer/term reference, renewal setting and decision deadline. Several term instances may belong to one product/held investment. Keep term principal allocations from duplicating balance amounts. Capture dated availability constraints (lock/unbonding/withdrawal pause/limit), debt/collateral restrictions and uncertainty.

Withdrawal scenarios freeze today's reporting-currency prices, tracking policy and selected positions. Default scenario stops renewals from today; it does not modify provider settings. Graph availability is cumulative unlocked on-platform value, with unavailable/unknown scope and fixed denominator documented. A target amount/date scenario adds repayment funding/cost and collateral safety assumptions, and can be infeasible. Selecting 50%/100% must define whether denominator means net assets, gross included assets or realizable amount; debt cannot simply be treated as negative unlock inventory. Resolve this before implementing the graph.

Later delivery planning adds route nodes/edges, fees, conversion quotes, eligibility and settlement timing; a route is not a ledger event until an actual movement occurs. Opt-in notification rules store enabled types, deadlines/lead times, net-dollar/percentage thresholds, horizon and delivered state. All default to off. Refresh intervals and manual refresh controls are settings, not transaction metadata.

## F. Later extensions without premature implementation

Portfolio identity enables separate family/client data now. Later access records associate principals with owner/admin/view permissions, invitations and acceptance/revocation. A manager's access never makes a client's assets part of their own personal total. Local handoff selects portfolio-scoped evidence, settings, imports and permitted API credentials through a secure manifest; OAuth grants are not copied. Preserve canonical event IDs/revisions so later synchronization does not duplicate the cloned history. Cloud sharing uses shared rows with scoped authorization; local clones remain separate DBs. Continuous synchronization needs tombstones/conflicts and credential boundaries later, not a claim that local copy revocation erases retained records.

Broader financial instruments can add instrument metadata, ownership shares, corporate actions and liability schedules when bank/broker/pension/mortgage support is scoped. Keep spending categories and tax/accounting policies as separate projections. Tax sync, mobile, backup, community adapters, affiliate attribution and invoices require their own later contracts; none justifies copying unused Airtable fields now.

## Phased migration and implementation plan

| Phase | Schema/core scope | Exit condition |
| --- | --- | --- |
| A1 | Portfolio/account/connection identity, stable source namespace, exact quantity abstraction and protected scales. | Existing imports/roles/totals preserved; multiple accounts do not collide; realistic large quantities survive writes and sums. |
| A2 | Source batches/records, classified events/effects and revisions, observations/coverage/reconciliation, tracking policy and scoped queries. | Reimport/remap is atomic; current verified snapshots coexist with withheld history; corrections/exclusions invalidate projections. |
| B | Held investments/products/rate evidence, receipts/principal flows, lots and disposal attribution, versioned calculation contracts. | Required earnings/benchmark examples have agreed calculations; unknown rates/attribution do not produce fabricated yield. |
| C | Chain identity refinements, share/exposure/accrual models and claims/recovery policy. | Wallet/DeFi value counted once; losses and exposures explained. |
| D/E | Catalogue/risk schemas and scenarios, term/constraint/notification records. | Independent catalogue and private settings; net opportunity and liquidity examples pass. |
| F | Sharing/sync and broader services/instruments when scheduled. | Separate reviewed contracts; no local-first/subscription-free regression. |

Recommended next implementation slice is A1 plus the minimal A2 event/evidence/reconciliation contract used by existing adapters and Kraken. Do not implement phase B calculations as part of the connector. Provider documentation should explain old and new mapping behavior as adapters transition.

Append migration steps to the existing registry without changing the first three or introducing an applied-version gate. Step numbers/names should be assigned when patches are ready, not reserved for this entire proposal. Every new step reruns on every open and checks current schema/data. Use per-step transactionality, explicit insert columns and deterministic upsert/backfill keys. Never rerun destructive rebuilds on already-upgraded tables or overwrite user labels, mappings, policy, explicit roles or evidence.

Backfill metadata conservatively; mark unavailable categories/provenance as legacy/unknown. Do not synthesize source evidence, independent observations, verified rates, term dates or a complete history. Widen stored quantities losslessly; replay float-derived data separately as an explicit import/remapping operation. Copy/rename tables only when schema inspection requires it, preserving IDs, ordering, import markers, observations and price coverage. Keep temporary-table creation/retry behavior transaction-safe. After rebuilding exact columns, migrate every writer/query in the same implementation slice so old SQL cannot truncate results.

A failure rolls back its step and stops opening the store with the step name. Earlier committed steps remain safe to rerun; corrections permit retry. The application should expose a schema/normalization compatibility version for diagnostics and refuse unsupported writers; this informational compatibility version must not skip migrations. File import hashes stay useful even when normalization versions change.

## Acceptance examples for implementation

1. **Debt and counterparties:** USD 100 assets, USD 20 signed debt, arbitrary fee counterparty balance → USD 80 net worth. Repaying USD 5 from tracked cash leaves USD 95 assets and USD 15 debt → USD 80, before fees. Collateral remains owned.
2. **Two accounts:** two Nexo/Kraken connections reuse source record `123`; both import once into their own positions. Reimporting either changes neither the other account nor its verification status.
3. **Large units and scale:** an 18-decimal balance above `i64::MAX`, and a sum of individually valid old effects above that limit, remain exact. A conflicting scale registration fails without changing existing quantities.
4. **Binance Earn:** subscription/redemption is internal principal movement, not income. A separately evidenced reward is income attributable to the earning investment; `apy: 0.0` placeholder migrates to unknown.
5. **Kraken-style fee:** acquire 1 asset B by spending 100 asset A, paying 2 asset C. Preserve three asset quantities and one fee component without equating numeric amounts or counting the fee twice. This is a synthetic mapping example, not verified Kraken endpoint behavior.
6. **Overlap and correction:** API and CSV support one event; two distinct identical rewards remain two. A parser upgrade reuses evidence, replaces only the active projection atomically, retains the earlier explanation and invalidates derived results.
7. **Different reward coin:** receive 10 R when 1 R = 0.1 base; now 1 R = 0.05 base. Immediate baseline is 1 base; hypothetical retained value is 0.5 base before costs. A later reward disposal changes actual earnings, not this hypothetical comparison.
8. **Partial disposal:** receive 10 R, sell 4 for 0.4 base after fees, retain 6 now worth 0.3 base. Actual reward value is 0.7 base; hypothetical retention still considers all 10. Lot/fee allocation is explainable under oldest-first and override rules.
9. **Changing principal:** hold 100 base for half a year and 200 for half a year; 15 base effective rewards → capital-time is 150 base-years, simple annual earning rate 10%. Equivalent compounded APY is separately defined, not silently labelled 10%.
10. **Missing history:** a complete current snapshot matches USD 1,000, but earlier reward coverage is missing. Show the current balance with warning; withhold that account's earnings/historical total. Show other reconciled accounts as a labelled subtotal.
11. **Exclusion/composite:** a USD 100 receipt holding represents underlying USD 100; total is USD 100. Excluding the holding removes both value routes and its earnings; hidden spam contributes nothing.
12. **Failure/recovery:** retain a failed claim of 100 units but value it at zero from the evidenced loss date. An actual recovered distribution is visible once, with outstanding claim status retained.
13. **Liquidity:** three USD 100 terms mature on different dates. Total exposure is USD 300 once; stop-renew scenario releases each allocation once. Renewal settings and actual balances are unchanged by the suggestion.
14. **Startup/retry:** fresh DB, legacy DB, upgraded DB and corrected-after-failure DB reach the same intended schema. Repeated opens preserve effects, roles, evidence, user policy and completion markers without duplicate backfills.
15. **Asset ambiguity:** two tokens share a ticker but have different chain/contract locators. They remain distinct. An unknown custodial alias has two plausible candidates; quantities remain visible, no candidate price is silently applied, and any incomplete total names the omission. A reviewed mapping preserves source evidence and triggers exact rescaling/reconciliation invalidation where needed.
16. **DeFi discovery overlap:** a wallet reports 10 shares and a vault adapter reports the same claim representing 12 A at block B. Record one holding with two evidence sources and dated exposure; select one valuation route. Independently held A remains separate. An unsupported nested asset produces a partial exposure warning, not a fabricated complete breakdown; cycles terminate with an issue.

## Decisions still requiring worked examples

The entity boundaries, preservation rules and recommended phases above are sufficient to scope the first implementation patch. Calculation-sensitive questions remain deliberately open: equivalent APY for changing principal; the historical reward adjustment factor/window; benchmark contribution/withdrawal/fee handling; shared/composite yield attribution; liquidity percentage denominator and debt stress assumptions. Resolve these with synthetic examples before their calculation phases. They do not block account identity, exact amounts, event provenance or existing-provider documentation.

Implementation choices still to verify locally include exact-number library/storage performance, legacy source replay availability, provider-specific record namespaces and rate/product coverage. Official connector API verification belongs to each provider task; this review does not assert new current endpoint support.
