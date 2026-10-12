# Product questions, pain points, and solutions

Updated 12 October 2026, distilled from the [user needs interview](../findings/user-needs.md), [desktop interview and follow-up decisions](desktop-app.md), and [older-spec comparison](../findings/older-spec-comparison.md). This is a proposed product brief and source material for a future homepage. It describes desired outcomes, not a list of features already shipped. The [project context](project.md) and topic specifications describe the current implementation.

## Product purpose

Help long-term investors understand what they own, what it earns, and whether the same assets could earn more elsewhere, while retaining control of their financial data.

The three central questions are:

1. **How much do I have across wallets, providers, and platforms?**
2. **How much yield is it earning?**
3. **Should it be earning more?**

Start with crypto and related fiat balances. Extend to banks, stocks, pensions, mortgages, and other personal assets and liabilities. The user-confirmed feature sequence is holdings overview, received/current annual earnings, opportunity comparisons, then withdrawal planning.

## Who we are building for

Start with long-term investors whose crypto and related fiat are scattered across exchanges, lending/staking products, wallets and DeFi positions. They currently use spreadsheets or separate provider dashboards to piece together their holdings and earnings. The app should make this a repeatable view of their money, rather than another trading screen to monitor constantly.

Next serve people managing portfolios for family, friends or clients. Broader household/company use and bank/broker/pension/mortgage coverage follow the confirmed roadmap. These are interview-derived audiences and needs, not market-size or customer-validation claims.

## Questions the software should answer

### Holdings and trustworthy totals

- What do I own across my accounts and wallets, and what is it worth in my reporting currency?
- What is my net worth after liabilities?
- What do my DeFi positions expose me to underneath their receipt tokens or pool shares?
- Which holdings or reports are verified, and which providers still need reconciliation?
- What was lost on failed platforms, and what has actually been recovered?
- Which numbers are known already, which are partial, and what information is missing?
- Where is each asset allocated across providers, accounts, products and individual subscriptions?
- Which small holdings are only dust, and which discovered tokens should be excluded as spam?

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
- Is the strategy doing better than simply holding the same assets with the same deposits and withdrawals?
- Are apparent gains investment returns, additional deposits, or transfers between my own accounts?

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

### Managing portfolios for other people

- Can I prepare and track a family member's, friend's or client's accounts before they start using the app?
- Can they join and find their existing records already there, without repeating setup/imports?
- Can we both manage settings and connections, while they retain the ability to remove my access?
- Can I see data imported through their OAuth connections without logging into their provider accounts?

Start with a local preconfigured invitation/handoff, followed by the cloud flow. Keep each person's portfolio identity distinct. Local handoff clones relevant history and carries supported API-key/manual-import inputs; cloud sharing grants access to the same rows. Continued cross-device synchronization comes later. Shared administration does not authorize financial execution or access to another person's OAuth login.

### Control, corrections and a calmer interface

- Can I use local tracking without a subscription or uploading my whole portfolio?
- Can I keep tracking imported records offline and use my own provider keys?
- Can I correct a missing price/rate or provider name, protect an override, and undo a mistake?
- Can I hide amounts or show scaled-down proportional amounts when demonstrating the app?
- Can I find the relevant asset/provider quickly without scrolling through spam and tiny holdings?
- Can I receive useful maturity or opportunity alerts without being interrupted for tiny gains?

The confirmed design uses one portfolio initially, an Overview and asset/provider navigation, allocation drill-down, searchable lists and row-scoped warnings. Local names/groupings, dated manual price/rate overrides, pinning and persistent Undo are requested correction tools. The latest update wins unless the override is pinned. Spam is excluded globally; dust is grouped per holding without changing portfolio value. Hidden and scaled-down privacy modes alter presentation only, preserving percentage rates and proportional relationships. Notifications are opt-in and configurable, with minimum improvements in net dollars and percentage yield after switching costs.

## Pain points and our proposed solutions

These are product hypotheses supported by the recorded answers, not findings from broader customer research. “Stage” describes intended delivery order, not shipped status; the readiness table below distinguishes what exists today.

| Pain point | Proposed solution | Stage |
| --- | --- | --- |
| Assets are scattered across platforms and wallets. | Unified holdings and allocation drill-down, by asset or provider. | First: holdings |
| Adding balances can overstate net worth by ignoring debt or counting counterparties as owned accounts. | Assets minus liabilities, with debt and borrowing costs explicit and external counterparties outside the portfolio. | Foundations |
| Wallet discovery produces clutter from spam and tiny balances. | Exclude spam from calculations; group dust without dropping its known value or hiding a meaningful balance elsewhere. | Holdings |
| Tracking requires repetitive spreadsheet updates. | Automatic provider sync and wallet discovery; original CSV imports/manual input only when needed, with export-page links. | Foundations |
| Incomplete history creates convincing but unreliable numbers. | Reconciliation warnings, withheld affected history, and explicit reconciled-only subtotals. | Foundations |
| DeFi receipt tokens obscure underlying exposure or get counted twice. | Position and underlying views with drill-down and single-count valuation. | Holdings |
| Advertised APY does not explain what rewards are worth after being held. | Current immediate APY beside cash-flow-aware historical effective yield in the earning asset. | Earnings |
| Deposits and internal transfers can look like investment performance. | Separate external cash flows from returns and compare against a matching no-yield buy-and-hold baseline. | Historical earnings/performance |
| Fees, debt costs, bonuses, and price changes obscure earnings. | Net earnings first, with costs, reward categories, and realized/unrealized price changes separately explained. | Earnings |
| It is hard to compare rates that depend on tiers, caps, or memberships. | Actual-balance net earnings comparisons including qualification and switching costs. | Opportunities |
| A higher yield may add risks or conceal common dependencies. | Traceable product/platform ratings, portfolio concentration analysis, and a provider dependency map. | Opportunities; paid risk data |
| Risk preferences differ even when the underlying assessment is shared. | Common ratings with personal tolerance, specific risk limits, warnings, and filters. | Opportunities |
| Locking everything into one term makes funds hard to access. | Per-asset liquid reserves, staggered terms, and an aggregate withdrawal timeline. | Withdrawal planning |
| A deadline requires coordinating maturities and debt across providers. | Target-amount/date plans including repayment and collateral constraints; routing/delivery later. | Withdrawal planning; routing later |
| Relying on a hosted portfolio service limits control over personal records. | Subscription-free local tracking/earnings with user-supplied provider keys; optional managed cloud convenience. | Primary product model |
| Provider labels, missing rates and wrong prices make the overview hard to use. | Local names/groupings, dated price/rate corrections, protected pins and persistent Undo, with source evidence retained. | Corrections/controls |
| Showing a portfolio reveals sensitive balances. | Hidden and proportionally scaled-down presentation modes without changing calculations. | First UI |
| Helping family or clients requires rebuilding their setup and switching between disconnected records. | Prepare their portfolio, invite them to existing data, retain revocable shared administration, and synchronize later. | Sharing, soon after single-user tracking |
| Important deadlines/improvements are easy to miss, while tiny gains can create notification noise. | Notifications off by default; opt-in maturity/renewal alerts and meaningful net-dollar/percentage improvement thresholds after switching costs. | With relevant product capabilities |

## Public and paid product boundaries

The planned opportunity catalogue, yields, and descriptive risk summaries will be public through a website and accessible dataset. Collect from provider APIs/websites where possible, with human review and manual additions/corrections.

Risk ratings and provider dependency maps are planned as paid API content. Bank/broker integrations are also planned through a paid cloud API offering. Local holdings and earnings remain usable without a subscription, including free provider integrations using users' own keys. The planned secondary cloud version offers one login, managed keys/sync, and OAuth where available.

## Other confirmed roadmap topics

Open source starts with the app; license and component scope remain to be specified. Shared portfolios follow soon after single-user tracking: family/client management first, then couples/groups/households, then companies. Tax reporting comes later as a cloud service that syncs local data. Spending tools, community integrations, and mobile/sync are later additions. Affiliate monetization follows live paid cloud/API plans; invoicing is much later. Exact sharing, tax/document, offline/sync, and business-model details remain open.

For family/client management, let a manager add and track another person's accounts before inviting them to a preconfigured app. On signup, the recipient should find their data already available. The inviter retains shared admin rights unless the recipient explicitly removes them. Transaction data is read-only; settings and account connections are read-write. Support this flow in the local desktop app first, then in the cloud version. The local sharing mechanism, synchronization, and credential boundaries remain to be specified.

Local handoff carries API keys and manual imports (such as CSVs) and clones that portfolio's past transactions into the recipient's database. Cloud sharing uses the same database rows with shared access. OAuth authorization involves the account holder; the inviter cannot log into their OAuth accounts on their behalf. Ongoing local synchronization, secure handoff, and revocation behavior still need specification.

Local installations should eventually remain synchronized, including data imported through the recipient's OAuth connections. The manager receives that data from the recipient's installation rather than accessing their OAuth accounts. Continuous synchronization is later roadmap work; the initial clone/handoff precedes it.

## Homepage source material

Working positioning line: **Know what you own, what it earns, and where the same assets could earn more.**

Working supporting line: **Bring scattered balances together, understand your earnings after costs, and plan around risk and access to your money—with a local-first portfolio.** This describes the product direction; use narrower copy for the current release.

The homepage can use the three original questions as its narrative. The benefit stories below translate the interview into copy themes; they are provisional wording, not approved published claims.

| Benefit story | User's question | Pain to describe | Solution to explain |
| --- | --- | --- | --- |
| See your whole portfolio | “How much do I have?” | Separate dashboards and spreadsheets hide the overall picture. | One overview, holdings by asset/provider and allocation drill-down, with liabilities deducted. |
| Understand what your money earns | “How much yield is it earning?” | Headline rates, reward-token price changes and costs tell different stories. | Received earnings, current annual earning rate, net costs and reward-versus-base-asset comparisons kept distinct. |
| Compare better uses for the assets you hold | “Should it be earning more?” | The highest advertised APY may not be the best result for your balance or risk limits. | Same-asset net-earnings comparisons, qualification/fee details and explained risks. |
| Know when your money is available | “How much can I withdraw, and when?” | Terms, renewals and collateral are spread across platforms. | One withdrawal timeline, liquid-reserve constraints, staggered terms and target-date plans. |
| Keep control of your records | “Can I use this without giving up my data?” | Mandatory hosting/subscriptions and opaque numbers reduce control. | Local-first storage, cached records, source evidence, corrections and an optional cloud experience. |
| Help someone start with their records ready | “Can I manage this for my family or clients?” | Repeated setup and imports make shared tracking cumbersome. | Preconfigured invitations, distinct portfolios and revocable shared administration. |

Use simple user language on the homepage: “what you own,” “what it earns,” “after fees,” “available to withdraw,” and “what is missing.” Explain APY and reconciliation in supporting content rather than leading with database/provider terminology. Avoid implying that all people have every pain point: these stories come from this product interview.

### Claim readiness as of 12 October 2026

| Area | Grounded current claim | Planned benefit that must be labelled as upcoming |
| --- | --- | --- |
| Portfolio overview | The desktop app displays already-imported current holdings, allocation trees, liabilities and known/partial values with warnings. Asset/provider navigation and search are available. | Complete provider/product coverage, UI account connections/imports, DeFi underlying exposure drill-down and broader personal net worth. |
| Valuation and trust | Explicit USD current-price refresh, dated exact price evidence, cached views and descending value sorting are implemented. Provider adapters have source-specific reconciliation and documented limits. | Continuous background acquisition, universal valuation and complete reconciled history for every account. A current price does not verify a balance. |
| Current earnings | Rust queries calculate current earnings from available rate/price evidence and evidenced borrowing costs. A separate labelled aggregate APY estimate assumes missing investment rates are zero; individual unknowns and warnings remain. | Fully populated rates, complete fee/tier/cap adjustment, historical received earnings, reward-retention analysis and buy-and-hold charts. A 0% estimate must not be sold as a verified 0% rate. |
| Privacy and corrections | Local storage, saved settings and hidden/scaled-down privacy modes are in the app. Metadata, overrides and persistent Undo have backend foundations. | Full manual-edit/Undo UI, all grouping customization, backup/sync and cloud convenience. Privacy modes hide presentation; they do not make external price/indexer requests anonymous. |
| Opportunities and risk | The requirements and publication/paid boundaries are documented. | The public catalogue/dataset, eligible comparisons, risk ratings, provider dependency maps and allocation suggestions. |
| Liquidity and sharing | Available imported lock/maturity/renewal metadata can be shown in details. Sharing and withdrawal requirements are documented. | Combined withdrawal graphs, term ladders, deadline planning, routing, invitations and shared administration. |

Supporting evidence: [desktop delivery findings](../findings/desktop-stage-6.md), [portfolio query contract](portfolio-queries.md), [UI metadata contract](ui-metadata.md), and provider specs linked from [the documentation index](../README.md). [The master todo](../todo/master-todo.md) owns delivery priorities; this brief does not create a competing checklist.

Before publishing, check each claim against shipped behavior. Do not promise guaranteed returns/liquidity, complete unverified history, automatic execution, anonymous remote API use, or universal provider coverage. Preserve the public catalogue versus paid ratings distinction, and the subscription-free local experience versus optional paid cloud/API services. Open-source licensing/component scope still needs specification. Roadmap copy should describe intended outcomes, not imply that every capability is available today.
