# User needs interview

Started 11 October 2026. This document records the user's answers as the interview proceeds, one question at a time. These are desired outcomes, not claims of implemented features. Export-derived model suggestions and provider priorities remain provisional until checked against these needs.

## Confirmed needs

Asked: What is the most important question the software should answer about your portfolio?

The user identified three questions:

1. “How much do I have across wallets / providers / platforms?”
2. “How much yield is it earning?”
3. “Should it be earning more?”

Design and provider work should serve these questions rather than reproduce every Airtable field. The user requested that follow-up questions be asked one by one.

The confirmed answers are distilled into [product questions, pain points, and proposed solutions](../spec/product-goals.md) for product planning and future homepage source material. This interview remains the evidence behind that brief.

### Investing horizon and financial coverage

The user clarified that this app is about long-term investing and agreed that opportunity alerts should avoid tiny gains. No specific minimum holding period or reallocation frequency has been selected.

Asked: After the holdings overview, which feature should be built first?

Answer: Received and expected earnings first, then opportunity comparisons, then withdrawal planning. This is the confirmed feature delivery order; the later money router follows the initial unlocked-on-platform planning stage. Exact release boundaries remain to be defined.

Asked: Should the first version track crypto and related fiat balances only, or also broader financial accounts?

Answer: Focus on crypto and related fiat balances for now. The plan is to soon include bank accounts, stocks, pensions, mortgages, and anything contributing to an individual's net worth. Bank and broker API integrations will be provided through a paid cloud API plan. Exact later-stage sequencing and the cloud API's personal-data/credential handling remain unspecified. This service is distinct from requiring the entire portfolio to be hosted in the secondary cloud app.

### Received and expected yield

Asked: Does yield mean income actually received, expected income at current rates, or both?

Answer: Both. Show yield actually received and the current annual earning rate of existing positions as separate views.

Asked: Should expected earnings follow current product reinvestment/renewal settings, with alternatives available for comparison?

This earlier answer was superseded by the user's clarification below; do not use renewal/reinvestment scenario forecasting as the definition of expected earnings.

User correction: Expected earnings are only what each investment position yields now, using “current yield rate * past returns for reward currency,” estimated over a year. The question answered is: “How much am I currently earning per year?”

Treat this as an annualized current-position earnings measure. Current reward valuation, observed effective reward performance, and advertised rate must be distinguishable so the adjustment can be explained. Opportunity comparisons retain their separate user-selected horizon, and withdrawal planning retains its stop-renewing-today default.

Asked: What historical period should the reward-currency adjustment use?

Answer: Since the investment position opened. The user defined the calculation as: if every reward had been kept from receipt until now, without selling any, how much of the earning/base asset would those rewards buy if swapped now? What cumulative effective percentage does that represent, and what APY is it equivalent to?

For different-currency rewards, retain the full reward receipt history, calculate hypothetical retained reward quantities, value their conversion into the earning asset at today's conversion rate, then show cumulative effective yield and an annualized APY equivalent over the position's elapsed lifetime. This hypothetical includes rewards actually sold; it is distinct from actual realized/retained earnings reporting. Keep the comparison with converting each reward into the earning asset on receipt available as previously requested.

The user further clarified that both rates should be shown:

- **Historical effective rate:** the annualized result of hypothetically retaining rewards since the position opened and converting them into the earning/base asset now. This reflects reward-token performance relative to the base asset, described by the user as the effect of inflation. It does not establish token supply inflation as the cause of relative price changes.
- **Immediate APY rate:** use the product's current yield rate, assuming each reward is sold for the earning/base asset immediately on receipt. Preserve this current-rate baseline alongside the historical retained-reward result rather than blending them into a single opaque rate.

Asked: Should immediate APY use the current product rate while historical effective yield uses actual past reward receipts?

Answer: Yes. Immediate APY is a current-rate measure; historical effective yield uses actual past reward receipts, hypothetically retained and valued in the base asset now, with invested capital and duration accounted for.

Asked: Should historical effective yield account for how much was invested and for how long when funds are added or withdrawn?

Answer: Yes. Account for deposits and withdrawals using invested amounts and their duration, rather than only the original deposit. Preserve dated capital changes separately from reward receipts and price changes. The exact cash-flow-aware calculation and annualization convention remain to be defined; no specific time-weighted or money-weighted methodology has been selected.

Swap-fee assumptions and behavior with incomplete history remain to be defined. Do not annualize an incomplete or unreconciled reward history as if complete.

Asked: What should the earnings dashboard's default reporting period be?

Answer: The last 12 months by default. Let users select this calendar year, all time, or a custom date range. Historical received earnings use the selected reporting period; that does not turn expected earnings into historical figures. The opportunity-comparison horizon is separately one year by default and user-selectable.

Asked: Should received earnings in USD use prices on receipt, today's prices, or both?

Answer: Default to today's prices. The user explained: “If not sold at price when received, then it's not earned.” The reporting intent is to avoid presenting receipt-date value as value realized or preserved when the reward was retained. Keep original reward quantities and receipt events available, while showing current value for retained rewards by default. This display preference is not a tax income-recognition rule.

Asked: Should rewards already sold use actual sale proceeds after fees rather than today's prices?

Answer: Yes; only the realized price counts for those rewards. Use actual realized proceeds after fees for the sold portion and current prices for the portion still held. Valuation evidence for non-fiat sale proceeds remains to be defined. Present reward proceeds and subsequent price gains/losses as reconciled views rather than adding both to a combined total and double counting value.

Asked: Does swapping a reward into another cryptocurrency count as realizing earnings for this report without cashing out to fiat?

Answer: Yes. Crypto-to-crypto swaps count as realization in the reward earnings report. Value that realization at the actual executed conversion, after fees; subsequent changes in the received asset must be kept distinct from the realized reward value. This is a reporting definition, not a jurisdiction-specific tax rule.

Asked: How should sales be attributed when purchased and earned units of the same token are mixed?

Answer: Let users choose and override the attribution rule, with oldest units first as the default. Preserve acquisition/reward identity and quantities so sold versus retained rewards and price gains can be explained under the selected rule. Supported alternative rules, override granularity, and whether attribution pools span accounts remain to be defined. This selects a portfolio-reporting default, not mandatory tax accounting.

Also show whether keeping reward assets consistently produced a negative return compared with selling them when received. The prior answer sets the hypothetical sale destination to the asset earning the reward and limits that comparison to different-currency rewards. The definition of “consistent,” comparison fees, and handling of partially sold rewards remain open.

Asked: Should cashback, signup bonuses, and promotional rewards count toward received yield or be shown separately from interest and staking rewards?

Answer: Show cashback, signup bonuses, and promotional rewards separately from interest and staking rewards, but include them in total earnings. Preserve these categories in source mapping and reporting rather than combining all credits into ordinary yield.

Asked: Should price gains/losses be separate from earnings, with realized and unrealized amounts split out?

Answer: Yes. Show price gains and losses separately from earnings, and distinguish realized from unrealized gains/losses. Reward receipt and subsequent price movement must remain distinct to avoid double counting. Cost-basis methodology and treatment of fees remain to be clarified; this reporting need does not by itself select jurisdiction-specific tax rules.

Asked: Should actual earnings show gross earnings, associated fees/borrowing costs, and net earnings separately?

Answer: Yes. Show gross earnings, fees and borrowing costs, and net earnings separately. Use net earnings as the main figure. Cost attribution to particular assets/products and distinction between earnings-related and other transaction fees remain to be defined.

### Value of retained rewards versus selling on receipt

The user also wants to know whether keeping reward coins has preserved, reduced, or increased the economic value of the yield, especially for coins that are or have been inflationary. Compare retained rewards with the hypothetical outcome of selling each reward when it was received. This should help answer whether selling rewards on receipt would have produced a better outcome over the observed period.

The subsequent answers establish a comparison of retained reward tokens against conversion into the earning asset on receipt. Token supply inflation and reward-token price changes are distinct measures; a fall in price alone does not establish inflation. A separate supply-dilution metric has not been requested.

Asked: What should each reward be sold into for the hypothetical comparison?

Answer: The asset that earned the reward. Compare keeping each reward token with converting it into the earning asset at receipt, then compare both outcomes at the same later valuation time. The baseline is not fiat or a stablecoin unless that is the earning asset.

Asked: Should same-coin rewards also be measured against token supply inflation or changes in fiat value?

Answer: No. This comparison is relevant only when rewards are paid in a different currency from the asset earning them. Same-coin rewards remain part of received and expected yield reporting, but do not need this additional comparison or a separate inflation/purchasing-power metric.

Later answers above establish a since-opening historical comparison and a separate current immediate APY. Exact annualization, hypothetical conversion costs, and tax treatment remain open. Selling rewards immediately is not assumed to guarantee the advertised yield.

### Opportunity catalogue for existing assets

Asked: Should earning more mean other opportunities for assets already held, or also switching to different assets?

Answer: Other opportunities for assets already held. The user wants to compare earning more on the same asset with another provider or product, rather than recommendations to change the asset allocation.

Maintain a catalogue of opportunities per asset with a list of risks associated with each opportunity and platform. The user asked how banks and hedge funds measure risk to present risk-adjusted rewards. Research and proposed application are recorded in [risk comparison findings](risk-comparison.md); no scoring methodology has yet been chosen by the user.

Asked: Should opportunities be compared using headline APY or estimated earnings on the user's actual balance after costs and restrictions?

Answer: Compare estimated earnings on the actual balance after fees, rate tiers, caps, and switching costs. Show headline APY as well, but use net earnings to compare opportunities. Rate-change assumptions, reward conversion assumptions, and treatment of taxes remain to be defined.

Asked: Should comparisons include opportunities requiring platform-token purchases, memberships, or other qualification conditions?

Answer: Yes. Clearly show qualification requirements, extra costs, and risks. Account for those costs in net earnings, and let users filter out opportunities requiring extra tokens or memberships. These qualifications are optional conditions for earning on existing assets, not a general mandate to recommend changing asset allocation. Treatment of recoverable token capital versus nonrecoverable fees, existing qualifying holdings, and token price scenarios remains to be defined.

Asked: Over what period should net earnings be compared by default?

Answer: One year by default, with the option to choose a different period. Account for switching costs over that selected period. Do not treat a one-off switching cost as a recurring annual cost automatically.

Asked: Should the software suggest splitting an asset across multiple opportunities to use better rate tiers and reduce concentration, or compare one destination at a time?

Answer: Yes to suggesting splits across opportunities. Comparisons may propose multiple destinations for the same held asset to use rate tiers/caps and reduce concentration, rather than considering only a single destination. This confirms allocation suggestions, not automatic transfers or execution. Allocation constraints and treatment of shared platform dependencies remain to be defined.

### Liquidity constraints, term staggering, and withdrawal timeline

Asked: Should users specify how much of each asset must remain immediately withdrawable?

Answer: Yes. Let users set an immediately withdrawable minimum for each asset and only suggest locking up the remainder. Allocation suggestions must respect that requirement.

For repeating fixed-term products such as Nexo, propose splitting locked holdings across several terms so that more becomes withdrawable sooner or more frequently. Suggestions should account for staggered maturity dates rather than optimizing yield alone. Automatic renewal assumptions, starting schedules, minimum deposits, and supported term lengths remain to be clarified.

Provide a graph of how much becomes withdrawable over time, combining holdings across all platforms/products simultaneously. It must answer:

- How long until 100% is withdrawable?
- How long until 50% is withdrawable?
- How long until a specific requested amount is withdrawable?

Asked: Should the withdrawal timeline show individual asset units, combined reporting-currency value, or both?

Answer: Combined value in the user's reporting currency. Aggregate availability across all platforms/products on that valuation basis; express requested withdrawal amounts and percentage thresholds consistently with it.

Asked: What should the default reporting currency be?

Answer: USD for now. Users can change the reporting currency in settings. The initial set of supported alternatives has not yet been specified.

Asked: Should the default timeline stop renewing fixed terms from today or follow current renewal settings until the user chooses to stop?

Answer: By default, assume the user stops renewing fixed terms from today. Model existing terms through their available exit/maturity dates rather than assuming they can be unlocked immediately. This is a planning assumption, not an instruction to change provider renewal settings automatically.

Provide another mode: “I need this much by this date; when should I withdraw what?” Accept a target amount in the reporting currency and a deadline, then produce a cross-platform/product withdrawal schedule showing amounts, sources, and timing. How to choose between feasible schedules, whether to preserve yield on funds not needed, and treatment of conversions or transfers remain open.

Asked: Does the target mean funds unlocked on platforms, or converted and delivered to a chosen wallet or bank account?

Answer: Both, in stages. Start with funds unlocked on their platforms. Later, add planning for funds converted and delivered to a chosen wallet or bank account, using the money router described below. These are distinct completion conditions; unlocked funds must not be presented as already delivered.

Asked: Should the withdrawal timeline hold today's prices constant or forecast future prices?

Answer: Hold today's prices constant. When the user needs a specific amount, suggest adding a buffer. The buffer size and how it is selected remain to be defined. A reporting-currency projection at today's prices must not be presented as a guarantee of the amount available at the deadline.

The treatment of future rewards, withdrawal delays, and uncertain/frozen availability remains open. This is a desired feature, not a claim of guaranteed withdrawal availability.

### Collateral-backed debt and withdrawals

Asked: Should withdrawal planning show what is withdrawable now and what becomes available after repaying debt, including repayment costs?

Answer: Yes. Show both views, account for repayment costs, and retain enough collateral to avoid liquidation. Plans must respect collateral requirements rather than treat all collateral as freely withdrawable. Released collateral is existing owned value becoming available, not newly earned value; repayment consumes funds and must not be counted twice. Liquidation safety margins, stressed-price assumptions, provider rules, and repayment funding choices remain to be defined. A plan at current prices cannot guarantee protection against future price moves.

### Failed-platform funds and recoveries

Asked: Should frozen funds on a failed platform use the original balance, estimated recovery value, or be excluded and shown separately?

Answer: Show them as zero/loss until recovered. Do not include hypothetical future recoveries as positive net worth. Keep the original balance/claim and recovery evidence visible separately so the loss and subsequent recoveries can be explained. Recognition dates, recovery classification, and partial-recovery handling remain to be defined; this valuation preference does not select a tax loss treatment.

### Current provider priorities, wallet coverage, and history

Asked: How far back should portfolio and received-yield history go?

Answer: From the first activity on each platform. Aim to reconstruct portfolio and received-yield history from inception, not only from connection time or a recent API window. Source availability may limit reconstruction.

Asked: When APIs cannot provide complete history, are CSV/statement imports and manual entry acceptable?

Answer: Yes, if there is no choice. Prefer automatic acquisition; use old CSVs/statements and manual entry when necessary to fill unavailable history. Provide direct links to each platform's export page to make obtaining CSVs easier. Platform-specific URLs, formats, and export instructions must be verified when that integration is implemented; none is assumed from this answer.

Asked: If history remains incomplete, should affected historical totals and yield figures be shown as partial results or withheld until reconciled?

Answer: Withhold them until reconciled. Keep the provider line visible with a warning icon. Do not silently treat unavailable figures as zero or present partial history as a reconciled result.

Asked: Should verified current provider balances still be shown while historical totals and yield figures are withheld?

Answer: Yes. Show verified current balances, retain the provider warning icon, and withhold historical totals and yield figures until reconciled. Current-balance verification does not establish historical completeness.

Asked: If one provider's history is unreconciled, should the combined historical total be withheld or limited to reconciled providers?

Answer: Show a clearly labelled total for reconciled providers only. Identify omitted providers; do not present this subtotal as the complete historical portfolio total or substitute zero for unreconciled amounts.

Asked: Which providers or wallets are actively used, and which should be supported after Nexo and Binance?

Answer: Kraken, MetaMask, self-hosted wallets on Solana and NEAR, and a few smart contracts. The user directed us to the Airtable exports for additional context.

Asked: Which integration should follow Nexo and Binance?

Answer: Kraken first, then EVM wallets, Solana, and NEAR, in that order.

Treat these as confirmed current-use priorities. MetaMask is a wallet interface; its addresses and networks determine ingestion scope. Solana and NEAR require chain/account coverage independent of the wallet application. Exact addresses and currently held contract positions remain to be identified.

Asked: Which networks are currently used through MetaMask?

Answer: Ethereum, Polygon, BNB Chain, Arbitrum, and zkSync, plus Solana coverage. The user reiterated that the Airtable exports should guide which wallets and positions to cover. NEAR remains in scope from the earlier answer; it was not withdrawn. Use exported wallets/positions as coverage inputs, while confirmed user needs determine which features and relationships to model.

Existing [Airtable findings](airtable-model.md) and [the earlier roadmap](../todo/provider-roadmap.md) identify QuickSwap, Alpha Homora/ibETH, MetaPool, Lido, Rocket Pool, and other protocol entries as leads. These do not establish which contracts are currently used. Earlier rankings inferred from exports must not override the user's named priorities.

Asked: Should adding a wallet automatically discover its assets and DeFi positions, or track only selected holdings?

Answer: Automatically discover assets and DeFi positions. Let the user hide spam tokens and exclude anything they do not want to track. Discovery must coexist with user controls rather than require manually listing every position.

Asked: Should automatic wallet discovery use third-party indexers, user node/RPC endpoints, or both?

Answer: Use default public indexers for each chain, with settings to override RPC nodes. Specific default services and endpoint configuration remain to be selected. An RPC override is not assumed to replace all indexer functionality; how acquisition uses the configured endpoints needs a defined implementation contract.

Asked: Should hidden spam tokens be excluded from portfolio totals and yield calculations, or only hidden from the holdings list?

Answer: Exclude them from portfolio totals and yield calculations too. Anything explicitly excluded from tracking must not affect those numbers. Exclusions are calculation controls, not merely presentation filters. Their effect on historical reports and any retained source evidence remains to be clarified.

Asked: Should DeFi reporting show both the position and its underlying asset exposure without double counting?

Answer: Yes. Show the DeFi position and its underlying asset exposure, count the economic value only once in portfolio totals, and let the user drill down into underlying assets. Underlying components are an exposure view of the position, not additional holdings to add to its value. Protocol-specific share conversions, changing composition, and treatment of nested positions remain implementation details to resolve.

### Later-stage money router

The user wants a money router that maps possible connections between blockchains and platforms and their fees, and tries to minimize fees when moving money. This is a later capability that will support converted-and-delivered target-date planning. Route coverage, fee freshness, conversion costs, timing, capacity, eligibility, route risks, and whether execution is ever supported remain to be clarified.

### Personal portfolio storage

Asked: Should personal portfolio data remain on the user's device, use a hosted account, or support both?

Answer: Local first. Provide a secondary cloud version for users who prefer hosted access and do not prioritize privacy in the same way. Local storage is the primary portfolio experience; cloud portfolio hosting is an alternative, not required to use the local version. Cloud timing, synchronization, account/credential handling, and access across devices remain unspecified.

This concerns personal portfolio storage, distinct from the public opportunity catalogue/dataset and the paid risk-rating API. Remote acquisition and API use still have their own privacy implications; the answer does not specify those boundaries yet.

Asked: Should local portfolio tracking and earnings analysis work without a paid subscription?

Answer: Yes. Local tracking and earnings analysis must work without a subscription. Paid APIs add risk ratings, provider dependency maps, and bank/broker integrations. Users can supply their own API keys to sync through free provider/integration APIs without subscribing to our service; third-party access requirements remain provider-specific.

The cloud version makes connections easier: one login, cloud-managed API keys and provider syncing, and provider OAuth connections where available. This convenience service is distinct from the local bring-your-own-key path. Cloud pricing, OAuth scopes, key handling, and whether cloud syncing can feed a local portfolio remain to be defined.

### Application interface and technology sequence

Asked: Should the local-first portfolio interface be a desktop app, a locally running browser dashboard, or both?

Answer: First pass: a desktop app using Tauri, with a frontend shared with the web version using SolidJS, Panda CSS, and Ark UI. Later: a desktop app in Rust using GPUI.

These are selected future directions, not claims of delivered interfaces. The later GPUI transition does not establish its timing or whether the Tauri desktop app will remain supported. Shared domain behavior should be preserved across interfaces; exact frontend/backend boundaries remain to be designed.

Asked: Should the first desktop version only track holdings and suggest actions, or also execute transfers, staking, and renewal changes?

Answer: Track holdings and suggest actions only in the first version. Consider executing transfers, staking, and renewal changes later. First-version allocation, withdrawal, and term-staggering plans are suggestions for the user to carry out, not automatic provider actions. Execution is not a committed later requirement.

### Refresh behavior

Asked: Should the desktop app refresh balances/rates automatically while running, with manual refresh available?

Answer: Yes. Refresh balances and rates automatically while the app is running, and let users configure the interval. Manual refresh belongs only in options. The app is not time sensitive: long-term investing does not require a prominent refresh button or a real-time trading experience. Default interval, retry behavior, and treatment of stale data remain to be defined.

### Opt-in notifications

Asked: Should the first version proactively notify users about maturities, renewal deadlines, and better yield opportunities, or show these only when the app is opened?

Answer: Support proactive notifications for maturities, renewal deadlines, and better yield opportunities. Let users configure which notifications they want. Notifications are off by default; suggest that users set them up and explicitly opt in.

Asked: Should better-yield alerts support minimum improvements in net dollars earned, percentage yield, or both?

Answer: Both. Let users set minimum improvements in net dollars earned and percentage yield. Include switching costs over their chosen comparison period, consistent with opportunity comparisons. Whether both thresholds must be met when both are configured, the percentage definition, handling of non-USD reporting currencies, notification channels, lead times, repetition limits, and background operation remain to be defined.

This is a product requirement, not a request to create reminders or recurring automations in this chat.

### Risk ratings and preferences

Asked: Should opportunities show separate risk ratings with explanations, one overall score, or both?

Answer: Both. Show an overall risk score alongside separate risk ratings with explanations. The scoring scale, components, weights, evidence requirements, and treatment of unknowns remain to be defined; selecting both views does not select a scoring methodology.

Asked: Should ratings use our own methodology, third-party ratings, or our methodology informed by third-party evidence?

Answer: Our own documented methodology, informed by third-party evidence. Keep sources and reasoning behind each rating traceable, with human review. Third-party evidence informs our assessment rather than replacing it with an externally supplied score. The numerical scale, risk components, aggregation rules, review process, and methodology-version handling remain to be designed.

Asked: Should allocation suggestions respect maximum exposure percentages per platform, protocol, and blockchain, in addition to individual risk-rating limits?

Answer: Yes. Let users set maximum exposure percentages per platform, protocol, and blockchain, and account for those limits in suggested allocations. The denominator for these limits and treatment of nested or overlapping exposures remain to be defined.

Include provider and asset concentration as risks to the overall portfolio. Shared opportunity/platform ratings remain the same for everyone; portfolio concentration depends on the user's holdings and is a separate portfolio-level assessment.

Risk analysis must map relationships between providers to account for domino effects: one provider's failure may cause another to fail and propagate through a chain of dependencies. Splitting holdings across nominally different providers must not automatically be treated as independent diversification. Relationship types, evidence, direction, exposure size, uncertainty, publication boundaries, and stress-scenario methodology remain to be defined. No cascade probabilities or numerical losses are assumed from the request alone.

Asked: Should the provider dependency map be public or reserved for the paid API?

Answer: Paid API. Provider relationship/dependency mapping is paid content alongside risk ratings, rather than part of the public catalogue's risk summaries. How public summaries refer to dependencies without publishing the paid map remains to be defined.

Asked: Should users be able to run provider-failure scenarios showing direct exposure and possible knock-on effects?

Answer: Yes, but much later on the roadmap. Preserve this as a later capability, not a prerequisite for initial holdings, earnings, opportunity comparisons, or withdrawal planning. Scenario assumptions and presentation remain to be defined.

### Eligibility and public/private access

Asked: Should the public opportunity catalogue be manually curated, automatically collected, or both with human review?

Answer: Both, with human review. Automatically collect opportunities and rates from provider APIs and websites where possible, and allow maintainers to add or correct entries manually. The review/publication workflow, update cadence, source precedence, and handling of conflicting or stale rates remain to be defined.

Asked: Should opportunities with missing or stale rates/risk assessments remain visible with warnings but be excluded from allocation suggestions until verified?

Answer: Yes. Keep them visible with clear warnings, but exclude them from allocation suggestions until both rates and risk assessments are verified and up to date. This is distinct from known risks exceeding user limits, which remain visible with warnings by default and can be filtered out. Freshness thresholds and verification criteria remain to be defined.

Asked: Should the catalogue show all opportunities or only those the user is eligible to use?

Answer: Show all opportunities, clearly indicate eligibility, and allow filtering to eligible opportunities. Country, account status, and investment amount were examples in the question; the exact eligibility rules and required user information remain to be defined.

The opportunities catalogue, yield information, and risk summaries should be public through both a website and an accessible dataset. Risk ratings remain private, including the overall score and separate risk ratings requested above. Public risk summaries and private ratings are distinct outputs.

Asked: Who should be able to see the private risk ratings?

Answer: Risk ratings will be part of the paid offering, delivered via API. Here, private means restricted paid access rather than necessarily personal ratings visible only to their author. Authentication, paid entitlements, permitted use/redistribution, and any website presentation of paid ratings remain unspecified.

Asked: Should risk ratings be the same for every customer or personalized?

Answer: Risk ratings are the same for everyone. Risk tolerance adjusts how those ratings are taken into account, not the ratings themselves. Keep the shared assessment separate from user-specific preferences and opportunity comparison. Whether tolerance affects filtering, ordering, weighting, or another presentation rule remains open.

Asked: How should users express their risk tolerance?

Answer: Both a simple conservative-to-aggressive setting and the option to set limits for individual risks. The presets, risk categories, limit units, and relationship between the general setting and individual limits remain to be defined.

Asked: How should opportunities exceeding a user's risk limits be presented?

Answer: Show them with a warning by default. Let users filter out opportunities that exceed their risk limits. Exceeding a limit must not silently hide an opportunity in the default view; filtering is an explicit user choice. The warning should identify the exceeded limits. Behavior when an assessment is unknown remains to be clarified.

These are desired publication boundaries, not authorization to publish current documentation, personal holdings, account information, or exports. Dataset format, access mechanism, update cadence, and licensing are still open.

## Follow-up on the older product draft

The user set the following scope and timing:

- Tax reporting: later, as a cloud service that syncs from local data. Transaction-document workflows and tax jurisdictions remain unspecified.
- Spending tools: yes, later. Exact categorization/subscription features remain to be scoped.
- Shared portfolios: yes, soon after single-user tracking. Prioritize family/client portfolio management, then couples/groups/households, then companies. Ownership, permissions, and sharing mechanisms remain to be defined.
- Open source: from the start. License and scope across app/service components remain to be specified.
- Community integrations: later.
- Mobile and sync: later. Backup, restore, and migration details remain open.
- Historical dashboard graphs: part of the initial plan. Include buy-and-hold comparison early, alongside historical performance graphs: compare against simply holding the same assets, accounting for fees and deposits/withdrawals. This is separate from the previously confirmed reward-retention comparison. Exact benchmark construction remains to be defined.
- Affiliate monetization: after paid cloud/API plans are live. Placement and independence rules remain to be defined.
- Invoicing: much later.

## Shared portfolio priorities

Asked: Which shared use case comes first: couples/households, companies, or managing portfolios for family/clients?

Answer: Family/clients first, then couples/groups/households, then companies. Shared access does not itself establish joint ownership or justify combining separate clients' net worth. Exact account boundaries, aggregate views, roles, and permissions remain to be defined.

Asked: Should managers be read-only by default, with owners granting edit permissions?

Answer: The user wants to add a friend/family member's accounts and track their portfolio on their behalf. They should be able to share a preconfigured app/invitation so the recipient signs up and finds their existing data already available. After the recipient joins, the inviter retains shared admin rights unless the recipient explicitly removes them.

Transaction data is read-only in this shared-user workflow. Settings and account connections are read-write. Do not reduce the inviter to read-only automatically on acceptance, and do not interpret admin access as authority to execute financial transactions. Local/cloud delivery, identity/ownership transfer, credential handling, sync, revocation effects, and interaction with earlier manual-history/attribution controls remain to be designed.

Asked: Should the preconfigured invitation flow work locally, in the cloud, or both?

Answer: Both. Start with the local desktop app, then support the same invitation flow in the cloud version. Shared admin access and recipient removal must work consistently with that sequence. How local devices exchange data and enforce permissions is still to be designed; do not assume a one-off export alone provides continuing shared administration.

Asked: Should invitations carry working provider connections or require the recipient to reconnect accounts needing API keys/OAuth?

Answer: The inviter should not be able to log into another person's OAuth accounts. Carry over API keys and manual imports such as CSVs. For local desktop use, clone the relevant portfolio's past transactions from the inviter's database into the recipient's database. For cloud use, share access to the same database rows rather than cloning them per user. OAuth authorization requires the account holder's involvement; do not imply the inviter can impersonate them.

API-key/CSV handoff packaging, encryption, selection of portfolio records, and ongoing local synchronization remain to be specified. The user authorized this as a product design requirement, not a request to transfer actual credentials in this chat.

Asked: After local handoff, should installations stay synchronized or use independent copies/manual updates?

Answer: Stay synchronized, but synchronization comes later on the roadmap. If the recipient adds OAuth connections, the manager receives/clones the resulting data from the recipient because they cannot access the recipient's OAuth accounts. Synchronize imported portfolio data rather than granting the manager OAuth login access. The initial handoff and later continuous synchronization are separate delivery stages; do not claim automatic cross-device updates before sync is implemented.

Asked: Should encrypted portfolio backup and restore be in the first local desktop version or later?

Answer: Later. Do not make an encrypted backup/restore feature a first-version requirement. Export/handoff behavior is a separate confirmed capability; backup scheduling, restore semantics, and key recovery remain unspecified.

## Still to clarify

### Buy-and-hold benchmark definition

Asked: Should the buy-and-hold baseline earn no staking/lending yield and match external deposits/withdrawals on the same dates?

Answer: Yes. The baseline earns no staking or lending yield and matches the user's external deposits and withdrawals on the same dates. Transfers between tracked owned accounts are internal movements, not additional portfolio contributions. Baseline starting assets, mapping of contributions/withdrawals to assets, handling of actual trades, and benchmark fee assumptions remain to be specified.

Exact yield calculation conventions, comparison cost assumptions, target-date withdrawal planning and buffer rules, later-stage routing details, term-staggering constraints, shared risk scoring methodology, paid API access details, eligibility rules, public dataset delivery, alternative reporting currencies, and exact contract coverage remain open. The major yield definitions, risk presentation, chain coverage, and integration order have been confirmed above; do not treat them as unanswered.

The [older-spec comparison and follow-up decisions](older-spec-comparison.md) distinguish confirmed timing above from remaining details. Preserve this distinction when updating implementation tasks.
