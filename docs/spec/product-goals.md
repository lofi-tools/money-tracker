# Product questions, pain points, and solutions

Confirmed direction as of 11 October 2026, distilled from the [user needs interview](../findings/user-needs.md). This is a proposed product brief and source material for a future homepage. It describes desired outcomes, not a list of features already shipped. The [project context](project.md) and topic specifications describe the current implementation.

## Product purpose

Help long-term investors understand what they own, what it earns, and whether the same assets could earn more elsewhere, while retaining control of their financial data.

The three central questions are:

1. **How much do I have across wallets, providers, and platforms?**
2. **How much yield is it earning?**
3. **Should it be earning more?**

Start with crypto and related fiat balances. Extend to banks, stocks, pensions, mortgages, and other personal assets and liabilities. The user-confirmed feature sequence is holdings overview, received/current annual earnings, opportunity comparisons, then withdrawal planning.

## Questions the software should answer

### Holdings and trustworthy totals

- What do I own across my accounts and wallets, and what is it worth in my reporting currency?
- What is my net worth after liabilities?
- What do my DeFi positions expose me to underneath their receipt tokens or pool shares?
- Which holdings or reports are verified, and which providers still need reconciliation?
- What was lost on failed platforms, and what has actually been recovered?

Aggregate economic value once, with underlying-asset drill-down. Discover wallet assets/positions automatically; hidden spam and explicitly excluded holdings do not affect totals or yield. USD is the default reporting currency, changeable in settings. Failed-platform funds have zero value until recovered, with losses and recovery evidence retained.

Historical value, earnings, and performance graphs belong in the initial dashboard. Include an early comparison against simply holding the same assets, accounting for fees and deposits/withdrawals. The baseline earns no staking or lending yield and matches external deposits/withdrawals on the same dates. Internal transfers between tracked accounts do not create new contributions. Show whether the investment strategy adds value over that baseline rather than treating new contributions as returns. This whole-portfolio comparison is separate from the reward-retention versus immediate-conversion comparison. The benchmark's asset mapping, initial quantities, treatment of actual trades, and fee assumptions remain to be specified.

Show verified current provider balances even if history is incomplete, retaining a warning. Withhold affected historical totals and yield until reconciled. Combined historical figures may show a clearly labelled subtotal of reconciled providers, identifying those omitted.

### Earnings and the value of rewards

- How much have I received in interest, staking rewards, cashback, signup bonuses, and promotional rewards?
- What remains after fees and borrowing costs?
- How much are my current positions earning per year at their current rates?
- If I had retained every reward since a position opened, how much of the earning asset would those rewards buy today?
- What cumulative effective return and annualized APY does that represent?
- Would converting each reward into the earning asset immediately have produced a better outcome than keeping it?
- What price gains or losses have I realized, and what remains unrealized?

Use net earnings as the main figure, with gross earnings and costs separately visible. Distinguish interest/staking from cashback/bonuses/promotions, while including these categories in total earnings. Show price gains/losses separately from earnings.

Refresh balances and rates automatically while the desktop app is running, with a user-configurable interval. Keep manual refresh in options. This is a long-term investing app; a real-time trading interface is not required.

For actual earnings, value retained rewards at today's prices by default. Use actual realized proceeds after fees for rewards sold or swapped; crypto-to-crypto swaps count as realization for this report. Default unit attribution to oldest first, with user-selectable rules and overrides. Preserve receipt evidence and avoid adding overlapping reward/gain views twice.

Show two distinct yield measures: **immediate APY**, based on the current product rate and immediate reward conversion into the earning asset; and **historical effective yield**, based on actual rewards hypothetically retained since opening and converted into that asset now. Account for invested capital and its duration when deposits/withdrawals change the position. This is relative reward-token performance, not proof that supply inflation caused a price change. The retained-versus-immediate conversion comparison applies to rewards paid in a different currency from the earning asset.

Default received-earnings reporting to the last 12 months, with calendar-year, all-time, and custom periods. Current annual earnings answer “how much am I currently earning per year?” Exact calculation and annualization conventions still need specification; this view must not silently become a forecast of future renewal or reinvestment decisions.

### Better opportunities for the same assets

- Could the assets I already hold earn more with another provider or product?
- What would my actual balance earn after fees, rate tiers, caps, qualification costs, and switching costs?
- Which opportunities am I eligible for, and what additional requirements apply?
- What risks come with each product and platform, and which exceed my preferences?
- Would splitting holdings across opportunities improve earnings while respecting my limits?
- Am I too concentrated in a provider or asset, including providers with shared dependencies?

Keep headline APY visible, but compare net earnings over one year by default or another chosen period. Include offers requiring extra tokens/memberships, showing qualifications, costs, and risks; users can filter them out. Show all opportunities with eligibility indicators and an eligible-only filter.

Use common risk ratings for all customers: an overall score plus separate ratings and explanations, produced through our documented methodology, third-party evidence, and human review. User tolerance changes how ratings are used. Offer conservative-to-aggressive presets and individual risk limits. Show opportunities exceeding limits with warnings by default and allow filtering them out. Missing/stale assessments or rates keep an opportunity visible with warnings but exclude it from allocation suggestions until verified and current.

Respect maximum exposure percentages per platform, protocol, and blockchain. Provider/asset concentration is a portfolio-level risk; shared provider dependencies can undermine apparent diversification. Provider-failure simulations are much later work.

### Availability and withdrawal planning

- How much is immediately withdrawable, and how much is locked or restricted?
- How long until 50%, 100%, or a requested amount becomes withdrawable across all platforms/products?
- If I need a particular amount by a deadline, when should I withdraw what?
- How can fixed terms be staggered to make funds available more frequently?
- What can I withdraw now, and what collateral becomes available after repaying debt?

Use a combined reporting-currency withdrawal graph at today's prices held constant. Default to stopping term renewals from today. Suggest a buffer for specific monetary targets. Let users require an immediately withdrawable minimum per asset and lock only the remainder. Debt-aware plans include repayment costs and collateral requirements.

Start with funds unlocked on-platform. Later, a money router maps platform/blockchain connections and fees, supporting conversion and delivery to a chosen wallet/bank account with lower routing costs. All first-version actions are suggestions; execution is a separate later possibility.

## Pain points and our proposed solutions

These are product hypotheses supported by the interview and older draft, not findings from broader customer research.

| Pain point | Proposed solution | Stage |
| --- | --- | --- |
| Assets are scattered across platforms and wallets. | Unified holdings and net-worth overview with independent balance verification. | First: holdings |
| Tracking requires repetitive spreadsheet updates. | Automatic provider sync and wallet discovery; original CSV imports/manual input only when needed, with export-page links. | Foundations |
| Incomplete history creates convincing but unreliable numbers. | Reconciliation warnings, withheld affected history, and explicit reconciled-only subtotals. | Foundations |
| DeFi receipt tokens obscure underlying exposure or get counted twice. | Position and underlying views with drill-down and single-count valuation. | Holdings |
| Advertised APY does not explain what rewards are worth after being held. | Current immediate APY beside cash-flow-aware historical effective yield in the earning asset. | Earnings |
| Fees, debt costs, bonuses, and price changes obscure earnings. | Net earnings first, with costs, reward categories, and realized/unrealized price changes separately explained. | Earnings |
| It is hard to compare rates that depend on tiers, caps, or memberships. | Actual-balance net earnings comparisons including qualification and switching costs. | Opportunities |
| A higher yield may add risks or conceal common dependencies. | Traceable product/platform ratings, portfolio concentration analysis, and a provider dependency map. | Opportunities; paid risk data |
| Risk preferences differ even when the underlying assessment is shared. | Common ratings with personal tolerance, specific risk limits, warnings, and filters. | Opportunities |
| Locking everything into one term makes funds hard to access. | Per-asset liquid reserves, staggered terms, and an aggregate withdrawal timeline. | Withdrawal planning |
| A deadline requires coordinating maturities and debt across providers. | Target-amount/date plans including repayment and collateral constraints; routing/delivery later. | Withdrawal planning; routing later |
| Relying on a hosted portfolio service limits control over personal records. | Subscription-free local tracking/earnings with user-supplied provider keys; optional managed cloud convenience. | Primary product model |
| Important maturities or meaningful yield improvements are easy to miss. | Opt-in configurable notifications, with net-dollar and percentage improvement thresholds after switching costs. | With relevant product capabilities |

## Public and paid product boundaries

The opportunity catalogue, yields, and descriptive risk summaries are public through a website and accessible dataset. Collect from provider APIs/websites where possible, with human review and manual additions/corrections.

Risk ratings and provider dependency maps are paid API content. Bank/broker integrations are also planned through a paid cloud API offering. Local holdings and earnings remain usable without a subscription, including free provider integrations using users' own keys. The secondary cloud version offers one login, managed keys/sync, and OAuth where available.

## Other confirmed roadmap topics

Open source starts with the app; license and component scope remain to be specified. Shared portfolios follow soon after single-user tracking: family/client management first, then couples/groups/households, then companies. Tax reporting comes later as a cloud service that syncs local data. Spending tools, community integrations, and mobile/sync are later additions. Affiliate monetization follows live paid cloud/API plans; invoicing is much later. Exact sharing, tax/document, offline/sync, and business-model details remain open.

For family/client management, let a manager add and track another person's accounts before inviting them to a preconfigured app. On signup, the recipient should find their data already available. The inviter retains shared admin rights unless the recipient explicitly removes them. Transaction data is read-only; settings and account connections are read-write. Support this flow in the local desktop app first, then in the cloud version. The local sharing mechanism, synchronization, and credential boundaries remain to be specified.

Local handoff carries API keys and manual imports (such as CSVs) and clones that portfolio's past transactions into the recipient's database. Cloud sharing uses the same database rows with shared access. OAuth authorization involves the account holder; the inviter cannot log into their OAuth accounts on their behalf. Ongoing local synchronization, secure handoff, and revocation behavior still need specification.

Local installations should eventually remain synchronized, including data imported through the recipient's OAuth connections. The manager receives that data from the recipient's installation rather than accessing their OAuth accounts. Continuous synchronization is later roadmap work; the initial clone/handoff precedes it.

## Homepage source material

Working positioning line: **Know what you own, what it earns, and where the same assets could earn more.**

The homepage can eventually develop four benefit stories: one portfolio view; earnings that reflect reward value and costs; better same-asset opportunities with explainable risks; and control over when funds become available. Privacy and local ownership support all four.

Before publishing, check each claim against shipped features and clearly label roadmap items. Do not imply guaranteed returns or liquidity, complete unverified history, automatic execution, or universal provider coverage. The [older-spec comparison](../findings/older-spec-comparison.md) records additional roadmap decisions and unresolved details.
