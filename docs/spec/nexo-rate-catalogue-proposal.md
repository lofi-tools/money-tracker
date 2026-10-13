# Proposed Nexo rate catalogue and qualification rules

Status: **approved research and design proposal**, committed as `46585c0`. Observed 13 October 2026. The [bundled catalogue, evaluator and account jurisdiction](nexo-rate-catalogue.md) are now implemented for review. Automatic held-rate defaults, expected-yield matching and the scraper remain outstanding. The [ordered task](../todo/master-todo.md#immediate-provider-wiring-fixes--ordered-proposed) requires approval before this stage's commit. The preceding term matcher is committed separately.

## Official sources and applicability

| Reference | Official source | Evidence obtained |
| --- | --- | --- |
| UK-FLEX | [Flexible Savings guide](https://support.nexo.com/article/how-to-earn-interest-with-flexible-savings) | Rendered article, whose footer explicitly says UK residents; rates, wallets, eligibility, payout and band mechanics. |
| UK-FIXED | [Fixed-term Savings guide](https://support.nexo.com/article/how-to-earn-interest-with-fixed-term-savings) | Rendered UK article; rates, durations, renewal and term-end payouts. |
| UK-TIER | [Loyalty Programme guide](https://support.nexo.com/article/nexo-loyalty-program-explained) | Rendered UK article; ratio denominator, tiers and balance grace period. |
| MINIMA | [Asset information table](https://nexoio.my.site.com/assets/assets-info) | Public iframe embedded in the Savings guides; exact product-specific unit thresholds. The table's own regional scope/effective date is not explicit. |
| PUBLIC-FLEX | [Flexible Savings page](https://nexo.com/earn-crypto) | Public headline maxima; not an applicable account rate matrix. |
| PUBLIC-FIXED | [Fixed-term Savings page](https://nexo.com/fixed-term-savings) | Public maxima, payout and renewal descriptions. |
| NEAR | [NEAR savings page](https://nexo.com/earn-crypto/near-protocol) | Public fixed maximum; no complete tier matrix. |
| US-FIXED | [US Fixed-term Yield guide](https://support.nexo.com/article/how-to-earn-interest-with-fixed-term-yield-us) | Separate Wealth Club programme; must not supply UK Loyalty rates. |
| ACCOUNT | [Savings breakdown](https://platform.nexo.com/savings-breakdown) | Requires login in the inspected browser; full asset/tier/payout/term/band matrix unavailable without account evidence. No sign-in attempted. |

The same Help Center URL may render regional content. Record the rendered programme/footer as well as the URL and observation time. UK browser content is evidence for a UK profile, **not a determination of the user's account jurisdiction**. Dates below are observation dates, not proof that rates became effective on that date. Do not backdate this snapshot into historical terms.

## Observed headline annual rates

Percentages below are advertised annual rates, predominantly **up to** maxima. They are not a complete APY matrix and must not become unconditional current rate evidence. Preserve the provider's annual-rate wording until nominal APR versus effective APY and compounding conventions are verified for each rule.

| Asset | Flexible annual rate/max | Fixed annual max | Source |
| --- | ---: | ---: | --- |
| BTC | 4.7% | 5.7% | PUBLIC-FLEX / PUBLIC-FIXED |
| ETH | 5.25% | 6.25% | PUBLIC-FLEX / PUBLIC-FIXED |
| NEXO | 3% (not labelled up to in UK-FLEX) | 9% | UK-FLEX / PUBLIC-FIXED |
| USDC | 8.5% | 10.5% | PUBLIC-FLEX / PUBLIC-FIXED |
| USDT | 9.5% | 11.5% | PUBLIC-FLEX / PUBLIC-FIXED; restrictions still apply |
| DOT | 13% | 15% | PUBLIC-FLEX / PUBLIC-FIXED |
| XRP | 6.25% | 8.25% | PUBLIC-FLEX / PUBLIC-FIXED |
| SOL | 7% | 8% | PUBLIC-FLEX / PUBLIC-FIXED |
| BNB | 6% | 7% | UK-FLEX / UK-FIXED |
| POL | 4.75% | 6.75% | UK-FLEX / UK-FIXED |
| NEAR | unknown from this collection | 8% | NEAR |
| EURX / GBPX / USDX | 6.5% | 10.5% | UK-FLEX / UK-FIXED |

The US source advertises ETH up to 8%, NEXO up to 12% and DOT up to 16%, under Wealth Club rather than Base/Silver/Gold/Platinum. Keep that evidence in a separate profile. Do not select the highest value across regional pages or mix US qualifications with UK rates.

## Tier and qualification evidence

UK-TIER gives Base without a NEXO requirement, Silver at 1%, Gold at 5%, and Platinum at 10%. Its ratio is **NEXO token value / (provider portfolio value − NEXO token value)**. This differs from NEXO divided by total value. Scope is the Nexo account ecosystem, not the user's all-provider portfolio. The guide mentions Savings, Credit, Futures and Dual Investment, and pending movements; it does not supply a complete debt/netting valuation specification.

UK-TIER requires $5,000 and describes a seven-day restoration window following a fall below it. UK-FIXED separately says existing terms below that threshold keep Base plus term bonus, lose the NEXO bonus, and cannot renew. Do not invent one common transition instant: collect platform status or mark the grace/term fallback timing unresolved. UK-FLEX requires Savings Wallet assets, except NEXO also earns in the Credit Wallet. It identifies USDT/USDP/PAXG restrictions for EEA residents and tier restrictions for DOGE/GRAM and GRT/OSMO. Opt-in outside the EEA lasts 365 days. These conditions need account-specific confirmation.

Proposed decision rules:

- Prefer an evidenced platform tier/eligibility state over a reconstruction from incomplete holdings. A missing provider wallet, stale/unpriced asset, unknown jurisdiction, denominator of zero or unmodelled pending movement gives unknown reconstructed qualification. Do not silently treat our net worth as Nexo's tier denominator.
- Keep qualification outcomes `eligible`, `ineligible`, `unknown`, separate from an applicable rate. Missing conditions are unknown, not zero; verified ineligibility can yield no earnings for its effective interval.
- Store grace-period start/check dates and explicit opt-in expiry where known. Keep active-term fallback separate from new-term creation/renewal permission.
- Do not infer wallet eligibility from unlocked ownership alone. Apply the NEXO Credit Wallet exception only under a matching, reviewed profile.

## Minimum units: proposed seed observations

UK-FLEX describes roughly $500 per asset/product, updated monthly on the 15th. Use the published **unit threshold**, not today's conversion of $500; do not combine flexible and locked quantities to pass two separate thresholds. The exact fixed threshold grouping (per term versus summed fixed balance) needs confirmation from account evidence before auto-selection.

The iframe values observed during this research for the initially relevant assets were:

| Asset | Flexible minimum units | Fixed minimum units |
| --- | ---: | ---: |
| BTC | 0.00653907 | 0.00644763 |
| ETH | 0.20492831 | 0.20079398 |
| NEXO | 626.09245306 | 616.7367224 |
| BNB | 0.69003359 | 0.69404555 |
| DOT | 493.68477449 | 504.79436015 |
| NEAR | 187.27224347 | 204.67768079 |
| POL | 5165.42639613 | 5207.93134613 |
| EURX | 436.13 | 433.34 |
| GBPX | 373.5 | 371.4 |
| USDX | 500 | 500 |
| USDT | 500.48509 | 500.129933 |
| USDC | 500.086845 | 500.052425 |

These are public observations, not private account data. Preserve exact decimal strings and source date. Reverify the iframe/profile and monthly effective date before using them as current qualification defaults; an expired or unscoped observation does not authorise an earning-rate assignment. `N/A` is explicit product unavailability, not a numeric zero. Source FIATx tickers need reviewed asset aliases; plain USD and USDX must not be merged.

## Durations and payout rules

The UK-FIXED table lists NEXO 3/12 months, DOT 1/6, XRP 1/3, BCH/TRX 1/3/6, TIA/EURX/USDT/USDC/USDP 3, GBPX/USDX/EURC 3/12, and one month for listed other assets, including ETH, BNB, NEAR and POL. ARB/DOGE do not support fixed terms. It also describes up to 50 terms per asset, a term-end payout, default renewal, and an optional collateral-driven early unlock. Mid-term payout changes can create two reward transactions. The fixed exchange feature can terminate selected terms with accrued interest forfeited.

UK-FLEX describes daily payouts following a 24–52-hour startup cooldown and snapshots, rather than every deposit immediately earning for a full day. Its NEXO payout example divides the nominal annual rate by 365, uses simple base-denominated accrual, then converts through USD/NEXO at payout. In-kind examples describe compounding. The detailed conversion rounding and numerical basis of every published percentage are not specified sufficiently for a universal formula.

Proposed calculation/matching rules:

- Store calendar-month duration separately from observed elapsed seconds. Neither one month = 30 days nor a generic unlock = scheduled maturity. Do not overwrite the matcher's observed-close semantics.
- Treat term length, payout currency, rate and eligibility as dated state: a fixed duration does not guarantee a fixed rate throughout it. Integrate changes over supported intervals; never apply today's catalogue to old rewards.
- Represent payout schedule separately from accrual schedule and compounding. Base-denominated expected earnings and a NEXO conversion are separate steps. A received NEXO quantity is not automatically a base-asset yield.
- Keep renewal and reinvestment as independent account/subscription settings. Published defaults describe newly created terms, not a user's existing toggles. Existing pinned overrides retain the agreed precedence.
- Store documented NEXO payout bonus ranges as ranges. The public guides give 0.25–2 percentage points for the UK Loyalty profile; that does not establish every tier/asset bonus cell. The complete matrix remains missing. Do not add the bonus to a maximum that already includes it, or subtract it to fabricate an in-kind base rate.
- Balance bands are marginal slices, not a rate applied to the whole balance. UK-FLEX names BTC/ETH/XRP/PAXG/LINK as banded assets; missing thresholds or upper-band rates make the applicable rate/earnings partial or unknown.
- Expected earnings may constrain a transaction match only with historically applicable rates, full principal coverage, payout choice and conversion evidence. Define tolerances from source precision/rounding, not an arbitrary percentage chosen to force a match. Two end payouts after a preference change need their own interval evidence; the current matcher correctly leaves competing rewards unresolved.

## Proposed bundled data and selection contract

Create a versioned, reviewed data file consumed by Rust; keep rules out of JSX and provider-specific magic constants. Suggested record fields:

- Identity: provider, programme/region scope, asset locator, flexible/fixed product, duration, tier, payout denomination and marginal band.
- Rate: exact fraction string, `annual_basis` (`apr`, `apy`, `unspecified`), accrual/compounding schedule, bonus fields, and `evidence_kind` (`headline_maximum`, `documented_rate`, `account_applicable`, `illustrative_example`).
- Conditions: account minimum/grace rule, unit minimum/grouping, eligible wallet, opt-in and expiry, jurisdiction/tier exclusions, and any promotion window.
- Provenance: source URL plus rendered regional scope, observation time, supported effective interval (nullable), review status and refresh deadline. A retrieval date never silently becomes the historical effective date.

A concrete proposed seed is NEXO Flexible: annual fraction `"0.03"`, UK article scope, `documented_rate`, annual basis not yet numerically confirmed, wallet condition Savings or Credit, plus portfolio/minimum/opt-in qualifications. ETH Fixed's `"0.0625"` is only `headline_maximum`, with no complete tier/payout/term/band cells. Both retain their source evidence; neither may automatically become an unconditional held-subscription APY.

The evaluator must return an applicable exact rate with assumptions/provenance, an explicit no-earn outcome, or unresolved reasons. Public headline records are display/catalogue evidence only. Compatible dated account evidence can complete a rule; a newer automated value respects existing manual/pinned override policy. Competing equally applicable records require review. Keep explicit source restrictions, price gaps, borrowing rates and reconciliation separate from yield assumptions.

## Missing evidence and acceptance before implementation

Still needed: the actual account programme/jurisdiction, complete rate matrix by tier/payout/duration/band, exact fixed-minimum grouping, annual basis and provider day-count/rounding conventions, supported historical rate snapshots, and qualification inputs absent from the CSV. The account breakdown page requires sign-in. A manually supplied table/export or confirmed account annotations can fill these gaps; no additional platform credentials are needed merely to preserve the public catalogue.

Implement meaningful cases for tier denominator/boundaries, grace transitions, minimum unit freshness, fixed eligibility versus active fallback, the NEXO collateral exception, unpriced/missing provider holdings, bonus double counting, marginal bands, payouts with changed currency, calendar terms, current versus historical dates, pinned overrides and conflicting regional evidence. Catalogue lookup must not contact the network or rewrite ownership. The later scraper should collect dated observations into this same format and require review for changed conditions.

Validation for this research stage: official pages were inspected through web retrieval and the rendered Help Center; the account matrix was confirmed to require login. Relative documentation links and whitespace checks pass. No runtime changes or new executable tests were made in the research stage. Subsequent implementation and validation are documented in the linked implemented contract.
