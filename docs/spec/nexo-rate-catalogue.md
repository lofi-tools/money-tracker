# Nexo bundled catalogue, eligibility and account jurisdiction

Implemented 13 October 2026, pending the user's implementation-review commit. The [approved proposal](nexo-rate-catalogue-proposal.md) retains source observations and design rationale. Sources: [bundled data](../../libs/lib-app/src/adapters/nexo/data/rates-v1.json), [evaluator](../../libs/lib-app/src/adapters/nexo/rate_catalogue.rs), [application service](../../libs/lib-app/src/service.rs), [account storage](../../libs/lib-core/src/storage.rs) and [migration 010](../../libs/lib-core/src/migrations.rs).

## Account-local jurisdiction

`account_jurisdictions` stores account ID, country code and declaration timestamp separately from imported account labels/references. `UK` normalizes to `GB`. Codes are syntactically checked (two uppercase ASCII letters); unsupported codes remain unknown to the Nexo programme evaluator, rather than being treated as UK. The table is initially empty: no migration or importer assigns a country from timezone, language, wallet holdings or this user's preference. `set_account_jurisdiction` requires an existing account, rejects older declarations, updates atomically with the portfolio metadata revision, and is idempotent for the same country. Reimports and repeated startup preserve it.

`AccountDto` carries optional jurisdiction with a deserialization default for older snapshots. Tauri details and the Solid details panel expose the original account's jurisdiction; older frozen previews remain compatible. There is no frontend jurisdiction editor or jurisdiction Undo action in this slice. The explicit helper targets an existing Nexo account without creating a database:

```sh
cargo run -p lib-app --example nexo-account-jurisdiction -- \
  "$PWD/.cache/data/portfolio.duckdb" legacy:nexo UK
```

On the user's instruction, the CLI portfolio's `legacy:nexo` account was declared UK. Its pre-change backup is in the ignored local cache; private database files are not committed. No other Nexo account receives this setting. The standard/default DB and development overrides remain [shared configuration](database-startup.md), not provider-specific paths.

## Bundled observations and applicability

`Catalogue::bundled()` parses and validates a versioned JSON snapshot at compile time using `include_str!`, with no network lookup. It holds source URLs, rendered regional scope, observation date, review deadline, exact decimal rate/minimum strings, product durations, tier thresholds, wallet exceptions, payout schedules and rule metadata. UK Loyalty and limited US headline observations are separate; the implemented eligibility profile is UK only.

All headline maxima are display evidence. The unqualified 3% NEXO flexible statement is retained as documented evidence, but its full applicability/annual basis is incomplete. No bundled record is a complete account-applicable rate. Consequently, **even fully eligible holdings get an unknown applicable rate from the current bundle**. No advertised maximum, inferred base rate or assumed payout bonus is saved as held-position `RateEvidence`.

The conditions/rates snapshot is observed 13 October; 27 October is the chosen two-week review deadline, not a provider guarantee. Outside that window the evaluator returns unknown. Public minimum-unit observations are unscoped and their effective/grouping rules are unconfirmed; 15 October is their next recheck marker. They are not silently promoted to current eligibility inputs. A caller must provide confirmed asset/product-scoped minimum evidence with effective/expiry dates and source reference. Its quantity must represent the same flexible-balance or individual-term scope. Dates are half-open: evidence expires on its `valid_until` date. Fixed durations are calendar months, never a hardcoded 30-day interval.

## Shared Rust evaluation contract

`Application::nexo_rate_eligibility(account, facts)` checks the saved account provider and reads that account's declaration. A declaration after the requested date cannot establish historical jurisdiction. `evaluate` is the pure counterpart for reviewed catalogue data. `Facts` are explicit source-confirmed inputs, not an automatic extraction from the incomplete CSV. The evaluator does not write rates, positions, quantities, policy, settings or overrides.

The result separates `eligible`, `ineligible` and `unknown` qualification from applicable-rate coverage. It returns stable reason codes, selected/headline record IDs, source URL, optional exact annual fraction and optional known annual earnings in base-asset units. These are prospective gross estimates holding principal/quotes constant; they are not received earnings, fees, borrowing costs or a NEXO payout-quantity prediction. There is no current frontend eligibility panel or automatic held-rate application. Existing current-rate queries and manual/pinned selection remain unchanged.

Implemented rules:

- UK tiers use exact `NEXO value / (provider portfolio − NEXO value)` boundaries, with zero-denominator/incomplete valuation unresolved. A reported platform tier takes precedence. The evaluator rejects negative quantities/valuations and NEXO value greater than total provider value.
- A verified platform access state takes precedence over balance reconstruction. Complete provider value above the account minimum supports access; below it without grace-status evidence stays unknown. Confirmed active fixed fallback uses Base; creation/renewal remain separate permissions, with unknown bonus cells still unresolved.
- Flexible earnings require wallet, opt-in and earnable/cooldown evidence. The NEXO Credit Wallet exception is explicit. Missing opt-in/cooldown/minimum facts stay unknown; known expiry, ineligible wallet or insufficient confirmed units are ineligible. Existing fixed terms are not retroactively denied by creation minima or opt-out.
- Fixed creation/renewal validates asset/duration and a confirmed per-term minimum. The 50-term limit applies to adding a new term; renewal replaces its existing slot. Published default renewal does not set a user's subscription toggle.
- Complete rate selection requires matching region, product, asset, tier, payout, duration where applicable, source scope, effective date and observation date. Missing matches and conflicting equally applicable evidence remain unknown. Headline and illustrative records are excluded even if other fields are populated.
- Marginal band slices use exact rational arithmetic. Known slices produce a known earnings subtotal; any relevant unknown slice withholds overall rate. Missing band tables for banded assets are not bypassed with a scalar headline.
- Effective APY is used directly. In-kind nominal APR requires explicit compounding frequency; simple NEXO APR can use the documented simple model. Discrete frequencies through 366 compound exactly; unsupported frequencies remain unknown. Bonus ranges are never added to a published total or used to reverse-engineer a base rate.

The bundle preserves EEA exclusions and schedule/cooldown descriptions as source metadata, but does not pretend to implement an EEA programme from UK rules. Automatic accrual reconstruction, expected-yield term disambiguation, provider status/minimum ingestion, complete tier/band imports, jurisdiction editing/Undo, and the periodic scraper remain later work.

## Validation and preservation

[Nexo catalogue tests](../../libs/lib-app/tests/nexo_catalogue.rs) cover twelve cases: eligible-but-unknown bundled rates, exact tier boundaries, scope/dates, collateral/cooldowns/minima/opt-in, grace and active fallback, fixed duration/minimum/count limits, complete applicable rates and conflicts, marginal partial bands, malformed evidence, stored per-account country isolation, explicit compounding, and renewal slot replacement. [Jurisdiction persistence test](../../libs/lib-core/tests/account_jurisdiction.rs) verifies idempotence, account isolation, metadata revision and reimport/reopen preservation. The existing offline copied-cache migration test preserved Nexo/Binance financial records through repeated opens.

Offline core/application/parser tests pass with the private Binance cache test explicitly excluded and provider credentials removed. The first broad run exposed that private test's missing cache/credentials prerequisite and macOS sandbox proxy-discovery failure; the documented offline lane was rerun outside that sandbox. It also found an existing Kraken readiness mismatch: acquisition was populated despite absent credentials while the prerequisite blocked it. The candidate now exposes no acquisition without credentials, matching its existing blocked state and offline acceptance test; provider transaction/fee mappings are untouched.

Desktop native service tests, actual Rust wire fixture generation, TypeScript checks, consumer builds and six desktop browser tests pass. The browser fixture verifies the UK label in details. No live provider refresh was needed. Implementation is uncommitted pending user review.

The annotation preview was regenerated from the user-selected CLI portfolio rather than the earlier Nexo test database. The UK account declaration is visible in details. That CLI database currently lacks some pricing/qualification evidence, so its unknown figures remain unknown; exporting the preview performs no provider refresh or ownership reimport.
