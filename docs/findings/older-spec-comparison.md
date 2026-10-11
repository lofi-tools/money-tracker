# Older product draft: comparison and follow-up decisions

Compared on 11 October 2026 with the [user needs interview](user-needs.md). Source: the user-supplied “Money tracker” draft in the pasted-text attachment. The comparison initially identified interview gaps; subsequent user decisions below establish scope and timing. Remaining questions in the topic table are details to resolve, not a reason to treat confirmed priorities as unanswered. External links and competitor statements in that draft have not been revalidated.

## Confirmed follow-up decisions

| Topic | User decision |
| --- | --- |
| Tax reporting | Later, through a cloud service syncing local data; document workflows and jurisdictions remain open. |
| Spending | Yes, later. |
| Shared portfolios | Soon after single-user tracking: family/clients first, then couples/groups/households, then companies. |
| Open source | From the start; exact license/component scope remains open. |
| Community integrations | Later. |
| Mobile, sync, and backups | Later. Local shared portfolios should eventually stay synchronized; encrypted backup/restore also comes later. Mechanisms remain to be scoped. |
| Historical dashboard graphs and benchmarks | Early: historical performance graphs and comparison against simply holding the same assets, accounting for fees and deposits/withdrawals. Exact benchmark construction remains open. |
| Affiliates | After paid cloud/API plans are live. |
| Invoicing | Much later. |

## Topics raised by the older draft

| Topic in the older draft | User-facing questions or decisions still open |
| --- | --- |
| Tax, accounting, and evidence | What records are needed for a tax return or audit? Should the app attach transaction proofs, enrich metadata, export reports, generate accounts, or eventually help file returns? Which jurisdictions and whose workflow? Current earnings, gains, and attribution requirements do not establish tax compliance. |
| Spending and recurring subscriptions | Where is money going? Which subscriptions are unused or could be cheaper? Are expense categories and AI tagging part of this product, a later module, or a separate app? |
| Multiple entities and shared use | How should personal, couple/household, company, family, event/group, or client portfolios be separated or combined? Who may view or edit them, and how are shared ownership and debts represented? The draft labels multi-entity work as later. |
| Extensibility and open-source commitments | Can community developers add integrations? What connector/plugin contract is needed, and which app/API components are open source under which license? Free local use is confirmed, but free use and open-source licensing are different commitments. |
| Mobile, cross-device access, and data continuity | Is mobile a companion or full application? How should local devices share data, and what works offline? How do users back up, restore, export, or migrate records if a device fails or the optional service closes? Desktop first and a secondary cloud version are confirmed; these workflows are not. |
| Historical dashboard and performance views | Net-worth/earnings history and reporting periods are confirmed. Buy-and-hold comparison belongs early alongside historical performance graphs, accounting for fees and deposits/withdrawals. Detailed chart views and benchmark construction remain to be specified. |
| Other monetization and distribution | Ads, affiliate/sponsor income, account-manager pricing, newsletters, and marketing channels were suggested. Current decisions confirm paid risk/dependency APIs and bank/broker integrations, not those additional business models. If affiliates are retained, how are sponsored placement and opportunity/risk assessment kept distinguishable? |
| Invoicing | The draft mentions invoicing and ZUGFeRD. Its role and priority are unconfirmed and should not expand initial crypto tracking implicitly. |

## Covered or substantially clarified

The interview covers local-first storage, subscription-free local holdings/earnings, a secondary managed cloud experience, bring-your-own API keys, provider OAuth where available, broad future net-worth coverage, debt and collateral, automated wallet/DeFi discovery, historical reconciliation, yield reporting, same-asset opportunity comparisons, paid risk ratings/dependency maps, opt-in alerts, withdrawal planning, and a later money router.

The current first-pass interface choice is Tauri with SolidJS, Panda CSS, and Ark UI shared with the web version; Rust/GPUI is a later desktop direction. The initial feature order is holdings, earnings, opportunity comparisons, then withdrawal planning. Integration order after Nexo/Binance is Kraken, EVM wallets, Solana, then NEAR.

## Earlier assumptions not carried forward automatically

- The old early plan includes Coinbase and bank integrations among first priorities; the user's newer integration order and crypto-first scope take precedence.
- “Underperforming assets” in the draft must not imply recommendations to change asset allocation: the user selected better opportunities for assets already held.
- The old append-only transaction statement does not describe the current stable-ID replacement ledger. Whether an immutable revision/audit history is desired remains a separate design question.
- Timing is now confirmed for tax reporting, spending, sharing, mobile/sync, and invoicing. Specific tax filing support, AI spending classification, advertising, and workflow details remain unselected.

Continue the interview one question at a time. Resolve remaining workflow details before adding model fields; user-confirmed timing is recorded in the roadmap.
