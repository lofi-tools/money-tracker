# Risk comparison research

Researched 11 October 2026 in response to the [user needs interview](user-needs.md). The user confirmed a same-asset opportunity catalogue with product and platform risks. The application suggestions below are proposals, not selected methodology or implemented behavior.

## Institutional approaches

Banks separate credit, market, operational, liquidity, and concentration risks rather than relying on one universal score. Credit models estimate probability of default (PD), loss given default (LGD), and exposure at default (EAD). Expected credit loss is commonly represented as PD × LGD × EAD over a specified horizon; unexpected losses also matter for capital. [Basel credit-risk components](https://www.bis.org/committees/bcbs/basel-consolidated-guidelines/module/cri/30), [OSFI expected-loss reporting](https://www.osfi-bsif.gc.ca/en/data-forms/reporting-returns/financial-reporting-instructions/2026-basel-capital-adequacy-reporting-bcar).

Investment managers use measures such as Sharpe (excess return divided by volatility), Sortino (return above a target relative to downside deviation), and information ratio (active return relative to tracking error). These describe different dimensions of observed performance and require consistent periods, return units, and benchmarks. [CFA risk-adjusted performance overview](https://rpc.cfainstitute.org/topics/performance-attribution/risk-adjusted-performance), [CFA Sharpe discussion](https://rpc.cfainstitute.org/research/cfa-digest/2010/02/refining-the-sharpe-ratio-digest-summary), [CFA Sortino discussion](https://rpc.cfainstitute.org/-/media/documents/code/gips/the-sortino-ratio.pdf).

Tail-risk analysis uses value at risk (a loss threshold at a specified confidence and horizon) and expected shortfall (average losses beyond that threshold). Liquidity analysis considers whether obligations and collateral needs can be met under stress. Neither a volatility ratio nor an expected loss number is a complete risk description. [Basel market-risk definitions](https://www.bis.org/committees/bcbs/basel-framework/standard/mar?allChapters=true), [Basel liquidity principles](https://www.bis.org/committees/bcbs/basel-consolidated-guidelines/module/lqy/10).

DeFi introduces or amplifies dependencies on smart contracts, governance, blockchains, oracles, bridges, leverage, and liquidity mismatches. These need explicit assessment alongside ordinary financial risks. [BIS DeFi risk summary](https://www.bis.org/publications/fsi-summary-financial-stability-risks-decentralised-finance-executive-summary).

## Proposed application to this software

Compare the incremental return and additional risks of putting the same asset into different products against simply holding that asset. Common price exposure does not eliminate product-specific liquidation, collateral, receipt-token, or reward-currency risks.

Show expected net yield, principal at risk, custody/counterparty exposure, withdrawal restrictions, leverage/liquidation conditions, technical dependencies, and shared platform/protocol concentration. Attach evidence sources, assessment date, and uncertainty to risk assessments. Unknown risk should remain unknown rather than becoming a low-risk score.

A stable quoted APY or smooth historical earnings series can conceal a rare default or exploit. A Sharpe ratio computed from interest credits alone would miss principal losses; model whole-position returns if using performance ratios. Historical ratios cannot establish future safety.

Do not invent default/exploit probabilities to produce precise adjusted yields. Where evidence supports estimates, show assumptions and sensitivity. An illustrative one-year approximation is net yield minus probability of loss times loss fraction. This is an expected-return adjustment, not a full risk score, and does not capture loss timing, foregone yield, liquidity costs, or correlated failures. More defensible initial presentation may be a transparent risk breakdown plus stress scenarios; the user has not yet selected that approach.
