# First desktop app — scope and implementation

Status: stages 1–6 are implemented, including the local Tauri/Solid portfolio browser. Financial edit/Undo controls, provider refresh and cross-platform packaging remain stages 7–9. The interview below retains the full agreed first-version scope; it is not a claim that every scenario is delivered. See [stage 6 behavior and limitations](../findings/desktop-stage-6.md).

## Priority decision

The user has prioritized building a first app/UI ahead of further connector expansion. [The master todo, section 0](../todo/master-todo.md#0-now-first-usable-desktop-app--ui), owns the immediate delivery tasks. Reuse the implemented core and provider foundations; make narrowly scoped supporting changes as needed. Remaining EVM reconciliation blockers stay visible rather than being hidden to make the UI appear complete.

## Previously confirmed requirements

These come from [the user interview](../findings/user-needs.md), [product goals](product-goals.md) and [project context](project.md). They constrain the design but do not settle which screens fit the first build.

- Desktop first: Tauri, with a SolidJS frontend shared with the future web version. Panda CSS + Ark UI in a separate TypeScript design-system package. Rust + GPUI is later.
- Local portfolio storage and offline cached use; no subscription required for local tracking/earnings. Cloud, mobile and continuous sync are later.
- Initial financial coverage: crypto and related fiat. Show holdings across providers/wallets, asset/liability separation and net worth; earnings then opportunity comparisons then withdrawal planning follow the established product order.
- USD reporting currency by default, changeable in settings. Historical net-worth/performance graphs and buy-and-hold comparison are early goals, with missing calculation capabilities labelled honestly.
- Verified current balances can remain visible for unreconciled providers with warnings. Withhold their historical totals/yield; label reconciled-provider-only historical subtotals explicitly. Unknown or missing amounts/prices never become zero by accident.
- Reviewed exclusions affect totals and earnings eligibility while preserving source history. Use [the implemented policy](tracking-exclusions.md).
- Automatic balance/rate refresh while running, configurable interval; manual refresh belongs in options. This is a long-term investing interface.
- Track and suggest actions; transfer/staking/renewal execution is outside the first-version scope. Notifications are opt-in and disabled by default.
- Single-user tracking first; family/client portfolios follow. Do not require sharing or cloud login for the local app.

## Navigation and settings follow-up

Implemented from browser annotations: Privacy mode lives inside Settings, and Settings is pinned at the bottom of the left navbar. Saving applies the selected privacy mode through the existing settings service; it is no longer a header control. The app has no Quit button: use native window controls or the OS Quit command, retaining the existing awaited worker shutdown.

The desktop shell fits the viewport instead of growing with portfolio rows. Grouping/search remain above the independently scrollable asset/provider list, with Settings always reachable below it. The right content pane scrolls independently; table horizontal overflow stays inside its wrapper. Scroll panes are keyboard-focusable with visible focus. At narrow widths, navbar values move below names to keep asset/provider labels readable. The 647×724 annotation viewport is covered by browser acceptance.

## Source boundaries

[Store](../../libs/lib-core/src/storage.rs) provides local identities, exact quantities, reviewed exclusions and valuation queries. [Provider adapters](../../libs/lib-app/src/adapters) now live in the shared application library and are consumed by the CLI. The [stage-1 service and exact JSON DTOs](application-services.md) are implemented; exact current portfolio projections and the local read-only desktop browser are implemented. Existing CLI environment/key setup does not constitute a finished desktop connection or credential-storage flow.

Net-worth valuation exists, subject to available prices and provider coverage. Evidenced current-rate annual earnings are implemented; historical earnings, historical reward adjustment, benchmark calculations and DeFi underlying interpretation remain outstanding. Do not fill real-data screens with fabricated rates, yields or benchmark returns.

## First-build interview

The interview was conducted one question at a time. The answers below and acceptance scenarios define the proposed first version; remaining technical calculation choices belong to their supporting contracts before implementation.

1. **First-build data/setup scope**
   - Asked: Should the first usable build browse the already imported portfolio, include account connection/CSV import inside the UI, or deliver both together?
   - Answer: Display the already-imported portfolio first. Add account connections and CSV imports through the UI later.
   - Consequence: The first build reads the existing local portfolio DB and exposes its data and coverage states. Connection/import onboarding is outside this milestone; existing CLI workflows remain the acquisition path.

2. **Holdings grouping and navigation**
   - Asked: Should holdings be grouped primarily by asset across all accounts, by provider/account, or shown as two switchable views?
   - Answer: Both as switchable views, with assets across all accounts as the default.
   - Layout: Inside the main view section, place the asset/provider navigation on the left. Above the list, place a grouping multi-button/tab control labelled “by asset” and “by provider”. “by asset” is selected by default; switching to “by provider” changes the navigation grouping.

3. **Overview entry and content**
   - Asked: Should an “Overview” entry above the asset/provider list be selected when the app opens, showing the whole portfolio?
   - Answer: Yes. Place Overview above the list and select it by default on launch. It shows only total current value and “currently earning (per year)” for the whole portfolio.
   - Consequence: Keep Overview to these two financial summaries; do not add charts, transaction lists or opportunity panels there without a subsequent decision. The grouping default remains “by asset”. Current annual earnings follows the previously agreed current-position/current-rate definition, not historical received earnings or a renewal/withdrawal forecast. Its calculation backend remains outstanding; unavailable inputs must not produce an invented amount or zero.

4. **Asset/provider detail and allocation breakdown**
   - Asked: When selecting an asset, should the main panel show its total quantity, current value and current annual earnings, followed by a provider/product breakdown table?
   - Answer: Current value first; total quantity is a secondary, smaller number; annual earnings follows. Selecting an asset or provider switches the right pane to a detailed allocation breakdown, including provider/product groupings and individual subscriptions to the same product template. Each allocation shows how much it is earning as APY and amount per year.
   - Consequence: Asset detail leads with reporting-currency value, shows the asset quantity with less visual emphasis, and shows current annual earnings. Provider detail leads with value and annual earnings; quantities must remain associated with their individual assets rather than sum unlike units. Distinguish product templates from individual held subscriptions, retaining each subscription as a separate item even when templates match. Group totals summarize those allocations, not additional holdings to count again. The expandable-table presentation and initial expansion depth are confirmed below.
   - Data boundary: Product/subscription metadata, evidenced current rates and annual-earnings calculations still need supporting implementation. Unknown product attribution or rates must remain explicit; do not fabricate subscriptions or APYs from generic balance buckets.

5. **Allocation table presentation**
   - Asked: Should the allocation breakdown use an expandable table, with provider/product summary rows that expand to show individual subscriptions?
   - Answer: Yes. Use an expandable table with provider/product summary rows that expand to show individual subscriptions.
   - Consequence: Keep summary rows and subscription rows in the same table hierarchy. Expanded children explain their parent allocation; they are not added again to portfolio totals. Show APY and annual earnings at the appropriate allocation level, with missing evidence labelled explicitly.

6. **Selection scope and expansion depth**
   - Asked: Should provider rows start expanded to show products, with individual subscriptions collapsed until a product is expanded?
   - Answer/clarification: The left column is navigation, grouped by provider or asset. Selecting either updates the right pane to an expandable table for the remaining breakdown. Expand one level by default; users can expand further wherever a finer breakdown exists.
   - Consequence: Expansion depth is relative to the selected navigation item, not a fixed provider-first hierarchy for both modes. Scope the right pane to the selected asset or provider; the selection is its context, not another repeated navigation layer. Initially expose one level of the remaining hierarchy, keeping finer levels collapsed. Offer expansion only where actual child allocations exist.

7. **Navigation entry content**
   - Asked: Should asset/provider entries in the left navigation show current value and annual earnings alongside their names, or just names?
   - Answer: Name and current value, plus a small current APY number.
   - Consequence: Each asset/provider navigation entry shows its name, reporting-currency current value and a visually secondary current APY. Annual earnings amounts belong in Overview/detail as already specified. The aggregate APY calculation is confirmed below; missing rate evidence is not a zero rate.

8. **Overall current APY**
   - Asked: Should the navigation APY be value-weighted across the asset/provider’s allocations, including evidenced idle balances at 0%?
   - Answer: Yes: calculate the effective overall percentage across the stash. Include 0% balances.
   - Rule for asset allocations before borrowing costs: Overall current APY = sum(allocation current value × allocation current APY) / total current value of included asset allocations. Include evidenced idle/non-earning allocations in the denominator at 0%; do not average rates without accounting for allocation size. Use reporting-currency values when combining different assets. Expanded children explain their parent rather than adding another weight.
   - User example: 50% of ETH earns 5%, 25% earns 6%, and 25% earns 0%. Overall current APY is 0.50 × 5% + 0.25 × 6% + 0.25 × 0% = **4%**.
   - Boundary: This is the blended current APY, distinct from historical effective yield after changes in reward-currency value. Unknown rates remain unknown, not 0%; zero total value does not permit dividing to fabricate an APY. Incomplete-rate coverage and liability presentation are confirmed below; the gross-assets denominator for overall APY after borrowing costs is confirmed below.

9. **Navigation ordering**
   - Asked: Should asset/provider entries in the left navigation be sorted by current value, largest first, or alphabetically by name?
   - Answer: Current value, largest first.
   - Consequence: Sort both asset and provider navigation lists by descending reporting-currency current value. Overview remains above the list. Do not treat a missing valuation as zero to manufacture its sort value.

10. **Transaction-history scope**
   - Asked: Should the first UI include transaction history accessible from each asset/provider, or initially focus on balances, allocations and current earnings?
   - Answer: Focus on balances, allocations and current earnings first. Transaction history comes later.
   - Consequence: Do not add transaction-history screens or tables to the first UI milestone. Preserve underlying transaction data and existing diagnostics; deferring its UI does not remove evidence needed for calculations or reconciliation.

11. **Visual density**
   - Asked: Should the interface be compact and information-dense, or more spacious with larger text and rows?
   - Answer: In between: compact enough, but with enough padding to look pretty.
   - Consequence: Use a balanced density for navigation and allocation tables: compact, readable rows with deliberate padding and whitespace. Avoid both cramped financial grids and oversized layouts that obscure the allocation hierarchy. Exact spacing tokens will be refined in the design-system implementation.

12. **Themes and settings override**
   - Asked: Should the first version support both light and dark themes, following the system preference by default?
   - Answer: Yes. Support light and dark themes, follow the system preference by default, and offer an override in settings.
   - Consequence: Settings offers System, Light and Dark. System mode follows OS theme changes; an explicit override persists locally across launches. Apply the selected theme consistently to navigation, tables, summaries, settings and warning states.

13. **Manual current-APY entry and overrides**
   - Asked: Where imported data lacks current APY, should the first app let users enter or override it manually for a product/subscription?
   - Answer: Yes. Allow APY entry or overrides per product/subscription when rates are missing or incorrect. Keep unknown rates clearly labelled until provided.
   - Consequence: Include local rate editing in this milestone even though connection/import UI is deferred. Associate an entered rate with its product or individual subscription, label it as manual, and preserve source-rate evidence separately rather than rewriting imported transactions. Persist the override locally and use it in affected current-earnings/APY calculations. A missing rate remains unknown; an explicitly entered 0% is a known non-earning rate. Latest-update and pinning rules are confirmed below; product-versus-subscription inheritance and detailed rate provenance still need a supporting calculation/data contract.

14. **Partial earnings and incomplete overall APY**
   - Asked: If some rates are unknown, should known annual earnings appear as a labelled partial subtotal, while overall APY for the affected asset/provider remains unknown until all its rates are available?
   - Answer: Yes. Show known annual earnings as a clearly labelled partial subtotal. Keep overall APY unknown until all rates are available.
   - Consequence: Apply completeness within the displayed scope: Overview, selected asset/provider and allocation group summaries. Sum only calculable eligible annual earnings and visibly label an incomplete sum as partial. Do not present that subtotal as complete earnings. If any included allocation lacks a current rate, its parent/group overall APY stays unknown; do not substitute 0%, drop the allocation from the denominator or show the known subset’s APY as the overall rate. Known 0% allocations count normally. Existing reconciliation and missing-price restrictions still apply.

15. **Warning details, affected rows and overrides**
   - Asked: Should clicking a provider’s warning icon open a details panel explaining missing/mismatched data and which figures are withheld or partial?
   - Answer: Yes. If a warning affects only one or a few provider rows, those rows also show their own warning icons in the detail view. When a user adds a manual override, the warning should no longer show.
   - Consequence: Provider warning icons open an explanation of outstanding issues and affected figures. In the allocation table, attach row warning icons to the specific affected allocations rather than implying every row has the issue; clicking them shows scoped details. Summarize unresolved row issues at the provider level. A valid active manual override clears its corresponding missing/incorrect-rate warning and updates aggregate completeness; a manual-source label is provenance, not a warning. Clear the provider warning when no unresolved issues remain. Unrelated issues remain visible: an APY override does not resolve an independent balance mismatch or fabricate reconciliation. Preserve the original evidence for diagnostics.

16. **Latest rate update and pinned overrides**
   - Asked: Should manual APY overrides remain active until explicitly removed, even when a provider later reports a different rate?
   - Answer: The latest update wins unless the manual override is pinned. A newer automated provider update supersedes an older manual override; a newer manual override supersedes an older fetched value.
   - Consequence: Store both manual and provider rate observations with their update timestamps/provenance. Select the latest applicable update for an unpinned rate, rather than permanently preferring manual input. A pinned manual override stays active despite later provider updates; retain those updates as evidence. Provide pin/unpin controls and show pin status as provenance, not an error warning. Unpinning returns rate selection to the latest-update rule. Replaying an old cached observation must not turn it into a new update.
   - Acceptance examples: A provider update after an unpinned manual edit becomes active. A later manual edit becomes active in turn. While that manual rate is pinned, subsequent provider updates remain recorded but do not replace it. Calculated APY and annual earnings follow the active rate.

17. **Subscription term columns**
   - Asked: Should subscription rows also show lock status, maturity date and auto-renew status when that information is available?
   - Answer: Yes. Show lock status, maturity date and auto-renew status when available.
   - Consequence: Include these term details alongside subscription allocations, APY and annual earnings. Distinguish unknown lock/renewal status or maturity from an evidenced flexible balance, disabled renewal or no applicable maturity. Display existing provider settings; changing real renewal settings remains outside first-version execution scope. Availability/withdrawal planning is still a separate later feature.

18. **Liability rows and negative annual earnings**
   - Asked: Should debts appear as clearly marked liability rows in the same allocation table, showing borrowed amounts and annual borrowing costs separately?
   - Answer: Yes. Show debts as clearly marked liability rows, with borrowed amounts and annual borrowing costs shown separately. Annual borrowing costs count as negative earnings per year.
   - Consequence: Keep assets and liabilities visibly distinct in the allocation table. Debt reduces current net worth; its current annual borrowing cost reduces annual earnings in the applicable asset/provider and Overview scope. Display the debt amount and cost separately, rather than hiding either in an asset row. Deduct each cost once when rolling up allocations; unknown borrowing rates/costs make the net-earnings total partial, not a zero-cost assumption. Use the gross-assets denominator confirmed below for the overall APY after borrowing costs.
   - Acceptance example: Assets yielding USD 500/year and borrowing costs of USD 100/year produce USD 400/year net earnings, with the USD 100 cost displayed as negative earnings on the liability row.

19. **Overall APY denominator after borrowing costs**
   - Asked: Should overall APY divide net annual earnings by gross asset value or net worth after debt?
   - Answer: Use gross assets, as the more conservative figure.
   - Rule: Overall current APY = net current annual earnings / included gross current asset value. Annual borrowing costs reduce the numerator; liabilities do not reduce the denominator or become negative allocation weights. Known idle assets remain in the denominator. Debt still reduces displayed net worth, independently of this APY calculation. Apply the existing complete-rate/valuation requirements; a zero gross-assets denominator has no defined overall APY.
   - Acceptance example: USD 10,000 assets and USD 2,000 debt give USD 8,000 net worth. USD 500/year asset earnings minus USD 100/year borrowing costs give USD 400/year net earnings and **4% overall APY** (400 / 10,000).

20. **Manual allocation splitting**
   - Asked: If imported balances lack product/subscription details, should the first app let users manually split them into allocations, such as a flexible balance and separate fixed-term subscriptions?
   - Answer: Not for now.
   - Consequence: Manual allocation splitting is outside this milestone. Display the breakdown evidenced by imports, retaining aggregate balances where finer detail is unavailable and labelling missing product/subscription attribution. Do not fabricate subscriptions or flexible/fixed-term splits. Expand only existing detail. Manual APY entry/overrides for available product/subscription records remain in scope; aggregate-holding APY overrides are also confirmed below.

21. **Database location and first-launch fallback**
   - Asked: Should the app automatically open the existing database, otherwise let users select one and remember the choice?
   - Answer: Open automatically from `~/.data/money-tracker/<DB filename>` on Unix systems including macOS and Linux. Allow overrides first through an environment variable, then a config file. If missing, offer to import/copy a selected database file or start new.
   - Contract: Use the existing filename `portfolio.duckdb`: the Unix default is `~/.data/money-tracker/portfolio.duckdb`. Reuse `APP_DATA_DIR` for the data-directory environment override, with a `data_dir` configuration setting. Precedence is environment → config file → default. The implemented [stage-2 startup contract](database-startup.md) specifies JSON configuration, native Windows paths and the shared resolver.
   - Startup: Open an existing database at the resolved location. If missing, offer **Copy existing database** or **Start new**. Copy the selected database into the resolved location and use that copy, preserving the original; this imports a database, not provider CSVs. Starting new creates an empty database with the normal ordered idempotent migrations. Future launches use the same resolved location. Invalid/unreadable databases show an error rather than silently becoming empty portfolios.
   - Implementation: Stage 2 aligns the CLI/shared resolver with the Unix default and loads JSON configuration. It detects the previous OS-based location, preserves its DB and provides explicit Rust copy/new actions with validation and a managed worker. The chooser/settings UI is still proposed; see [database startup](database-startup.md).

22. **Refresh interval and launch behavior**
   - Asked: What should the default automatic balance/rate refresh interval be while the app is open?
   - Answer: Daily by default, with the interval changeable in settings. Do not refresh when the app opens unless enabled in settings; the user intends to enable that only for testing.
   - Consequence: Load stored portfolio data on launch without initiating a balance/rate refresh by default. Settings provides the interval (daily default) and a separate **Refresh on open** toggle (off by default). Persist both settings. Scheduled refresh runs while the app is running. Ordinary launches do not trigger a refresh while the launch toggle is off; a missed scheduled refresh is the exception confirmed below. Manual refresh remains in settings/options, as already agreed. A refresh failure retains the last stored view and its freshness/coverage indicators.

23. **Explicit Quit and missed-refresh catch-up**
   - Asked: When the window closes, should the app quit or keep running in the system tray/menu bar so scheduled refresh can continue?
   - Answer/clarification: Quit is explicit and quits for real. If previous refreshes were missed, refresh once on the next open.
   - Consequence: An explicit Quit ends the app and its background refresh work; do not keep a hidden process refreshing after Quit. Persist refresh scheduling state across launches. When scheduled refreshes were missed while the app was stopped, perform one catch-up refresh on the next open, even when the optional refresh-on-open toggle is off. Do not run one refresh for each missed interval or duplicate catch-up with an enabled launch refresh. Display cached data while the refresh runs. Without a missed interval or enabled launch toggle, opening the app does not refresh.
   - Acceptance examples: Quit and reopen before the next due refresh: no fetch unless refresh-on-open is enabled. Reopen after one or several missed daily intervals: one catch-up fetch. Explicit Quit leaves no background refresh process. Window-close behavior is clarified separately below.

24. **Window close terminates the app**
   - Asked: Should closing just the window leave the app running in the background, with Quit as a separate action?
   - Answer: Closing the window quits the app. No background process.
   - Consequence: Both closing the window and explicit Quit terminate the app and its refresh work. Do not add tray/menu-bar background operation. Preserve data/settings/scheduling state on normal shutdown; the next launch uses the single missed-refresh catch-up rule above.

25. **Partial valuation, manual prices and irrelevant holdings**
   - Asked: If some holdings lack a current price, should current value show a clearly labelled partial subtotal, with unpriced holdings still visible?
   - Answer: Yes. Show a clearly labelled partial subtotal and keep unpriced holdings visible with warnings. Allow manual asset-price overrides when unknown, or declarations as dust/spam when irrelevant.
   - Consequence: Sum known eligible values and label incomplete current-value totals as partial. Keep unpriced holdings and their quantities visible with scoped warning icons. Add local manual current-price entry for an exactly identified asset, retaining source evidence and manual provenance; resolving the missing price clears that issue’s warning. Price entry must not invent a quantity/scale, alter source transactions or resolve unrelated reconciliation issues. Add user declarations for dust/spam without deleting source history; reuse the reviewed tracking-policy foundation for spam exclusions. Dust’s inclusion and navbar grouping are confirmed below. Price update/pinning semantics are confirmed below.

26. **Dust inclusion and navbar grouping**
   - Asked: Should dust be excluded like spam, or hidden while its known value still counts?
   - Answer: If value is known, count it; group all dust assets into “Dust” in the navbar as well.
   - Consequence: Dust is a presentation classification, not a tracking exclusion. Group dust holdings under one **Dust** navigation entry, subject to the provider-specific scope clarified below; do not label an entire asset as dust just because one provider holds a small amount. Its detail view retains the individual holdings and provider/product breakdown. Known dust values remain in portfolio totals and the gross-assets APY denominator, with known earnings included under the normal coverage rules. The group summary and its children must not be counted twice. Unpriced dust remains unpriced: no fabricated zero value, and the existing partial-total/coverage rules apply. Spam remains excluded through the reviewed tracking policy; preserve history for both classifications.

27. **Manual-price updates and pinning**
   - Asked: Should manual asset prices follow the same latest-update-wins rule as APY, with an optional pin to prevent provider updates replacing them?
   - Answer: Yes.
   - Consequence: An unpinned manual price supersedes older applicable provider observations; a newer provider update supersedes that manual price. A pinned manual price remains active while later provider observations are preserved separately. Show source/pin status, persist the choice locally and apply it to current valuations. Unpinning returns to latest-update selection. Replaying old cached prices does not create new updates. Keep timestamps/provenance and original price evidence; a current override does not rewrite historical prices or source quantities.

28. **Asset-wide spam and provider-specific dust**
   - Asked: Should declaring an asset as dust or spam apply across all accounts holding that same asset, or only the selected account?
   - Answer: Spam is spam for the asset everywhere. Dust depends on the holding’s fiat value at a provider: a small holding can be dust there while another provider holds a significant amount of that asset.
   - Consequence: Spam classification follows the exact resolved asset identity across holdings, including other accounts holding that same asset; never propagate by matching ticker/name alone. Dust is scoped to an asset’s holding at a provider, not a global asset flag. Preserve larger holdings of that asset elsewhere in ordinary navigation/detail. Both classifications preserve source history; dust remains financially included when valued and spam remains excluded. Grouping must not double count amounts. Automatic dust assignment is confirmed below; the default USD 1 threshold is confirmed below; asset summaries retaining dust values are confirmed below.
   - Source gap: Current reviewed exclusions are account/asset-scoped. Asset-wide spam classification requires persistent identity-level policy that also applies to newly discovered accounts; merely copying decisions to today’s accounts is not sufficient. Provider/asset dust metadata is separate from exclusion policy and is not implemented yet.

29. **Automatic dust threshold and manual classification**
   - Asked: Should dust be assigned automatically below a configurable fiat-value threshold, or only when users mark a holding manually?
   - Answer: Automatically below a configurable fiat-value threshold, with manual marking or unmarking of a holding as dust.
   - Consequence: Evaluate known fiat values at provider/asset holding scope against the configured threshold. Offer Auto, Mark as dust and Keep out of dust states, persisting a manual choice until changed or reset to Auto. Re-evaluate Auto holdings as balances/prices or the threshold change. Unknown prices cannot establish that a holding is below the threshold; retain normal missing-price warnings, including for manually marked unpriced dust. This grouping concerns asset holdings, not liability rows, and never excludes valued dust from financial calculations.

30. **Default dust threshold**
   - Asked: What should the default dust threshold be in USD?
   - Answer: USD 1.
   - Consequence: Default automatic dust classification uses known provider/asset holding value strictly below USD 1. A USD 1 holding is not below the threshold. Users can change the threshold or manually mark/unmark a holding as already agreed; unknown prices do not become zero for classification.

31. **Asset summaries retain dust holdings**
   - Asked: If ETH is worth USD 1,000 on one provider and USD 0.50 dust on another, should the asset navbar entry show USD 1,000.50 or only USD 1,000?
   - Answer: USD 1,000.50 across both providers. Dust grouping must not change the asset’s total value.
   - Consequence: Asset navigation summaries and selected-asset totals include that asset’s tracked holdings across all providers, including dust. Dust is a filtered navigation/detail view of existing holdings, not another balance bucket. Its value can overlap an asset summary; never calculate portfolio totals by adding navbar entries or adding Dust to already-inclusive asset totals. Preserve the same quantities, current value and earnings coverage when changing presentation/grouping.
   - Acceptance example: ETH’s navbar value and detail total remain USD 1,000.50; the Dust view includes the USD 0.50 holding. Portfolio value includes those ETH holdings once, not USD 1,001.00.

32. **Allocation-table default ordering**
   - Asked: Should the allocation table have sortable columns, defaulting to largest current value first within each group?
   - Answer: Largest current value first.
   - Consequence: Default sibling-row ordering is descending current reporting-currency value. Preserve the expandable hierarchy rather than flattening all subscriptions to sort them globally. Missing valuation remains explicitly unknown, not a manufactured zero. The answer confirms the default ordering; optional interactive column sorting is not a first-build requirement yet.

33. **Aggregate-holding APY overrides and finer-data invalidation**
   - Asked: Where imports provide only a provider/asset balance, should users be able to enter a single APY for that whole holding without creating product/subscription splits?
   - Answer: Yes, as a manual override. Propose invalidating it when a sub-product/subscription later adds finer data, even for only part of the whole holding.
   - Consequence: Allow a scoped manual overall APY on an aggregate provider/asset holding when no product/subscription breakdown is available. This is a rate annotation, not an invented allocation split. Retain the override and its provenance when new finer data appears. The proposed invalidation concerns the whole-holding assumption even if the new breakdown is partial; never count aggregate earnings and child earnings twice, or silently copy the old aggregate rate onto the uncovered remainder. Once invalidated, use available child evidence and the existing partial-earnings/unknown-overall-APY rules where coverage remains incomplete. Prompted review and protection of pinned overrides are confirmed below.

34. **Review aggregate overrides when finer data arrives**
   - Asked: Should finer data automatically invalidate an aggregate override, even if pinned, or prompt a review?
   - Answer: Prompt the user to review it. Do not automatically invalidate a pinned override.
   - Consequence: New product/subscription detail triggers a scoped review prompt explaining that the whole-holding APY assumption may need replacing, even if the new detail is partial. Structural detail alone does not silently delete the override. A pinned aggregate rate stays active until the user changes it; keep new child evidence separately and never add child earnings to already-inclusive aggregate earnings. Let the user retain the override or explicitly switch to available detailed rates; the latter follows partial-coverage rules when a remainder lacks rates. Ordinary applicable unpinned rate updates still follow the latest-update rule; pin protection remains intact. Preserve review/override provenance.

35. **Navigation search**
   - Asked: Should the navbar include a search field to find assets/providers by name or ticker?
   - Answer: Yes.
   - Consequence: Add case-insensitive name/ticker search to the asset/provider navigation lists. Search filters navigation entries in the selected grouping mode; it does not change portfolio totals, earnings calculations or tracking decisions. Preserve exact asset/provider identities when names or tickers match several entries. Keep Overview accessible while searching and show a clear empty result when nothing matches.

36. **Inline rate and price editing**
   - Asked: Should manual APY and price edits happen inline in the table, or in a small editor opened from the row?
   - Answer: Inline in the row, with an icon to pin.
   - Consequence: Edit applicable APY and current-price values directly in their table rows. Place a pin/unpin icon alongside the editable value with visible state and an accessible label. Persist edits at the agreed product/subscription, aggregate-holding or exact-asset scope; a price edited from one holding still applies to that asset’s current valuations elsewhere. Preserve provenance and apply the existing latest-update/pin rules. Validate input before saving, keep unknown distinct from zero, and update affected summaries after a successful save.

37. **Subscriptions have their own rate histories**
   - Asked: Should a product-level APY apply to all its subscriptions by default, while allowing an individual subscription to have its own rate?
   - Answer: Not as live inheritance. A product is a template; a subscription is an instance linked to APY changes, beginning at subscription time. Older subscriptions may have different starting rates, and an existing subscription can change APY later.
   - Consequence: Persist subscription-specific effective-dated rate history. Product advertised rates do not automatically update subscriptions. Show the subscription's currently applicable rate; derive product summary APY from its held subscriptions. Inline edits retain prior periods and source evidence. See the [proposed subscription rate history model](data-model-proposal.md#subscription-rate-history--proposed-confirmed-entity-boundary).

38. **Manual APY effective dates**
   - Asked: When editing APY inline, should it take effect from today by default, with an option to choose an earlier effective date for corrections?
   - Answer: Yes. Default to today, with an earlier effective date available for corrections.
   - Consequence: Inline APY editing defaults the effective date to today and allows choosing an earlier date. Record when the edit was made separately from when the rate applies. Preserve earlier evidence and subsequent applicable rate changes; a backdated correction must not silently replace every later rate or alter reward transactions. Apply existing pin and evidence-resolution rules within the affected period.

39. **Defer the APY-history editor**
   - Asked: Should the first version let users view and correct earlier APY changes for a subscription, or only edit its current rate?
   - Answer: No APY-history editor in the first version. Transactions ultimately determine historical achieved APY.
   - Consequence: Keep inline current-rate editing and its optional earlier effective date, but defer a dedicated rate-history browser/editor. Preserve rate evidence internally for current-rate selection and audit. Historical achieved/effective yield is calculated from actual reward/cost transactions, principal deposits/withdrawals and time invested, using the agreed valuation methodology; advertised or manually entered rate history does not substitute for missing transaction history.

40. **Current holdings only**
   - Asked: Should the allocation table show only current holdings by default, leaving closed or zero-balance subscriptions for a later history view?
   - Answer: Yes. Show only current holdings for now; closed or zero-balance subscriptions come later with the history view.
   - Consequence: Omit closed and exact-zero-balance subscriptions from the first allocation view without deleting their records, rate evidence or transactions. Keep nonzero unpriced holdings visible; an unknown valuation is not a zero balance. Zero-yield holdings remain visible and included in weighted APY. Nonzero liabilities remain visible under the existing liability contract. Historical calculations and reconciliation retain closed/zero-balance records where needed.

41. **Inline borrowing rates and negative earnings**
   - Asked: For debts, should the borrowing rate be editable inline like APY, with annual borrowing cost calculated from the outstanding balance?
   - Answer: Yes. Edit borrowing rates inline with a pin icon, calculate annual costs from the outstanding balance, and count them as negative earnings.
   - Consequence: Allow a scoped annual borrowing-rate edit in each liability row, alongside a pin/unpin icon. Reuse manual-rate provenance, effective-date and latest-applicable-update rules, protecting pinned overrides. Calculate current annual cost from the positive magnitude of outstanding debt and its applicable annual rate, respecting evidenced APR/APY conventions and terms. Display the cost separately and subtract it once from net annual earnings at every rollup; do not compound a negative sign from the signed liability balance. Unknown borrowing rates produce partial net earnings until resolved. The debt balance remains source-derived and reconciliation remains strict.

42. **Privacy modes**
   - Asked: Should the first version include a privacy toggle that hides monetary amounts and asset quantities while leaving names and percentage rates visible?
   - Answer: Yes. Also provide a second privacy mode showing much lower amounts while maintaining proportions.
   - Consequence: Provide an app-wide privacy toggle that masks monetary amounts and asset quantities in Overview, navigation, allocation rows and warning details, including annual earnings and borrowing costs. Keep asset/provider/product names and percentage rates visible. Do not expose masked values through hover text or accessibility labels. This changes presentation only: totals, tracking policy, rate selection and sorting still use the underlying data. Privacy mode does not alter stored financial records.
   - Second mode: Offer scaled display alongside fully hidden amounts and normal display. Use one common positive factor below 1 for holding quantities, values, liabilities and annual earnings/costs across the portfolio; preserve signs, allocation proportions, totals and percentage rates. Apply scaling before display rounding and never reclassify dust or exclusions from scaled figures. Clearly indicate that figures are scaled; source data and calculations remain unchanged. Hover/accessibility text and warning details must follow the active privacy mode. Default the factor to 10% of actual amounts; let users choose a positive scale below 100%. Apply one chosen factor consistently throughout the portfolio.

43. **Local account and subscription names**
   - Asked: Should the first version let users rename accounts and subscriptions locally, keeping the provider’s original name/reference available in details?
   - Answer: Yes.
   - Consequence: Persist optional local display-name overrides on exact account/subscription IDs. Show these names in the relevant allocation rows and include them in applicable navigation search. Preserve the provider’s original name/reference in details and allow clearing the local name to restore the source label. Provider refreshes may update source metadata but do not replace a local display name. Renaming never changes identity, mappings, ledger history or reconciliation.

44. **Scaled privacy default and configuration**
   - Asked: What default scale should the second privacy mode use—for example, 1% of actual amounts?
   - Answer: Use 10% by default; let users choose the scale.
   - Consequence: Initialize scaled privacy mode at a factor of 0.1 and provide a user setting for the factor. Persist the chosen scale locally and apply it consistently to displayed holdings, liabilities and earnings/costs, retaining unchanged percentage rates and financial calculations.

45. **Account tree and initial expansion**
   - Asked: When a provider has several accounts or wallets, should its allocation table show a separate expandable group for each account, using local names?
   - Answer: Accounts form a tree, like GnuCash: accounts have parents. Expand only one extra level below top-level accounts.
   - Consequence: Support nested parent–child accounts rather than a flat provider-to-account list. Initially show top-level accounts and their immediate children; deeper descendants remain collapsed until expanded. Apply the selected asset/provider scope without losing the relevant account ancestry. Parent summaries roll up direct holdings and descendants once; grouping accounts do not create additional balances. Preserve local names and provider/source identity independently of the tree. See the [proposed account hierarchy](data-model-proposal.md#account-hierarchy--proposed).

46. **Editable groupings and provider defaults**
   - Asked: Should the first version let users create grouping accounts and move accounts between parents, or initially show only the stored/imported hierarchy?
   - Answer: Allow editing groupings. Start with a sensible structure for each provider; each provider can define a default structure mapping.
   - Consequence: Include local grouping creation and account reparenting within a provider in the first UI; custom cross-provider groups are deferred below. Provider adapters supply default hierarchy mappings based on supported source account/product structure, creating stable grouping nodes only where meaningful. Preserve user-edited parents/names across refreshes; defaults seed uncustomized/new records rather than rebuilding the entire tree. Keep grouping changes separate from source economic movements and reconciliation. Do not fabricate subscriptions or product allocations where imports lack detail.

47. **Later cross-provider grouping in the selected-asset view**
   - Asked: Should custom groups span providers, for example a “Long-term savings” group containing both Nexo and Kraken accounts?
   - Answer: Yes, when an asset is selected in “by asset” navigation. Put this later on the roadmap; it is not a priority.
   - Consequence: Defer custom groups spanning providers from the first version. Later, allow asset-scoped allocation groupings across providers in the selected-asset detail pane. These are presentation memberships over existing holdings, not new ownership or provider accounts. Preserve provider-specific source trees and count each holding once. Do not apply such a group to every asset by globally reparenting a source account. The exact saved grouping/membership schema is a later design task.

48. **One portfolio in the first UI**
   - Asked: Should the first UI open one portfolio only, or offer a switcher for multiple portfolios already stored in the database?
   - Answer: One portfolio for now. Add a portfolio switcher when family/client tracking is supported.
   - Consequence: All first-version views and settings operate on one explicit portfolio identity; there is no portfolio switcher. Never combine separate portfolios merely because they share a database. Preserve the existing portfolio/account model for later family/client tracking and its switcher.

49. **Row details and expansion controls**
   - Asked: Should clicking an account/subscription name open a details panel with its original provider reference, rate source and lock/renewal information, while a separate arrow expands its children?
   - Answer: Yes. Clicking the name opens details; a separate arrow expands children.
   - Consequence: Make account/subscription names open their scoped details panel, showing original provider names/references, applicable rate provenance/pin state and available lock/maturity/renewal metadata. Use a distinct expansion arrow only on rows with children. Opening details does not change expansion; expanding children does not open details. Keep inline rate/price/pin controls independent of both actions, with accessible labels and keyboard controls. Warning icons continue to open scoped issue details.

50. **Right-side details panel**
   - Asked: Should the details panel slide in on the right, keeping the allocation table visible rather than replacing it?
   - Answer: Yes.
   - Consequence: Show scoped row/issue details in a panel sliding in from the right of the main view. Keep the allocation table visible and preserve its selection, scroll and expansion state. Provide an explicit close control and keyboard dismissal; closing returns focus to the invoking name or warning control. Respect reduced-motion preferences for the slide transition.

51. **Neutral visual palette**
   - Asked: Should the visual style use a neutral palette with one accent color, or follow an existing app/style reference?
   - Answer: Use a neutral palette with one accent color for now.
   - Consequence: Use neutral surfaces, borders and text with one primary accent for selection and interactive emphasis, consistently across light/dark themes. Keep warning/error/success states distinguishable through semantic colors and icons/text; these are status cues rather than additional brand accents. Combine this with the agreed balanced density and accessible focus/contrast states. The specific accent can be chosen during design-system implementation.

52. **Full currency amounts with selective decimals**
   - Asked: Should currency values show full amounts with thousands separators and two decimals rather than abbreviations such as $12.3k?
   - Answer: Full amounts, with decimals only when the amount is small and the decimals are significant; otherwise omit them.
   - Consequence: Use thousands separators and avoid automatic abbreviations. Decimal precision is adaptive rather than fixed at two places for every value. Preserve exact underlying data and distinguish nonzero tiny values from zero; the confirmed USD thresholds and examples are recorded below.

53. **Separate design-system package — plan only**
   - Requested: A separate TypeScript pnpm workspace package built with SolidJS, Panda CSS and Ark UI, consumed by the app.
   - Clarification: Plan this package only; do not implement it yet.
   - Consequence: The [design-system package plan](web-design-system.md) defines boundaries, styling, exports, initial components and validation. No frontend/workspace scaffolding is part of the current documentation task.

54. **Currency display thresholds**
   - Asked: Should currency values show two decimals below $100, whole dollars at $100 or more, and “<$0.01” for nonzero values below one cent?
   - Answer: Yes.
   - Consequence: For USD, select precision from the absolute displayed amount: two decimals below $100, whole dollars at $100 or more, and “<$0.01” for a positive nonzero amount below one cent. Preserve negative signs for costs/losses (for example, “−<$0.01” for a tiny negative value); actual zero displays as $0.00. Use thousands separators, no automatic abbreviations, and ordinary rounding only for display. Examples: $12.34 → $12.34; $100.25 → $100; $12,345.67 → $12,346; $0.004 → <$0.01. In scaled privacy mode, format the scaled amount; never reveal the underlying amount. Calculations, dust thresholds and reconciliation use unrounded original values.

55. **Token quantity precision**
   - Asked: Should token quantities trim trailing zeros and show enough decimals to keep small balances nonzero, with the full exact quantity available in details?
   - Answer: Yes.
   - Consequence: Trim insignificant trailing zeros in table/navigation quantity displays and adapt decimal precision so a nonzero balance does not appear as zero. Details expose the full exact decimal quantity using the asset’s protected scale, without conversion through floating-point numbers. Display formatting never changes ledger units or scale. All quantity displays, including details, respect hidden/scaled privacy modes; full original quantities are available only in normal mode.

56. **Move-to regrouping action**
   - Asked: Should account regrouping support drag-and-drop, with a “Move to…” action as an alternative?
   - Answer: Use a Move to action in the details pane or context menu.
   - Consequence: Provide “Move to…” for an account/group in its details pane or context menu, opening a valid-parent selector. Drag-and-drop is not a first-version requirement. Restrict choices to the agreed within-provider, single-portfolio scope; exclude the selected node and its descendants to prevent cycles. Moving changes grouping metadata only and preserves source identity, balances and user-selected parent state across refreshes. Keep the action accessible by keyboard.

57. **Undo manual changes**
   - Asked: Should the first version offer Undo for manual changes such as rates, names, grouping and dust/spam classification?
   - Answer: Yes. Offer Undo for manual changes to rates, names, grouping and dust/spam classification.
   - Consequence: Capture the previous state of each successful manual change and allow restoring it, including associated override/pin metadata, local labels, parent links and dust/spam policy. An undo is a new user action, preserving original evidence/revisions rather than deleting source records. Revalidate parent choices and applicable rate evidence at undo time; do not silently undo intervening provider imports or refreshes. Recalculate affected totals, earnings eligibility and warnings after the restored state is committed. Undo persists across app restarts as confirmed below.

58. **Persistent action log for Undo**
   - Asked: Should Undo remain available after restarting the app, or only during the current session?
   - Answer: Keep Undo across app restarts. Store an action log and the previous state that can be restored.
   - Consequence: Persist manual action records with stable target identity, action type, timestamp/order, previous and resulting state, and links to any undo action. Commit the setting/policy change and its action record atomically. Restore available undo history when opening the same portfolio after restart. Apply Undo as a compensating action, preserving log/evidence and provider updates. Revalidate restoration against current state and report conflicts rather than silently discarding later changes. See the [proposed action log](data-model-proposal.md#persistent-manual-action-log--proposed).

The first-version UI scope is sufficiently specified for planning. No implementation is requested. Connection/import setup, transaction history, a dedicated APY-history editor, manual allocation splitting, cross-provider custom groups and the portfolio switcher remain deferred. Supporting calculation conventions, configuration details and technical package version choices still need their documented implementation-stage review.

## First-version acceptance scenarios — proposed

1. **Open existing data:** Resolve the configured database with environment/config/default precedence, open one explicitly scoped portfolio and run ordered idempotent migrations. Select Overview in by-asset navigation by default; show current value and current net annual earnings. Missing DB offers copying an existing file or creating an empty one; unreadable DB does not silently become empty.
2. **Navigate holdings:** Search names/tickers without changing financial totals. Lists and sibling allocations sort by current value descending. Selecting an asset/provider scopes the expandable account/product/subscription breakdown, with roots and immediate account children visible. Current nonzero unpriced and 0%-yield holdings remain visible; closed/exact-zero subscriptions remain stored but absent from this view.
3. **Financial correctness:** Parent/child totals and dust views count each holding once. Liabilities reduce net worth; annual debt costs subtract from earnings once. Weighted current APY uses gross assets including 0%-yield balances. Unknown rates/prices produce labelled partial subtotals and unknown overall APY where required; source mismatch warnings remain scoped and historical unreconciled figures stay withheld.
4. **Edit rates:** Inline APY/borrowing/price edits and pin icons update relevant current summaries without altering source transactions. Rate edits default to today and permit earlier effective dates. Separate subscriptions keep independent rate histories; product advertised changes do not overwrite their terms. Newer applicable evidence wins unless pinned; finer allocation evidence prompts aggregate-override review.
5. **Edit organization and tracking:** Local names survive refresh with original names/references in details. Move to actions reparent within valid provider/account trees without cycles or identity changes. Provider defaults initialize uncustomized structure. Asset-wide spam exclusion preserves history; provider/asset dust remains included, defaults below $1 and supports manual classification.
6. **Undo after restart:** Change a rate/name/parent/classification, close and reopen the app, then Undo. Restore the prior local state, refresh affected figures and retain both the change and its undo in the action log. A concurrent/newer conflicting change is surfaced without rolling back provider imports.
7. **Details and presentation:** Names open the right-side panel while separate arrows expand children. Keep the table visible and preserve its state. Support neutral light/dark/system themes, accessible keyboard/focus behavior and reduced motion. Currency formatting and exact quantity detail follow the confirmed rules.
8. **Privacy:** Hidden mode masks amounts/quantities in every visible/detail/hover/accessibility presentation. Scaled mode defaults to 10%, configurable, preserves proportions and percentage rates, and labels the figures as scaled. Neither mode changes financial calculations or dust classifications.
9. **Refresh and lifecycle:** Daily automatic refresh is configurable; ordinary refresh-on-open defaults off. One catch-up refresh occurs after missed schedules, coalesced with any launch refresh. Manual refresh is in options. Closing the window or using the OS Quit command ends the process; cached data and warnings remain usable offline.
10. **Package boundary:** The future Solid app consumes the separate pnpm Solid/Panda/Ark design system with stable exports and reproducible builds. It presents existing portfolio data through typed core/application contracts; synthetic showcase values are never represented as imported portfolio facts.

The shared design-system compact follow-up uses 4–12px corners, smaller typography and tighter spacing throughout controls, navigation, cards and tables. The header is 48–56px tall; the viewport containment and independently scrollable panes remain in effect. Sage colors, exact formatting and unknown/partial financial states are unchanged.

Navigation placement refinement: Overview stays in the fixed controls area above the By asset / By provider tabs and search. Only asset/provider entries scroll. Settings remains at the navbar bottom, with a decorative gear icon and transparent background/border at rest; its background appears on hover and its keyboard focus outline remains visible.

The portfolio pane no longer displays the local-snapshot/current-rate explanatory footnote. Calculation assumptions remain documented, and scoped unknown/partial states and warning details remain visible in the UI.

Settings is a main-pane destination: it replaces the entire right-side portfolio content and highlights Settings in the navbar using the shared selection colors. Other navigation items are unselected while Settings is active. Selecting Overview, an asset/provider or Dust returns to portfolio content. Evidence and warning details continue to use the separate details panel.
