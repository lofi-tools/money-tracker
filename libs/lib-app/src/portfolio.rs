//! One snapshot drives overview, navigation, allocation and scoped details.
use crate::{Application, dto::*, exact::ExactRatio};
use anyhow::{Result, ensure};
use chrono::{DateTime, Utc};
use lib_core::{AccountId, AssetId, PositionId, PositionRole, ProviderId, Units, ui_metadata::*};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortfolioRequest {
    pub context: ReadContext,
    pub reporting_asset: AssetId,
    pub dust_threshold: ExactDecimal,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalculatedMoney {
    pub asset_id: AssetId,
    pub amount: ExactRatio,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Summary {
    /// Standalone summaries retain the qualifications of this estimate.
    pub calculation_basis: String,
    pub gross_assets: FigureDto<CalculatedMoney>,
    pub liabilities: FigureDto<CalculatedMoney>,
    pub current_value: FigureDto<CalculatedMoney>,
    pub gross_annual_earnings: FigureDto<CalculatedMoney>,
    pub annual_borrowing_costs: FigureDto<CalculatedMoney>,
    pub net_annual_earnings: FigureDto<CalculatedMoney>,
    /// Net earnings / gross assets, never net worth. Exact annual fraction.
    pub current_apy: FigureDto<ExactRatio>,
    /// Display estimate only: missing investment rates count as zero, never borrowing costs.
    pub estimated_current_apy: FigureDto<ExactRatio>,
    pub historical_effective_apy: FigureDto<ExactRatio>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CurrentHolding {
    #[serde(default)]
    pub inferred_allocations: Vec<crate::adapters::nexo::allocations::TermAllocation>,
    #[serde(default)]
    pub display_provider: Option<ProviderId>,
    #[serde(default)]
    pub display_provider_name: Option<String>,
    pub id: String,
    pub account_id: AccountId,
    pub provider_id: ProviderId,
    pub position_ids: Vec<PositionId>,
    pub role: PositionRole,
    pub quantity: QuantityDto,
    pub ledger_quantities: Vec<LedgerPositionDto>,
    pub subscription: Option<Subscription>,
    pub label: String,
    pub value: FigureDto<CalculatedMoney>,
    pub price: Option<SelectedPrice>,
    pub rate: Option<SelectedRate>,
    pub rate_scope: Option<RateScope>,
    pub rate_evidence: Vec<EvidenceDto>,
    pub annual_earnings: FigureDto<CalculatedMoney>,
    pub annual_borrowing_cost: FigureDto<CalculatedMoney>,
    pub summary: Summary,
    pub is_dust: bool,
    pub evidence: Vec<EvidenceDto>,
    pub verification: Option<VerificationRecord>,
    pub verification_current: bool,
    pub issues: Vec<IssueDto>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum Selection {
    Overview,
    Asset(AssetId),
    Exposure(AssetId),
    Provider(ProviderId),
    Dust,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlternativeRepresentation {
    pub name: String,
    pub ticker: String,
    pub provider: ProviderId,
    pub role: PositionRole,
    pub quantity: QuantityDto,
    pub value: FigureDto<CalculatedMoney>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NavigationEntry {
    /// Summed direct native quantity in display-only underlying units.
    #[serde(default)]
    pub direct_quantities: Option<Vec<QuantityDto>>,
    #[serde(default)]
    pub alternatives: Vec<AlternativeRepresentation>,
    /// Display-only signed valuation; never relaxes strict summary/reconciliation.
    #[serde(default)]
    pub estimated_current_value: Option<FigureDto<CalculatedMoney>>,
    pub selection: Selection,
    pub name: String,
    pub ticker: Option<String>,
    #[serde(default)]
    pub token_metadata: Option<lib_core::token_metadata::TokenMetadata>,
    #[serde(default)]
    pub metadata_unresolved: bool,
    /// Original exact-asset quantities, partitioned by role; never converted to underlying units.
    pub quantities: Vec<(PositionRole, QuantityDto)>,
    pub summary: Summary,
    pub issues: Vec<IssueDto>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum AllocationTarget {
    Provider(ProviderId),
    Account(AccountId),
    Product {
        account: AccountId,
        product: lib_core::ProductId,
    },
    Subscription(String),
    Holding(String),
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AllocationNode {
    #[serde(default)]
    pub inferred_allocation: Option<crate::adapters::nexo::allocations::TermAllocation>,
    pub id: String,
    pub target: AllocationTarget,
    pub parent: Option<String>,
    pub children: Vec<String>,
    pub name: String,
    pub holding_ids: Vec<String>,
    pub summary: Summary,
    pub issues: Vec<IssueDto>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AllocationTree {
    pub selection: Selection,
    pub roots: Vec<String>,
    pub nodes: Vec<AllocationNode>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Details {
    pub node: AllocationNode,
    pub holdings: Vec<CurrentHolding>,
    pub original_account: Option<AccountDto>,
    pub product: Option<ProductTemplate>,
    pub subscription: Option<Subscription>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EarningsContribution {
    pub holding_ids: Vec<String>,
    pub scope: RateScope,
    pub selected_rate: SelectedRate,
    pub gross_earnings: FigureDto<CalculatedMoney>,
    pub borrowing_costs: FigureDto<CalculatedMoney>,
    pub evidence: Vec<EvidenceDto>,
    pub issues: Vec<IssueDto>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortfolioProjection {
    pub contract_version: u32,
    pub request: PortfolioRequest,
    pub revisions: PortfolioRevisions,
    pub overview: Summary,
    pub assets: Vec<NavigationEntry>,
    pub providers: Vec<NavigationEntry>,
    pub dust: NavigationEntry,
    pub holdings: Vec<CurrentHolding>,
    pub contributions: Vec<EarningsContribution>,
    pub allocation: AllocationTree,
    pub metadata: UiMetadataDto,
    pub issues: Vec<IssueDto>,
    pub calculation_basis: String,
}
const SUMMARY_BASIS: &str = "current_rate_one_year; constant_principal_and_quotes; net_of_evidenced_borrowing_only; additional_fees_and_historical_reward_adjustment_not_estimated";

fn issue(code: &str, scope: EntityScope, message: &str, withheld: Vec<WithheldFigure>) -> IssueDto {
    IssueDto {
        code: code.into(),
        scope,
        message: message.into(),
        withheld,
    }
}
fn hscope(h: &CurrentHolding) -> EntityScope {
    h.position_ids.first().map_or_else(
        || EntityScope::Account {
            account_id: h.account_id.clone(),
        },
        |id| EntityScope::Position {
            position_id: id.clone(),
        },
    )
}
fn money(quote: &AssetId, amount: ExactRatio) -> CalculatedMoney {
    CalculatedMoney {
        asset_id: quote.clone(),
        amount,
    }
}
fn known<T>(value: T, evidence: Vec<EvidenceDto>) -> FigureDto<T> {
    FigureDto::Known { value, evidence }
}
fn unknown<T>(issues: Vec<IssueDto>) -> FigureDto<T> {
    FigureDto::Unknown { issues }
}
fn amount(f: &FigureDto<CalculatedMoney>) -> Option<ExactRatio> {
    match f {
        FigureDto::Known { value, .. } | FigureDto::Partial { value, .. } => {
            Some(value.amount.clone())
        }
        _ => None,
    }
}
fn complete<T>(f: &FigureDto<T>) -> bool {
    matches!(f, FigureDto::Known { .. } | FigureDto::NotApplicable)
}
fn evidence<T>(f: &FigureDto<T>) -> Vec<EvidenceDto> {
    match f {
        FigureDto::Known { evidence, .. } | FigureDto::Partial { evidence, .. } => evidence.clone(),
        _ => vec![],
    }
}
fn figure_issues<T>(f: &FigureDto<T>) -> Vec<IssueDto> {
    match f {
        FigureDto::Unknown { issues } | FigureDto::Partial { issues, .. } => issues.clone(),
        _ => vec![],
    }
}
fn dedup<T: PartialEq>(v: Vec<T>) -> Vec<T> {
    let mut out = vec![];
    for x in v {
        if !out.contains(&x) {
            out.push(x)
        }
    }
    out
}
fn sum(quote: &AssetId, figures: Vec<FigureDto<CalculatedMoney>>) -> FigureDto<CalculatedMoney> {
    if figures.is_empty() {
        return known(money(quote, ExactRatio::zero()), vec![]);
    }
    let mut total = ExactRatio::zero();
    let mut has = false;
    let mut all = true;
    let mut ev = vec![];
    let mut issues = vec![];
    for f in figures {
        if let Some(x) = amount(&f) {
            total = total.add(&x);
            has = true
        }
        all &= complete(&f);
        ev.extend(evidence(&f));
        issues.extend(figure_issues(&f));
    }
    if all {
        known(money(quote, total), dedup(ev))
    } else if has {
        FigureDto::Partial {
            value: money(quote, total),
            evidence: dedup(ev),
            issues: dedup(issues),
        }
    } else {
        unknown(dedup(issues))
    }
}
fn subtract(
    quote: &AssetId,
    a: FigureDto<CalculatedMoney>,
    b: FigureDto<CalculatedMoney>,
) -> FigureDto<CalculatedMoney> {
    let mut ev = evidence(&a);
    ev.extend(evidence(&b));
    let mut issues = figure_issues(&a);
    issues.extend(figure_issues(&b));
    let value = match (amount(&a), amount(&b)) {
        (Some(a), Some(b)) => Some(a.sub(&b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b.neg()),
        _ => None,
    };
    match value {
        Some(v) if complete(&a) && complete(&b) => known(money(quote, v), dedup(ev)),
        Some(v) if v.is_zero() && ev.is_empty() => unknown(dedup(issues)),
        Some(v) => FigureDto::Partial {
            value: money(quote, v),
            evidence: dedup(ev),
            issues: dedup(issues),
        },
        None => unknown(dedup(issues)),
    }
}
fn empty_summary(quote: &AssetId) -> Summary {
    let z = known(money(quote, ExactRatio::zero()), vec![]);
    Summary {
        calculation_basis: SUMMARY_BASIS.into(),
        gross_assets: z.clone(),
        liabilities: z.clone(),
        current_value: z.clone(),
        gross_annual_earnings: z.clone(),
        annual_borrowing_costs: z.clone(),
        net_annual_earnings: z,
        current_apy: FigureDto::NotApplicable,
        estimated_current_apy: FigureDto::NotApplicable,
        historical_effective_apy: FigureDto::NotApplicable,
    }
}
fn rate_factor(rate: &AnnualRate) -> Option<ExactRatio> {
    let r = ExactRatio::from_decimal(&rate.fraction);
    match (&rate.basis, rate.compounds_per_year) {
        (AnnualRateBasis::Apr, Some(n)) if n <= 366 => Some(
            ExactRatio::integer(1)
                .add(&r.div(&ExactRatio::integer(n as i32))?)
                .pow(n)
                .sub(&ExactRatio::integer(1)),
        ),
        (AnnualRateBasis::Apr, Some(_)) => None,
        _ => Some(r),
    }
}
fn contribution_for(
    c: &EarningsContribution,
    ids: &BTreeSet<String>,
    scope: EntityScope,
    cost: bool,
) -> FigureDto<CalculatedMoney> {
    let overlap = c.holding_ids.iter().filter(|id| ids.contains(*id)).count();
    if overlap == 0 {
        return FigureDto::NotApplicable;
    }
    if overlap == c.holding_ids.len() {
        return if cost {
            c.borrowing_costs.clone()
        } else {
            c.gross_earnings.clone()
        };
    }
    unknown(vec![issue(
        "aggregate_earnings_unattributed",
        scope,
        "Aggregate rate earnings cannot be split into child allocations without evidenced attribution",
        vec![WithheldFigure::CurrentEarnings, WithheldFigure::OverallApy],
    )])
}
fn summary(
    request: &PortfolioRequest,
    hs: &[&CurrentHolding],
    cs: &[EarningsContribution],
) -> Summary {
    let quote = &request.reporting_asset;
    let assets: Vec<_> = hs
        .iter()
        .filter(|h| h.role == PositionRole::Asset)
        .collect();
    let debts: Vec<_> = hs
        .iter()
        .filter(|h| h.role == PositionRole::Liability)
        .collect();
    let gross = sum(quote, assets.iter().map(|h| h.value.clone()).collect());
    let liabilities = sum(quote, debts.iter().map(|h| h.value.clone()).collect());
    let ids: BTreeSet<_> = hs.iter().map(|h| h.id.clone()).collect();
    let detail_scope = if hs.is_empty() {
        EntityScope::Portfolio {
            portfolio_id: request.context.portfolio_id.clone(),
        }
    } else if hs.len() == 1 {
        hscope(hs[0])
    } else if hs.iter().all(|h| h.account_id == hs[0].account_id) {
        EntityScope::Account {
            account_id: hs[0].account_id.clone(),
        }
    } else {
        EntityScope::Portfolio {
            portfolio_id: request.context.portfolio_id.clone(),
        }
    };
    let mut earning = vec![];
    let mut costs = vec![];
    let mut covered = BTreeSet::new();
    for c in cs {
        if c.holding_ids.iter().any(|id| ids.contains(id)) {
            covered.extend(c.holding_ids.iter().cloned());
            if matches!(c.scope, RateScope::BorrowingPosition(_)) {
                costs.push(contribution_for(c, &ids, detail_scope.clone(), true))
            } else {
                earning.push(contribution_for(c, &ids, detail_scope.clone(), false))
            }
        }
    }
    for h in hs {
        if !covered.contains(&h.id) {
            let f = unknown(vec![issue(
                "rate_unknown",
                hscope(h),
                "No applicable evidenced rate; unknown is not zero",
                vec![WithheldFigure::CurrentEarnings, WithheldFigure::OverallApy],
            )]);
            if h.role == PositionRole::Liability {
                costs.push(f)
            } else {
                earning.push(f)
            }
        }
    }
    let earning = sum(quote, earning);
    let costs = sum(quote, costs);
    let net = subtract(quote, earning.clone(), costs.clone());
    let source_apy_issues: Vec<_> = hs
        .iter()
        .flat_map(|h| {
            let codes: Vec<_> = h
                .verification
                .as_ref()
                .map(|r| {
                    r.issues
                        .iter()
                        .filter(|i| {
                            i.withheld.iter().any(|w| w == "overall_apy")
                                && (i.position.is_none()
                                    || i.position
                                        .as_ref()
                                        .is_some_and(|id| h.position_ids.contains(id)))
                        })
                        .map(|i| i.code.as_str())
                        .collect()
                })
                .unwrap_or_default();
            h.issues
                .iter()
                .filter(move |i| codes.contains(&i.code.as_str()))
                .cloned()
        })
        .collect();
    let apy = if hs.is_empty() || (complete(&gross) && amount(&gross).is_some_and(|v| v.is_zero()))
    {
        FigureDto::NotApplicable
    } else if !source_apy_issues.is_empty() {
        unknown(dedup(source_apy_issues.clone()))
    } else if complete(&gross) && complete(&net) {
        match (amount(&net), amount(&gross)) {
            (Some(n), Some(g)) => n.div(&g).map_or(FigureDto::NotApplicable, |v| {
                known(v, dedup([evidence(&gross), evidence(&net)].concat()))
            }),
            _ => FigureDto::NotApplicable,
        }
    } else {
        unknown(dedup([figure_issues(&gross), figure_issues(&net)].concat()))
    };
    let missing = figure_issues(&earning);
    let estimated = if matches!(apy, FigureDto::Unknown { .. })
        && source_apy_issues.is_empty()
        && complete(&gross)
        && complete(&costs)
        && !missing.is_empty()
        && missing.iter().all(|i| i.code == "rate_unknown")
    {
        match (amount(&gross), amount(&costs)) {
            (Some(g), Some(c)) => amount(&earning).unwrap_or_else(ExactRatio::zero).sub(&c).div(&g)
                .map(|value| FigureDto::Partial {
                    value, evidence: dedup([evidence(&gross), evidence(&earning), evidence(&costs)].concat()),
                    issues: vec![issue("unknown_rate_assumed_zero", detail_scope.clone(), "Missing investment rates assumed zero for this display estimate; individual rates remain unknown", vec![])],
                }).unwrap_or(FigureDto::NotApplicable),
            _ => apy.clone(),
        }
    } else {
        apy.clone()
    };
    Summary {
        calculation_basis: SUMMARY_BASIS.into(),
        current_value: subtract(quote, gross.clone(), liabilities.clone()),
        gross_assets: gross,
        liabilities,
        gross_annual_earnings: earning,
        annual_borrowing_costs: costs,
        net_annual_earnings: net,
        current_apy: apy,
        estimated_current_apy: estimated,
        historical_effective_apy: if hs.is_empty() {
            FigureDto::NotApplicable
        } else {
            unknown(vec![issue(
                "historical_reward_adjustment_unavailable",
                EntityScope::Portfolio {
                    portfolio_id: request.context.portfolio_id.clone(),
                },
                "Historical reward retention/disposal and cash-flow analysis is not available",
                vec![WithheldFigure::HistoricalEarnings],
            )])
        },
    }
}
impl Application {
    pub fn portfolio_projection(&self, request: PortfolioRequest) -> Result<PortfolioProjection> {
        ensure!(
            !request.dust_threshold.is_negative(),
            "dust threshold must be nonnegative"
        );
        ensure!(
            !request.reporting_asset.0.trim().is_empty(),
            "reporting asset required"
        );
        self.store()
            .read_snapshot(|| self.project_snapshot(request))
    }
    fn project_snapshot(&self, request: PortfolioRequest) -> Result<PortfolioProjection> {
        let store = self.store();
        let p = &request.context.portfolio_id;
        let cutoff = request.context.cutoff;
        let metadata = self.ui_metadata(p)?;
        let ledger = self.ledger_balances(request.context.clone())?;
        let reports = store.verifications(p)?;
        let mut holdings = vec![];
        for account in &metadata.accounts {
            let aid = &account.account.id;
            let provider = &account.account.provider_id;
            let report = reports
                .iter()
                .filter(|r| r.account == *aid && r.cutoff <= cutoff)
                .max_by_key(|r| (&r.cutoff, &r.id))
                .cloned();
            let report_current = report
                .as_ref()
                .map(|r| store.verification_is_current(r))
                .transpose()?
                .unwrap_or(false);
            let observations = store.balance_evidence(aid)?;
            let usable: Vec<_> = observations
                .iter()
                .filter(|o| {
                    o.cutoff <= cutoff
                        && o.complete
                        && o.independent
                        && matches!(o.basis, BalanceBasis::Total | BalanceBasis::Debt)
                })
                .collect();
            let mut groups: BTreeMap<(String, bool), Vec<LedgerPositionDto>> = BTreeMap::new();
            for l in ledger.positions.iter().filter(|l| l.account_id == *aid) {
                groups
                    .entry((
                        l.quantity.asset_id.0.clone(),
                        l.role == PositionRole::Liability,
                    ))
                    .or_default()
                    .push(l.clone());
            }
            for (id, asset, scale, role) in store.account_position_metadata(aid)? {
                if role == PositionRole::ExternalCounterparty
                    || (role == PositionRole::Asset && store.asset_is_excluded(aid, &asset)?)
                {
                    continue;
                }
                let group = groups
                    .entry((asset.0.clone(), role == PositionRole::Liability))
                    .or_default();
                if !group.iter().any(|l| l.position_id == id) {
                    group.push(LedgerPositionDto {
                        position_id: id,
                        account_id: aid.clone(),
                        role,
                        quantity: QuantityDto {
                            asset_id: asset,
                            units: Units::from(0),
                            scale,
                        },
                    });
                }
            }
            for o in &usable {
                if !store.asset_is_excluded(aid, &o.asset)? || o.basis == BalanceBasis::Debt {
                    groups
                        .entry((o.asset.0.clone(), o.basis == BalanceBasis::Debt))
                        .or_default();
                }
            }
            for ((asset, debt), ls) in groups {
                let asset = AssetId::str(&asset);
                let role = if debt {
                    PositionRole::Liability
                } else {
                    PositionRole::Asset
                };
                let obs: Vec<_> = usable
                    .iter()
                    .copied()
                    .filter(|o| o.asset == asset && (o.basis == BalanceBasis::Debt) == debt)
                    .filter(|o| {
                        if debt {
                            o.units.is_negative() || o.units.is_zero()
                        } else {
                            !o.units.is_negative()
                        }
                    })
                    .collect();
                let latest_aggregate = obs
                    .iter()
                    .copied()
                    .filter(|o| o.position.is_none())
                    .max_by_key(|o| (&o.cutoff, &o.evidence.updated_at, &o.id));
                let latest_position = obs
                    .iter()
                    .copied()
                    .filter(|o| o.position.is_some())
                    .map(|o| o.cutoff)
                    .max();
                // Never split a differing account total across subscription buckets.
                if let Some(o) =
                    latest_aggregate.filter(|o| latest_position.is_none_or(|t| o.cutoff >= t))
                {
                    if ls.len() != 1 {
                        if !o.units.is_zero() {
                            holdings.push(self.make_holding(
                                &request,
                                &metadata,
                                aid,
                                provider,
                                role,
                                QuantityDto {
                                    asset_id: asset,
                                    units: o.units.clone(),
                                    scale: o.scale,
                                },
                                ls,
                                None,
                                Some(o),
                                report.clone(),
                                report_current,
                            )?);
                        }
                        continue;
                    }
                }
                for l in ls {
                    let o = obs
                        .iter()
                        .copied()
                        .filter(|o| o.position.as_ref() == Some(&l.position_id))
                        .max_by_key(|o| (&o.cutoff, &o.evidence.updated_at, &o.id))
                        .or_else(|| {
                            latest_aggregate
                                .filter(|o| latest_position.is_none_or(|t| o.cutoff >= t))
                        });
                    let q = o.map_or_else(
                        || l.quantity.clone(),
                        |o| QuantityDto {
                            asset_id: o.asset.clone(),
                            units: o.units.clone(),
                            scale: o.scale,
                        },
                    );
                    if q.units.is_zero() {
                        continue;
                    }
                    let sub = metadata
                        .subscriptions
                        .iter()
                        .find(|s| s.positions.contains(&l.position_id))
                        .cloned();
                    holdings.push(self.make_holding(
                        &request,
                        &metadata,
                        aid,
                        provider,
                        role,
                        q,
                        vec![l],
                        sub,
                        o,
                        report.clone(),
                        report_current,
                    )?);
                }
            }
        }
        self.attach_nexo_allocations(&request, &mut holdings)?;
        let contributions = self.earnings_contributions(&request, &holdings)?;
        for h in &mut holdings {
            let hs = vec![&*h];
            h.summary = summary(&request, &hs, &contributions);
            h.annual_earnings = if h.role == PositionRole::Asset {
                h.summary.gross_annual_earnings.clone()
            } else {
                FigureDto::NotApplicable
            };
            h.annual_borrowing_cost = if h.role == PositionRole::Liability {
                h.summary.annual_borrowing_costs.clone()
            } else {
                FigureDto::NotApplicable
            };
            let relevant: Vec<_> = contributions
                .iter()
                .filter(|c| c.holding_ids.contains(&h.id))
                .collect();
            if relevant.len() == 1 {
                let c = relevant[0];
                h.rate = Some(c.selected_rate.clone());
                h.rate_scope = Some(c.scope.clone());
                h.rate_evidence = c.evidence.clone();
                h.issues.extend(c.issues.clone());
            }
            h.issues.extend(figure_issues(&h.annual_earnings));
            h.issues.extend(figure_issues(&h.annual_borrowing_cost));
            h.issues = dedup(std::mem::take(&mut h.issues));
        }
        for h in &mut holdings {
            if h.provider_id.0 == "evm" {
                if let Some(software) = metadata.local_state.iter().find_map(|(target, record)| {
                    match (target, &record.value) {
                        (LocalTarget::Account { id }, Some(LocalValue::Account(value)))
                            if id == &h.account_id =>
                        {
                            value.wallet_software.clone()
                        }
                        _ => None,
                    }
                }) {
                    h.display_provider = Some(ProviderId::from(
                        format!("wallet-software:{}", software.trim().to_lowercase()).as_str(),
                    ));
                    h.display_provider_name = Some(if software.eq_ignore_ascii_case("metamask") {
                        "MetaMask".into()
                    } else {
                        software
                    });
                }
            }
        }
        // Classification is provider/asset-wide, independent of node expansion.
        let mut pa: BTreeMap<(String, String), Vec<usize>> = BTreeMap::new();
        for (i, h) in holdings
            .iter()
            .enumerate()
            .filter(|(_, h)| h.role == PositionRole::Asset)
        {
            pa.entry((h.provider_id.0.clone(), h.quantity.asset_id.0.clone()))
                .or_default()
                .push(i);
        }
        for ((provider, asset), indices) in pa {
            let state = store.local_state(
                p,
                &LocalTarget::Dust {
                    provider: provider.as_str().into(),
                    asset: AssetId::str(&asset),
                },
            )?;
            let f = sum(
                &request.reporting_asset,
                indices.iter().map(|i| holdings[*i].value.clone()).collect(),
            );
            let unresolved =
                crate::token_metadata::display_identity(store, &AssetId::str(&asset))?.2;
            let dust = unresolved
                || amount(&f).is_none()
                || match state.value {
                    Some(LocalValue::Dust(DustMode::Dust)) => true,
                    Some(LocalValue::Dust(DustMode::Significant)) => false,
                    _ => {
                        complete(&f)
                            && amount(&f).is_some_and(|v| {
                                !v.is_negative()
                                    && v < ExactRatio::from_decimal(&request.dust_threshold)
                            })
                    }
                };
            for i in indices {
                holdings[i].is_dust = dust;
            }
        }
        let overview = summary(
            &request,
            &holdings.iter().collect::<Vec<_>>(),
            &contributions,
        );
        let mut asset_ids = BTreeSet::new();
        let mut provider_ids = BTreeSet::new();
        for h in &holdings {
            asset_ids.insert(h.quantity.asset_id.0.clone());
            provider_ids.insert(
                h.display_provider
                    .as_ref()
                    .unwrap_or(&h.provider_id)
                    .0
                    .clone(),
            );
        }
        let mut assets = vec![];
        for id in asset_ids {
            let asset_id = AssetId::str(&id);
            let metadata = store.token_metadata(&asset_id)?;
            let (name, ticker, unresolved) =
                crate::token_metadata::display_identity(store, &asset_id)?;
            let mut entry = nav(
                &request,
                &holdings,
                &contributions,
                Selection::Asset(asset_id),
                name,
                ticker,
            );
            entry.metadata_unresolved = unresolved
                && entry
                    .quantities
                    .iter()
                    .all(|(role, _)| *role != PositionRole::Liability);
            entry.token_metadata = metadata;
            assets.push(entry);
        }
        let mut providers = vec![];
        for id in provider_ids {
            let name = holdings
                .iter()
                .find(|h| h.display_provider.as_ref().is_some_and(|p| p.0 == id))
                .and_then(|h| h.display_provider_name.clone())
                .unwrap_or(provider_display_name(
                    &ProviderId(id.clone()),
                    store.provider_display(&ProviderId(id.clone()))?.as_ref(),
                ));
            providers.push(nav(
                &request,
                &holdings,
                &contributions,
                Selection::Provider(id.as_str().into()),
                name,
                None,
            ));
        }
        sort_nav(&mut assets);
        sort_nav(&mut providers);
        let dust = nav(
            &request,
            &holdings,
            &contributions,
            Selection::Dust,
            "Others".into(),
            None,
        );
        let allocation = build_tree(
            &request,
            &metadata,
            &holdings,
            &contributions,
            Selection::Overview,
        )?;
        let issues = dedup(holdings.iter().flat_map(|h| h.issues.clone()).collect());
        Ok(PortfolioProjection{contract_version:CONTRACT_VERSION,request,revisions:metadata.revisions.clone(),overview,assets,providers,dust,holdings,contributions,allocation,metadata,issues,calculation_basis:"current_rate_one_year; constant_principal_and_quotes; gross_yield_minus_evidenced_borrowing_costs; additional_fees_and_switching_not_estimated; historical_reward_adjustment_unavailable".into()})
    }
    fn attach_nexo_allocations(
        &self,
        request: &PortfolioRequest,
        holdings: &mut [CurrentHolding],
    ) -> Result<()> {
        use crate::adapters::nexo::{NEXO_LEDGER_SOURCE, allocations};
        let account = AccountId::from("legacy:nexo");
        let mut snapshots = vec![];
        for (hash, payload) in self
            .store()
            .legacy_source_snapshots(&account, NEXO_LEDGER_SOURCE)?
        {
            let rows = nexo_csv::NexoCsv::from_reader(payload.as_bytes())?;
            if let Some(as_of) = rows
                .iter()
                .map(|r| r.date_time_utc)
                .max()
                .filter(|t| *t <= request.context.cutoff)
            {
                snapshots.push((as_of, hash, rows));
            }
        }
        let Some((_, hash, rows)) = snapshots
            .into_iter()
            .max_by(|a, b| (&a.0, &a.1).cmp(&(&b.0, &b.1)))
        else {
            return Ok(());
        };
        let source_cutoff = rows
            .iter()
            .map(|r| r.date_time_utc)
            .max()
            .unwrap_or(request.context.cutoff)
            .min(request.context.cutoff);
        let result = allocations::open_allocations(&rows, &account, &hash, source_cutoff);
        for h in holdings
            .iter_mut()
            .filter(|h| h.account_id == account && h.role == PositionRole::Asset)
        {
            let parts = result
                .as_ref()
                .map_err(|_| ())
                .and_then(|parts| allocations::partition(&h.quantity, parts).map_err(|_| ()));
            match parts {
                Ok(mut parts) => {
                    for part in &mut parts {
                        let flexible = part.kind=="unclassified_residual";
                        if !flexible {part.captured=self.captured_nexo_term(&part.account,&part.id,request.context.cutoff)?;}
                        let product=lib_core::ProductId::from(format!("nexo-{}:{}:{}",if flexible {"flex"}else{"fixed"},part.account.0,part.quantity.asset_id.0));
                        let group_product=lib_core::ProductId::from(format!("nexo-fixed-group:{}:{}",part.id,part.count));
                        let mut selected=if part.kind=="fixed" {self.store().selected_rate(&request.context.portfolio_id,&RateScope::Subscription(part.id.clone()),request.context.cutoff)?} else {None};
                        let mut expected_product=product.clone();
                        let mut expected_scope=if selected.is_some() {RateScope::Subscription(part.id.clone())} else {RateScope::Product(product.clone())};
                        if selected.is_none() && self.store().product_template(&group_product)?.is_some() {
                            selected=self.store().selected_rate(&request.context.portfolio_id,&RateScope::Product(group_product.clone()),request.context.cutoff)?;
                            if selected.is_some(){expected_scope=RateScope::Product(group_product.clone());expected_product=group_product;}
                        }
                        if selected.is_none() && self.store().product_template(&product)?.is_some() && (!flexible || self.store().subscriptions(&request.context.portfolio_id)?.iter().any(|s|s.product_id==product && s.account_id==part.account)){selected=self.store().selected_rate(&request.context.portfolio_id,&RateScope::Product(product),request.context.cutoff)?;}
                        if selected.as_ref().and_then(|r|r.evidence_id.as_ref()).is_some_and(|id|id.starts_with("nexo-browser:")) {part.expected_rate_note=Some("Captured current Nexo rate".into());}
                        if let Some(rate)=&selected {part.expected_evidence=self.rate_provenance(&request.context.portfolio_id,&expected_scope,rate)?;}
                        part.expected_apy=selected.and_then(|r|r.rate.as_ref().and_then(rate_factor));
                        if let Some(product) = self.store().product_template(&expected_product)? {
                            if part.expected_apy.is_some() && part.expected_rate_note.is_none(){part.expected_rate_note=Some(product.label);}
                            if !flexible {
                                part.maturity = self.store().subscription(&part.id)?.and_then(|s|s.maturity);
                                part.expected_maturity = part.opened_at.zip(product.term_months)
                                    .and_then(|(start,months)|start.checked_add_months(chrono::Months::new(months)));
                            }
                        }
                    }
                    h.inferred_allocations=parts;
                },
                Err(_)=>h.issues.push(issue("nexo_allocation_unresolved",EntityScope::Account {account_id:account.clone()},"Term movements cannot be partitioned against the current aggregate; source history retained",vec![]))
            }
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn make_holding(
        &self,
        request: &PortfolioRequest,
        _metadata: &UiMetadataDto,
        account: &AccountId,
        provider: &ProviderId,
        role: PositionRole,
        quantity: QuantityDto,
        ledger: Vec<LedgerPositionDto>,
        mut subscription: Option<Subscription>,
        observation: Option<&BalanceEvidence>,
        verification: Option<VerificationRecord>,
        verification_current: bool,
    ) -> Result<CurrentHolding> {
        let p = &request.context.portfolio_id;
        let cutoff = request.context.cutoff;
        let store = self.store();
        let positions: Vec<_> = ledger.iter().map(|l| l.position_id.clone()).collect();
        let id = serde_json::to_string(&(
            account,
            &quantity.asset_id,
            role,
            if ledger.len() == 1 {
                positions.clone()
            } else {
                vec![]
            },
            ledger.len() != 1,
        ))?;
        let scope = positions.first().map_or_else(
            || EntityScope::Account {
                account_id: account.clone(),
            },
            |id| EntityScope::Position {
                position_id: id.clone(),
            },
        );
        let mut issues = vec![];
        let mut ev = vec![];
        if let Some(o) = observation {
            ev.push(provider_evidence(&o.evidence, Some(o.cutoff)));
            if o.cutoff != cutoff {
                issues.push(issue("balance_as_of",scope.clone(),"Independent quantity has its own dated cutoff; not verified at the requested common cutoff",vec![]));
            }
            if o.position.is_none() && ledger.len() != 1 {
                subscription = None;
                issues.push(issue(
                    "allocation_unattributed",
                    scope.clone(),
                    "Independent account total does not establish subscription allocations",
                    vec![],
                ));
            }
        } else {
            ev.push(EvidenceDto::LedgerReplay { cutoff });
        }
        let verified = verification.as_ref().is_some_and(|r| {
            verification_current
                && r.outcome == VerificationOutcome::Verified
                && r.cutoff == cutoff
                && r.history_complete
        });
        if !verified {
            issues.push(issue(
                "history_unverified",
                scope.clone(),
                "Historical totals and received yield require reconciled complete history",
                vec![
                    WithheldFigure::HistoricalValue,
                    WithheldFigure::HistoricalEarnings,
                ],
            ));
        }
        if let Some(r) = &verification {
            if !verification_current {
                issues.push(issue(
                    "verification_stale",
                    scope.clone(),
                    "Ledger or tracking policy changed after this verification",
                    vec![
                        WithheldFigure::HistoricalValue,
                        WithheldFigure::HistoricalEarnings,
                    ],
                ));
            }
            for i in &r.issues {
                if i.position.is_none()
                    || i.position.as_ref().is_some_and(|x| positions.contains(x))
                {
                    issues.push(issue(
                        &i.code,
                        i.position.as_ref().map_or_else(
                            || EntityScope::Account {
                                account_id: account.clone(),
                            },
                            |id| EntityScope::Position {
                                position_id: id.clone(),
                            },
                        ),
                        &i.message,
                        i.withheld
                            .iter()
                            .filter_map(|s| match s.as_str() {
                                "current_value" => Some(WithheldFigure::CurrentValue),
                                "current_earnings" => Some(WithheldFigure::CurrentEarnings),
                                "overall_apy" => Some(WithheldFigure::OverallApy),
                                "historical_value" => Some(WithheldFigure::HistoricalValue),
                                "historical_earnings" => Some(WithheldFigure::HistoricalEarnings),
                                _ => None,
                            })
                            .collect(),
                    ));
                }
            }
        }
        if subscription
            .as_ref()
            .is_some_and(|s| s.closed_at.is_some_and(|t| t <= cutoff))
        {
            subscription = None;
            issues.push(issue("closed_subscription_nonzero",scope.clone(),"Closed subscription has a nonzero holding; retained without a fabricated current subscription",vec![]));
        }
        if quantity.units.is_negative() && role == PositionRole::Asset
            || !quantity.units.is_negative() && role == PositionRole::Liability
        {
            issues.push(issue(
                "unexpected_balance_sign",
                scope.clone(),
                "Balance sign conflicts with its economic role",
                vec![
                    WithheldFigure::CurrentValue,
                    WithheldFigure::CurrentEarnings,
                    WithheldFigure::OverallApy,
                ],
            ));
        }
        let price = if quantity.asset_id == request.reporting_asset {
            None
        } else {
            store.selected_price(p, &quantity.asset_id, &request.reporting_asset, cutoff)?
        };
        let v = if quantity.asset_id == request.reporting_asset {
            Some(ExactRatio::quantity(&quantity.units.abs(), quantity.scale))
        } else {
            price.as_ref().map(|q| {
                ExactRatio::quantity(&quantity.units.abs(), quantity.scale)
                    .mul(&ExactRatio::from_decimal(&q.price))
            })
        };
        let mut value_ev = ev.clone();
        if let Some(q) = &price {
            value_ev.extend(self.price_provenance(
                p,
                &quantity.asset_id,
                &request.reporting_asset,
                q,
            )?);
        }
        let explicit_value_block = observation.is_none()
            && issues
                .iter()
                .any(|i| i.withheld.contains(&WithheldFigure::CurrentValue));
        let block = explicit_value_block
            || (observation.is_none()
                && verification.as_ref().is_some_and(|r| {
                    r.outcome == VerificationOutcome::Mismatch
                        && (r.issues.is_empty()
                            || r.issues.iter().any(|i| {
                                i.position.is_none()
                                    || i.position.as_ref().is_some_and(|x| positions.contains(x))
                            }))
                }));
        if block {
            issues.push(issue(
                if explicit_value_block {"current_value_withheld"} else {"balance_mismatch_unobserved"},
                scope.clone(),
                "Current ledger quantity lacks usable independent evidence needed to resolve source coverage or mismatch",
                vec![
                    WithheldFigure::CurrentValue,
                    WithheldFigure::CurrentEarnings,
                    WithheldFigure::OverallApy,
                ],
            ));
        }
        let value = if block || issues.iter().any(|i| i.code == "unexpected_balance_sign") {
            unknown(
                issues
                    .iter()
                    .filter(|i| i.withheld.contains(&WithheldFigure::CurrentValue))
                    .cloned()
                    .collect(),
            )
        } else {
            match v {
                Some(v) => known(money(&request.reporting_asset, v), dedup(value_ev)),
                None => {
                    let i = issue(
                        "price_unknown",
                        scope.clone(),
                        "No exact dated quote in reporting currency; holding stays visible",
                        vec![
                            WithheldFigure::CurrentValue,
                            WithheldFigure::CurrentEarnings,
                            WithheldFigure::OverallApy,
                        ],
                    );
                    issues.push(i.clone());
                    unknown(vec![i])
                }
            }
        };
        let label = if let Some(s) = &subscription {
            match store
                .local_state(p, &LocalTarget::Subscription { id: s.id.clone() })?
                .value
            {
                Some(LocalValue::SubscriptionName(n)) => n,
                _ => s.label.clone(),
            }
        } else {
            crate::token_metadata::display_identity(store, &quantity.asset_id)?.0
        };
        Ok(CurrentHolding {
            inferred_allocations: vec![],
            id,
            account_id: account.clone(),
            display_provider: None,
            display_provider_name: None,
            provider_id: provider.clone(),
            position_ids: positions,
            role,
            quantity,
            ledger_quantities: ledger,
            subscription,
            label,
            value,
            price,
            rate: None,
            rate_scope: None,
            rate_evidence: vec![],
            annual_earnings: FigureDto::NotApplicable,
            annual_borrowing_cost: FigureDto::NotApplicable,
            summary: empty_summary(&request.reporting_asset),
            is_dust: false,
            evidence: ev,
            verification,
            verification_current,
            issues,
        })
    }
    fn price_provenance(
        &self,
        p: &lib_core::PortfolioId,
        asset: &AssetId,
        quote: &AssetId,
        q: &SelectedPrice,
    ) -> Result<Vec<EvidenceDto>> {
        if let Some(id) = &q.evidence_id {
            if id.starts_with("legacy-price:") {
                return Ok(vec![EvidenceDto::Provider {
                    source: "legacy-price-estimate".into(),
                    reference: "Stored CLI market quote (binary-float input; approximate estimate)"
                        .into(),
                    observed_at: q.as_of,
                    effective_from: Some(q.as_of),
                }]);
            }
            return Ok(self
                .store()
                .price_evidence(asset, quote)?
                .into_iter()
                .filter(|e| e.id == *id)
                .map(|e| provider_evidence(&e.evidence, Some(e.as_of)))
                .collect());
        }
        self.manual_provenance(
            p,
            &LocalTarget::Price {
                asset: asset.clone(),
                quote: quote.clone(),
            },
            q.updated_at,
            q.as_of,
        )
    }
    fn manual_provenance(
        &self,
        p: &lib_core::PortfolioId,
        target: &LocalTarget,
        updated_at: DateTime<Utc>,
        effective_from: DateTime<Utc>,
    ) -> Result<Vec<EvidenceDto>> {
        let action = self
            .store()
            .manual_actions(p)?
            .into_iter()
            .rev()
            .find(|a| a.target == *target);
        Ok(action
            .map(|a| EvidenceDto::Manual {
                action_id: a.id,
                updated_at,
                effective_from,
            })
            .into_iter()
            .collect())
    }
    fn rate_provenance(
        &self,
        p: &lib_core::PortfolioId,
        scope: &RateScope,
        r: &SelectedRate,
    ) -> Result<Vec<EvidenceDto>> {
        if let Some(id) = &r.evidence_id {
            if id == "native-wallet-zero" {
                return Ok(vec![EvidenceDto::Provider {
                    source: "plain-native-wallet".into(),
                    reference:
                        "Liquid native coin held in a wallet has no staking/lending product yield"
                            .into(),
                    observed_at: r.updated_at,
                    effective_from: Some(r.effective_at),
                }]);
            }
            return Ok(self
                .store()
                .rate_evidence(p, scope)?
                .into_iter()
                .filter(|e| e.id == *id)
                .map(|e| provider_evidence(&e.evidence, Some(e.effective_at)))
                .collect());
        }
        self.manual_provenance(
            p,
            &LocalTarget::Rate {
                scope: scope.clone(),
            },
            r.updated_at,
            r.effective_at,
        )
    }
    fn earnings_contributions(
        &self,
        request: &PortfolioRequest,
        hs: &[CurrentHolding],
    ) -> Result<Vec<EarningsContribution>> {
        let p = &request.context.portfolio_id;
        let cutoff = request.context.cutoff;
        let mut scopes = vec![];
        for h in hs {
            if h.role == PositionRole::Liability {
                for id in &h.position_ids {
                    scopes.push(RateScope::BorrowingPosition(id.clone()));
                }
                continue;
            }
            scopes.extend([
                RateScope::Provider(h.provider_id.clone()),
                RateScope::Asset(h.quantity.asset_id.clone()),
                RateScope::ProviderAsset {
                    provider: h.provider_id.clone(),
                    asset: h.quantity.asset_id.clone(),
                },
                RateScope::AccountAsset {
                    account: h.account_id.clone(),
                    asset: h.quantity.asset_id.clone(),
                },
            ]);
            if let Some(s) = &h.subscription {
                scopes.extend([
                    RateScope::Subscription(s.id.clone()),
                    RateScope::Product(s.product_id.clone()),
                ]);
            }
        }
        let mut candidates = vec![];
        for scope in dedup(scopes) {
            let selected = self.store().selected_rate(p, &scope, cutoff)?.or_else(|| {
                hs.iter()
                    .find(|h| {
                        h.role == PositionRole::Asset
                            && h.subscription.is_none()
                            && h.provider_id.0.eq_ignore_ascii_case("evm")
                            && h.quantity.asset_id.0.ends_with("/native")
                            && scope
                                == RateScope::AccountAsset {
                                    account: h.account_id.clone(),
                                    asset: h.quantity.asset_id.clone(),
                                }
                    })
                    .map(|_| SelectedRate {
                        rate: Some(AnnualRate {
                            fraction: "0".parse().expect("zero"),
                            basis: AnnualRateBasis::Apy,
                            compounds_per_year: None,
                            reward_asset: None,
                        }),
                        effective_at: cutoff,
                        updated_at: cutoff,
                        pinned: false,
                        evidence_id: Some("native-wallet-zero".into()),
                    })
            });
            if let Some(r) = selected {
                let ids: Vec<_> = hs
                    .iter()
                    .filter(|h| scope_matches(&scope, h))
                    .map(|h| h.id.clone())
                    .collect();
                if !ids.is_empty() {
                    candidates.push((scope, r, ids));
                }
            }
        }
        // Explicit aggregate overrides cover their whole scope. Source aggregates
        // cannot be inherited across an uncovered remainder when finer data exists.
        let detailed = candidates.clone();
        candidates.retain(|(s, r, ids)| {
            r.evidence_id.is_none()
                || scope_rank(s) >= 5
                || !detailed.iter().any(|(ss, _, i)| {
                    scope_rank(ss) > scope_rank(s) && i.iter().any(|id| ids.contains(id))
                })
        });
        candidates.sort_by(|(sa, a, _), (sb, b, _)| {
            b.pinned
                .cmp(&a.pinned)
                .then(b.evidence_id.is_none().cmp(&a.evidence_id.is_none()))
                .then_with(|| {
                    if a.evidence_id.is_none() && b.evidence_id.is_none() {
                        b.updated_at.cmp(&a.updated_at)
                    } else {
                        scope_rank(sb).cmp(&scope_rank(sa))
                    }
                })
                .then(scope_rank(sb).cmp(&scope_rank(sa)))
                .then_with(|| format!("{sa:?}").cmp(&format!("{sb:?}")))
        });
        let mut covered = BTreeSet::new();
        let mut out = vec![];
        for (scope, selected, ids) in candidates {
            // Never apply a conflicting aggregate to only its remainder.
            if ids.iter().any(|id| covered.contains(id)) {
                continue;
            }
            covered.extend(ids.iter().cloned());
            let selected_hs: Vec<_> = hs.iter().filter(|h| ids.contains(&h.id)).collect();
            let principal = sum(
                &request.reporting_asset,
                selected_hs.iter().map(|h| h.value.clone()).collect(),
            );
            let ev = self.rate_provenance(p, &scope, &selected)?;
            let mut issues = vec![];
            if self.store().override_requires_review(
                p,
                &LocalTarget::Rate {
                    scope: scope.clone(),
                },
            )? {
                issues.push(issue("aggregate_override_review",rate_scope_entity(&scope),"New source or finer allocation evidence needs review; existing override and pin remain active",vec![]));
            }
            let f = match selected.rate.as_ref().and_then(rate_factor) {
                Some(f) => {
                    if selected_hs.iter().any(|h| {
                        h.issues.iter().any(|i| {
                            i.withheld.contains(&WithheldFigure::CurrentEarnings)
                                && i.code != "price_unknown"
                        })
                    }) {
                        unknown(dedup(
                            selected_hs
                                .iter()
                                .flat_map(|h| {
                                    h.issues
                                        .iter()
                                        .filter(|i| {
                                            i.withheld.contains(&WithheldFigure::CurrentEarnings)
                                        })
                                        .cloned()
                                })
                                .collect(),
                        ))
                    } else {
                        match amount(&principal) {
                            Some(v) => {
                                let value = money(&request.reporting_asset, v.mul(&f));
                                let evidence = dedup([evidence(&principal), ev.clone()].concat());
                                if complete(&principal) {
                                    known(value, evidence)
                                } else {
                                    FigureDto::Partial {
                                        value,
                                        evidence,
                                        issues: figure_issues(&principal),
                                    }
                                }
                            }
                            None => unknown(figure_issues(&principal)),
                        }
                    }
                }
                None => unknown(vec![issue(
                    if selected.rate.is_none() {
                        "rate_unknown"
                    } else {
                        "compounding_unsupported"
                    },
                    rate_scope_entity(&scope),
                    "Applicable rate is unknown or uses an unsupported compounding convention",
                    vec![WithheldFigure::CurrentEarnings, WithheldFigure::OverallApy],
                )]),
            };
            issues.extend(figure_issues(&f));
            let issues = dedup(issues);
            let debt = matches!(scope, RateScope::BorrowingPosition(_));
            out.push(EarningsContribution {
                holding_ids: ids,
                scope,
                selected_rate: selected,
                gross_earnings: if debt {
                    FigureDto::NotApplicable
                } else {
                    f.clone()
                },
                borrowing_costs: if debt { f } else { FigureDto::NotApplicable },
                evidence: ev,
                issues,
            });
        }
        for h in hs.iter().filter(|h| {
            h.role == PositionRole::Asset
                && !h.inferred_allocations.is_empty()
                && !covered.contains(&h.id)
        }) {
            let mut figures = vec![];
            for part in &h.inferred_allocations {
                let value = match (&part.expected_apy, amount(&h.value)) {
                    (Some(rate), Some(principal)) if complete(&h.value) => {
                        let share = ExactRatio::quantity(&part.quantity.units, part.quantity.scale)
                            .div(&ExactRatio::quantity(&h.quantity.units, h.quantity.scale));
                        share
                            .map(|s| {
                                known(
                                    money(&request.reporting_asset, principal.mul(&s).mul(rate)),
                                    [evidence(&h.value), part.expected_evidence.clone()].concat(),
                                )
                            })
                            .unwrap_or_else(|| unknown(vec![]))
                    }
                    _ => unknown(vec![issue(
                        "rate_unknown",
                        hscope(h),
                        "Unclassified or unsupported term yield remains unknown",
                        vec![WithheldFigure::CurrentEarnings, WithheldFigure::OverallApy],
                    )]),
                };
                figures.push(value);
            }
            let earnings = sum(&request.reporting_asset, figures);
            // Partition earnings are one contribution for the original holding;
            // child values never enter ownership or portfolio sums again.
            out.push(EarningsContribution {holding_ids:vec![h.id.clone()],scope:RateScope::AccountAsset {account:h.account_id.clone(),asset:h.quantity.asset_id.clone()},selected_rate:SelectedRate {rate:None,effective_at:cutoff,updated_at:cutoff,pinned:false,evidence_id:Some("nexo-qualified-partition".into())},gross_earnings:earnings.clone(),borrowing_costs:FigureDto::NotApplicable,evidence:vec![EvidenceDto::Provider {source:"nexo-expected-partition".into(),reference:"User-supplied applicable fixed product rates on exact inferred principal; residual yield remains unknown".into(),observed_at:cutoff,effective_from:Some(cutoff)}],issues:figure_issues(&earnings)});
        }
        Ok(out)
    }
    pub fn overview(&self, request: PortfolioRequest) -> Result<Summary> {
        Ok(self.portfolio_projection(request)?.overview)
    }
    pub fn navigation(
        &self,
        request: PortfolioRequest,
        by_asset: bool,
        search: &str,
    ) -> Result<Vec<NavigationEntry>> {
        let projection = self.portfolio_projection(request)?;
        let mut nav = if by_asset {
            projection.asset_navigation()
        } else {
            projection.providers
        };
        let search = search.to_lowercase();
        nav.retain(|n| {
            !n.metadata_unresolved
                && (n.name.to_lowercase().contains(&search)
                    || n.ticker
                        .as_ref()
                        .is_some_and(|t| t.to_lowercase().contains(&search)))
        });
        if by_asset
            && projection.dust.name.to_lowercase().contains(&search)
            && projection.holdings.iter().any(|h| h.is_dust)
        {
            nav.push(projection.dust)
        }
        sort_nav(&mut nav);
        Ok(nav)
    }
    pub fn allocation_tree(
        &self,
        request: PortfolioRequest,
        selection: Selection,
    ) -> Result<AllocationTree> {
        self.portfolio_projection(request)?
            .allocation_tree(selection)
    }
    pub fn details(&self, request: PortfolioRequest, target: AllocationTarget) -> Result<Details> {
        self.portfolio_projection(request)?.details(target)
    }
    pub fn issues(&self, request: PortfolioRequest, selection: Selection) -> Result<Vec<IssueDto>> {
        let p = self.portfolio_projection(request)?;
        Ok(dedup(
            p.holdings
                .iter()
                .filter(|h| selected(&selection, h))
                .flat_map(|h| h.issues.clone())
                .collect(),
        ))
    }
}
fn provider_evidence(e: &SourceEvidence, effective_from: Option<DateTime<Utc>>) -> EvidenceDto {
    EvidenceDto::Provider {
        source: e.source.clone(),
        reference: e.reference.clone(),
        observed_at: e.observed_at,
        effective_from,
    }
}
fn scope_rank(s: &RateScope) -> u8 {
    match s {
        RateScope::Provider(_) => 1,
        RateScope::Asset(_) => 2,
        RateScope::ProviderAsset { .. } => 3,
        RateScope::Product(_) | RateScope::AccountAsset { .. } => 4,
        RateScope::Subscription(_) => 5,
        RateScope::BorrowingPosition(_) => 6,
    }
}
fn scope_matches(s: &RateScope, h: &CurrentHolding) -> bool {
    if h.role == PositionRole::Liability {
        return matches!(s,RateScope::BorrowingPosition(id) if h.position_ids.len()==1 && h.position_ids.contains(id));
    }
    match s {
        RateScope::Provider(p) => h.provider_id == *p,
        RateScope::Asset(a) => h.quantity.asset_id == *a,
        RateScope::ProviderAsset { provider, asset } => {
            h.provider_id == *provider && h.quantity.asset_id == *asset
        }
        RateScope::AccountAsset { account, asset } => {
            h.account_id == *account && h.quantity.asset_id == *asset
        }
        RateScope::Product(id) => h.subscription.as_ref().is_some_and(|s| s.product_id == *id),
        RateScope::Subscription(id) => h.subscription.as_ref().is_some_and(|s| s.id == *id),
        RateScope::BorrowingPosition(_) => false,
    }
}
fn selected(s: &Selection, h: &CurrentHolding) -> bool {
    match s {
        Selection::Overview => true,
        Selection::Asset(id) => h.quantity.asset_id == *id,
        Selection::Exposure(id) => crate::asset_groups::representation(&h.quantity.asset_id.0)
            .is_some_and(|r| r.underlying == id.0),
        Selection::Provider(id) => h.display_provider.as_ref().unwrap_or(&h.provider_id) == id,
        Selection::Dust => h.is_dust,
    }
}
fn nav(
    r: &PortfolioRequest,
    hs: &[CurrentHolding],
    cs: &[EarningsContribution],
    selection: Selection,
    name: String,
    ticker: Option<String>,
) -> NavigationEntry {
    let group: Vec<_> = hs.iter().filter(|h| selected(&selection, h)).collect();
    let mut quantities = vec![];
    if matches!(&selection, Selection::Asset(_) | Selection::Exposure(_)) {
        let assets: BTreeSet<_> = group
            .iter()
            .map(|h| h.quantity.asset_id.0.clone())
            .collect();
        for asset in assets {
            for role in [PositionRole::Asset, PositionRole::Liability] {
                let same: Vec<_> = group
                    .iter()
                    .filter(|h| h.role == role && h.quantity.asset_id.0 == asset)
                    .collect();
                if let Some(h) = same.first() {
                    let units = same.iter().fold(Units::from(0), |mut sum, h| {
                        sum += &h.quantity.units;
                        sum
                    });
                    quantities.push((
                        role,
                        QuantityDto {
                            asset_id: AssetId::str(&asset),
                            units,
                            scale: h.quantity.scale,
                        },
                    ));
                }
            }
        }
    }
    let estimated_current_value =
        matches!(&selection, Selection::Asset(_) | Selection::Exposure(_)).then(|| {
            sum(
                &r.reporting_asset,
                group
                    .iter()
                    .map(|h| {
                        // A negative savings balance is a mapping/history problem,
                        // not evidence of borrowing. Only liability roles create debt.
                        if h.issues.iter().any(|i| i.code == "unexpected_balance_sign") {
                            return unknown(h.issues.clone());
                        }
                        let price = if h.quantity.asset_id == r.reporting_asset {
                            Some(ExactRatio::integer(1))
                        } else {
                            h.price.as_ref().map(|p| ExactRatio::from_decimal(&p.price))
                        };
                        let Some(price) = price else {
                            return unknown(h.issues.clone());
                        };
                        let units = if h.role == PositionRole::Liability {
                            lib_core::Units::from(0) - h.quantity.units.abs()
                        } else {
                            h.quantity.units.clone()
                        };
                        let value = money(
                            &r.reporting_asset,
                            ExactRatio::quantity(&units, h.quantity.scale).mul(&price),
                        );
                        if complete(&h.value) {
                            known(value, evidence(&h.value))
                        } else {
                            FigureDto::Partial {
                                value,
                                evidence: evidence(&h.value),
                                issues: h.issues.clone(),
                            }
                        }
                    })
                    .collect(),
            )
        });
    NavigationEntry {
        direct_quantities: None,
        alternatives: vec![],
        estimated_current_value,
        token_metadata: None,
        metadata_unresolved: false,
        selection,
        name,
        ticker,
        quantities,
        summary: summary(r, &group, cs),
        issues: dedup(group.iter().flat_map(|h| h.issues.clone()).collect()),
    }
}
fn sort_nav(ns: &mut [NavigationEntry]) {
    ns.sort_by(|a, b| {
        amount(
            b.estimated_current_value
                .as_ref()
                .unwrap_or(&b.summary.current_value),
        )
        .cmp(&amount(
            a.estimated_current_value
                .as_ref()
                .unwrap_or(&a.summary.current_value),
        ))
        .then(a.name.cmp(&b.name))
        .then_with(|| format!("{:?}", a.selection).cmp(&format!("{:?}", b.selection)))
    });
}
fn node_id(t: &AllocationTarget) -> String {
    serde_json::to_string(t).expect("serializable target")
}
/// Stored provider display choices win; known brands have readable defaults.
fn provider_display_name(
    id: &ProviderId,
    display: Option<&lib_core::ui_metadata::ProviderDisplay>,
) -> String {
    display.and_then(|d| d.name.clone()).unwrap_or_else(|| {
        match id.0.to_ascii_lowercase().as_str() {
            "nexo" => "Nexo".into(),
            "binance" => "Binance".into(),
            "kraken" => "Kraken".into(),
            "evm" => "EVM wallets".into(),
            _ => id.0.clone(),
        }
    })
}

fn build_tree(
    request: &PortfolioRequest,
    metadata: &UiMetadataDto,
    hs: &[CurrentHolding],
    cs: &[EarningsContribution],
    selection: Selection,
) -> Result<AllocationTree> {
    let group: Vec<_> = hs.iter().filter(|h| selected(&selection, h)).collect();
    let mut nodes: BTreeMap<String, AllocationNode> = BTreeMap::new();
    let mut add = |target: AllocationTarget, parent: Option<AllocationTarget>, name: String| {
        let id = node_id(&target);
        nodes.entry(id.clone()).or_insert_with(|| AllocationNode {
            inferred_allocation: None,
            id,
            target,
            parent: parent.as_ref().map(node_id),
            children: vec![],
            name,
            holding_ids: vec![],
            summary: empty_summary(&request.reporting_asset),
            issues: vec![],
        });
    };
    for h in &group {
        let pt = AllocationTarget::Provider(
            h.display_provider
                .as_ref()
                .unwrap_or(&h.provider_id)
                .clone(),
        );
        let pname = provider_display_name(
            &h.provider_id,
            metadata.providers.iter().find(|p| p.id == h.provider_id),
        );
        add(
            pt.clone(),
            None,
            h.display_provider_name.clone().unwrap_or(pname),
        );
        let mut account = Some(h.account_id.clone());
        while let Some(id) = account {
            let a = metadata
                .accounts
                .iter()
                .find(|a| a.account.id == id)
                .ok_or_else(|| anyhow::anyhow!("holding account metadata missing"))?;
            let custom = metadata
                .local_state
                .iter()
                .find_map(|(t, r)| match (t, &r.value) {
                    (LocalTarget::Account { id: target }, Some(LocalValue::Account(c)))
                        if *target == id =>
                    {
                        c.local_label.clone().or_else(|| {
                            c.wallet_software
                                .as_ref()
                                .map(|software| format!("{software} · {}", a.account.label))
                        })
                    }
                    _ => None,
                });
            let parent = a
                .parent
                .clone()
                .map(AllocationTarget::Account)
                .unwrap_or(pt.clone());
            add(
                AllocationTarget::Account(id),
                Some(parent),
                custom.unwrap_or(a.account.label.clone()),
            );
            account = a.parent.clone();
        }
        let mut parent = AllocationTarget::Account(h.account_id.clone());
        if let Some(s) = &h.subscription {
            let product = AllocationTarget::Product {
                account: h.account_id.clone(),
                product: s.product_id.clone(),
            };
            let name = metadata
                .products
                .iter()
                .find(|p| p.id == s.product_id)
                .map(|p| p.label.clone())
                .unwrap_or(s.product_id.0.clone());
            add(product.clone(), Some(parent), name);
            let sub = AllocationTarget::Subscription(s.id.clone());
            add(sub.clone(), Some(product), h.label.clone());
            parent = sub;
        }
        add(
            AllocationTarget::Holding(h.id.clone()),
            Some(parent),
            h.label.clone(),
        );
    }
    // Unique leaf ownership; ancestor IDs are sets so direct + child values count once.
    for h in &group {
        let mut id = Some(node_id(&AllocationTarget::Holding(h.id.clone())));
        while let Some(key) = id {
            let node = nodes
                .get_mut(&key)
                .ok_or_else(|| anyhow::anyhow!("allocation parent missing"))?;
            if !node.holding_ids.contains(&h.id) {
                node.holding_ids.push(h.id.clone());
            }
            id = node.parent.clone();
        }
    }
    let links: Vec<_> = nodes
        .values()
        .filter_map(|n| n.parent.as_ref().map(|p| (p.clone(), n.id.clone())))
        .collect();
    for (parent, id) in links {
        nodes
            .get_mut(&parent)
            .expect("validated parent")
            .children
            .push(id);
    }
    for node in nodes.values_mut() {
        let gh: Vec<_> = group
            .iter()
            .copied()
            .filter(|h| node.holding_ids.contains(&h.id))
            .collect();
        node.summary = summary(request, &gh, cs);
        node.issues = dedup(gh.iter().flat_map(|h| h.issues.clone()).collect());
    }
    let order: BTreeMap<_, _> = nodes
        .iter()
        .map(|(id, n)| (id.clone(), amount(&n.summary.current_value)))
        .collect();
    let sort = |a: &String, b: &String| order[b].cmp(&order[a]).then(a.cmp(b));
    for node in nodes.values_mut() {
        node.children.sort_by(sort);
    }
    let mut roots: Vec<_> = nodes
        .values()
        .filter(|n| n.parent.is_none())
        .map(|n| n.id.clone())
        .collect();
    roots.sort_by(sort);
    // Informational partitions sit below their aggregate holding. Their values
    // are never rolled up again into holdings or portfolio earnings.
    for h in &group {
        let parent = node_id(&AllocationTarget::Holding(h.id.clone()));
        for part in &h.inferred_allocations {
            let target = AllocationTarget::Holding(part.id.clone());
            let id = node_id(&target);
            let mut child_summary = empty_summary(&request.reporting_asset);
            let quantity = ExactRatio::quantity(&part.quantity.units, part.quantity.scale);
            let price = if part.quantity.asset_id == request.reporting_asset {
                Some(ExactRatio::integer(1))
            } else {
                h.price.as_ref().map(|p| ExactRatio::from_decimal(&p.price))
            };
            let scope = EntityScope::Account {
                account_id: h.account_id.clone(),
            };
            let warning = issue(
                "nexo_term_inferred",
                scope.clone(),
                match part.kind.as_str() {
                    "unclassified_residual" => {
                        "Flexible savings: user confirmed the non-fixed balance earns the flexible base rate; quantity is the remainder at the export cutoff"
                    }
                    "unresolved_fixed" => {
                        "Several same-asset terms share the exact principal; lock/unlock transactions cannot identify the surviving individual terms or their start dates"
                    }
                    _ => {
                        "Lock principal and start date are identified. Expected rate/maturity use account annotations; the CSV does not confirm the actual due date, renewal or current live status"
                    }
                },
                vec![],
            );
            let value = if complete(&h.value) {
                price
                    .map(|p| {
                        known(
                            money(&request.reporting_asset, quantity.mul(&p)),
                            evidence(&h.value),
                        )
                    })
                    .unwrap_or_else(|| unknown(h.issues.clone()))
            } else {
                unknown(h.issues.clone())
            };
            child_summary.gross_assets = value.clone();
            child_summary.current_value = value;
            let missing = issue(
                "rate_unknown",
                scope,
                "No supported contractual rate for this inferred allocation",
                vec![WithheldFigure::CurrentEarnings, WithheldFigure::OverallApy],
            );
            child_summary.gross_annual_earnings = unknown(vec![missing.clone()]);
            child_summary.net_annual_earnings = unknown(vec![missing.clone()]);
            child_summary.current_apy = unknown(vec![missing]);
            child_summary.estimated_current_apy = child_summary.current_apy.clone();
            if let Some(rate) = &part.expected_apy {
                child_summary.current_apy = known(rate.clone(), part.expected_evidence.clone());
                child_summary.estimated_current_apy = child_summary.current_apy.clone();
                if let Some(principal) = amount(&child_summary.current_value) {
                    child_summary.gross_annual_earnings = known(
                        money(&request.reporting_asset, principal.mul(rate)),
                        [
                            evidence(&child_summary.current_value),
                            part.expected_evidence.clone(),
                        ]
                        .concat(),
                    );
                    child_summary.net_annual_earnings = child_summary.gross_annual_earnings.clone();
                }
            }
            let name = match part.kind.as_str() {
                "fixed" => format!(
                    "Fixed term · {}",
                    part.opened_at.unwrap().format("%Y-%m-%d")
                ),
                "unresolved_fixed" => format!("{} fixed terms · identity unresolved", part.count),
                _ => "Flexible savings".into(),
            };
            nodes
                .get_mut(&parent)
                .expect("holding parent")
                .children
                .push(id.clone());
            nodes.insert(
                id.clone(),
                AllocationNode {
                    id,
                    target,
                    parent: Some(parent.clone()),
                    children: vec![],
                    name,
                    holding_ids: vec![],
                    summary: child_summary,
                    issues: vec![warning],
                    inferred_allocation: Some(part.clone()),
                },
            );
        }
    }
    // Display groups summarize child partitions without claiming ledger ownership.
    for h in &group {
        let parent = node_id(&AllocationTarget::Holding(h.id.clone()));
        if !h.inferred_allocations.is_empty() {
            if let Some(policy) = crate::provider_grouping::policy(&h.provider_id.0) {
                let mut sections: BTreeMap<String, (crate::provider_grouping::Group, Vec<String>)> =
                    BTreeMap::new();
                for child in &nodes[&parent].children {
                    if let Some(section) = nodes[child]
                        .inferred_allocation
                        .as_ref()
                        .and_then(|part| policy.partition_group(&part.kind))
                    {
                        sections
                            .entry(section.key.clone())
                            .or_insert_with(|| (section, vec![]))
                            .1
                            .push(child.clone());
                    }
                }
                for (_, (section, fixed)) in sections {
                    let target = AllocationTarget::Product {
                        account: h.account_id.clone(),
                        product: lib_core::ProductId::from(format!(
                            "layout:{}:{}:{}",
                            h.provider_id.0, section.key, h.id
                        )),
                    };
                    let id = node_id(&target);
                    let summaries: Vec<_> =
                        fixed.iter().map(|id| nodes[id].summary.clone()).collect();
                    let mut total = empty_summary(&request.reporting_asset);
                    total.current_value = sum(
                        &request.reporting_asset,
                        summaries.iter().map(|s| s.current_value.clone()).collect(),
                    );
                    total.gross_assets = total.current_value.clone();
                    total.gross_annual_earnings = sum(
                        &request.reporting_asset,
                        summaries
                            .iter()
                            .map(|s| s.gross_annual_earnings.clone())
                            .collect(),
                    );
                    total.net_annual_earnings = total.gross_annual_earnings.clone();
                    total.current_apy = match (
                        amount(&total.gross_annual_earnings),
                        amount(&total.current_value),
                    ) {
                        (Some(e), Some(v)) if !v.is_zero() => {
                            let rate = e.div(&v).expect("nonzero partition value");
                            if complete(&total.gross_annual_earnings)
                                && complete(&total.current_value)
                            {
                                known(rate, vec![])
                            } else {
                                FigureDto::Partial {
                                    value: rate,
                                    evidence: vec![],
                                    issues: figure_issues(&total.gross_annual_earnings),
                                }
                            }
                        }
                        _ => unknown(
                            fixed
                                .iter()
                                .flat_map(|id| nodes[id].issues.clone())
                                .collect(),
                        ),
                    };
                    total.estimated_current_apy = total.current_apy.clone();
                    for child in &fixed {
                        nodes.get_mut(child).unwrap().parent = Some(id.clone());
                    }
                    nodes
                        .get_mut(&parent)
                        .unwrap()
                        .children
                        .retain(|child| !fixed.contains(child));
                    nodes.get_mut(&parent).unwrap().children.push(id.clone());
                    nodes.insert(
                        id.clone(),
                        AllocationNode {
                            id,
                            target,
                            parent: Some(parent.clone()),
                            children: fixed,
                            name: section.label,
                            holding_ids: vec![],
                            summary: total,
                            issues: vec![],
                            inferred_allocation: None,
                        },
                    );
                }
                if matches!(selection, Selection::Asset(_) | Selection::Exposure(_)) {
                    let original = nodes.remove(&parent).unwrap();
                    if let Some(ancestor) = original.parent {
                        for child in &original.children {
                            nodes.get_mut(child).unwrap().parent = Some(ancestor.clone());
                        }
                        let children = &mut nodes.get_mut(&ancestor).unwrap().children;
                        children.retain(|id| id != &parent);
                        children.extend(original.children);
                    }
                }
            }
        }
    }
    // Chain grouping comes from the EVM policy, with software-scoped display roots.
    let mut chain_buckets: BTreeMap<(String, String), Vec<(String, String)>> = BTreeMap::new();
    for node in nodes.values() {
        let AllocationTarget::Account(account) = &node.target else {
            continue;
        };
        let Some(parent) = node.parent.as_ref() else {
            continue;
        };
        if !matches!(&nodes[parent].target,AllocationTarget::Provider(id) if id.0.starts_with("wallet-software:"))
        {
            continue;
        }
        if let Some(chain) = metadata
            .accounts
            .iter()
            .find(|a| a.account.id == *account)
            .and_then(|a| a.account.source_reference.as_ref())
            .and_then(|r| crate::provider_grouping::policy("evm")?.chain_group(r))
        {
            chain_buckets
                .entry((parent.clone(), chain.key))
                .or_default()
                .push((node.id.clone(), chain.label));
        }
    }
    for ((parent, chain), members) in chain_buckets {
        let display = match &nodes[&parent].target {
            AllocationTarget::Provider(id) => id.0.clone(),
            _ => unreachable!(),
        };
        let owner=metadata.accounts.iter().filter(|a|a.account.provider_id.0=="evm")
            .filter(|a|a.account.source_reference.as_ref().and_then(|r|crate::provider_grouping::policy("evm")?.chain_group(r)).is_some_and(|g|g.key==chain))
            .filter(|a|metadata.local_state.iter().any(|(t,r)|matches!((t,&r.value),(LocalTarget::Account{id},Some(LocalValue::Account(c))) if id==&a.account.id && c.wallet_software.as_ref().is_some_and(|s|format!("wallet-software:{}",s.trim().to_lowercase())==display))))
            .min_by_key(|a|&a.account.id.0).expect("chain owner").account.id.clone();
        let target = AllocationTarget::Product {
            account: owner,
            product: lib_core::ProductId::from(format!("layout:{display}:chain:{chain}")),
        };
        let id = node_id(&target);
        let mut children = vec![];
        let mut ids = vec![];
        let label = members[0].1.clone();
        for (account_node, _) in &members {
            let old = nodes.remove(account_node).unwrap();
            ids.extend(old.holding_ids);
            // Source account identity remains in holding details and metadata.
            for child in &old.children {
                nodes.get_mut(child).unwrap().parent = Some(id.clone());
            }
            children.extend(old.children);
            nodes
                .get_mut(&parent)
                .unwrap()
                .children
                .retain(|c| c != account_node);
        }
        ids.sort();
        ids.dedup();
        let grouped: Vec<_> = group
            .iter()
            .copied()
            .filter(|h| ids.contains(&h.id))
            .collect();
        nodes.get_mut(&parent).unwrap().children.push(id.clone());
        nodes.insert(
            id.clone(),
            AllocationNode {
                id,
                target,
                parent: Some(parent),
                children,
                name: label,
                holding_ids: ids,
                summary: summary(request, &grouped, cs),
                issues: dedup(grouped.iter().flat_map(|h| h.issues.clone()).collect()),
                inferred_allocation: None,
            },
        );
    }
    Ok(AllocationTree {
        selection,
        roots,
        nodes: nodes.into_values().collect(),
    })
}
impl PortfolioProjection {
    /// Group explicitly reviewed representations for navigation; amounts/prices remain exact-asset scoped.
    pub fn asset_navigation(&self) -> Vec<NavigationEntry> {
        let mut groups: BTreeMap<String, Vec<&NavigationEntry>> = BTreeMap::new();
        let mut out = vec![];
        for entry in &self.assets {
            let Selection::Asset(id) = &entry.selection else {
                continue;
            };
            if let Some(r) = crate::asset_groups::representation(&id.0) {
                groups.entry(r.underlying.into()).or_default().push(entry);
            } else if !entry.metadata_unresolved
                && self
                    .holdings
                    .iter()
                    .any(|h| h.quantity.asset_id == *id && !h.is_dust)
            {
                out.push(entry.clone());
            }
        }
        for (id, entries) in groups {
            let selection = Selection::Exposure(AssetId::str(&id));
            if !self
                .holdings
                .iter()
                .any(|h| selected(&selection, h) && !h.is_dust)
            {
                continue;
            }
            if entries.len() == 1 && entries[0].selection == Selection::Asset(AssetId::str(&id)) {
                out.push(entries[0].clone());
            } else {
                out.push(nav(
                    &self.request,
                    &self.holdings,
                    &self.contributions,
                    selection,
                    crate::asset_groups::group_label(&id).into(),
                    Some(id),
                ));
            }
        }
        for entry in &mut out {
            let underlying = match &entry.selection {
                Selection::Exposure(id) => id.0.clone(),
                Selection::Asset(id) => match crate::asset_groups::representation(&id.0) {
                    Some(r) => r.underlying.to_string(),
                    None => continue,
                },
                _ => continue,
            };
            let group: Vec<_> = self
                .holdings
                .iter()
                .filter(|h| selected(&entry.selection, h))
                .collect();
            let direct: Vec<_> = group
                .iter()
                .copied()
                .filter(|h| {
                    h.role == PositionRole::Asset
                        && crate::asset_groups::is_direct(&h.provider_id.0, &h.quantity.asset_id.0)
                })
                .collect();
            let scale = direct.iter().map(|h| h.quantity.scale).max().unwrap_or(0);
            let units = direct.iter().fold(Units::from(0), |mut sum, h| {
                sum +=
                    Units::from_decimal(&h.quantity.units.decimal_string(h.quantity.scale), scale)
                        .expect("exact widening of a validated quantity");
                sum
            });
            entry.direct_quantities = Some(if direct.is_empty() {
                vec![]
            } else {
                vec![QuantityDto {
                    asset_id: AssetId::str(&underlying),
                    units,
                    scale,
                }]
            });
            let mut alternatives: BTreeMap<(String, String, bool), Vec<CurrentHolding>> =
                BTreeMap::new();
            for h in group {
                if h.role != PositionRole::Asset
                    || !crate::asset_groups::is_direct(&h.provider_id.0, &h.quantity.asset_id.0)
                {
                    alternatives
                        .entry((
                            h.provider_id.0.clone(),
                            h.quantity.asset_id.0.clone(),
                            h.role == PositionRole::Liability,
                        ))
                        .or_default()
                        .push(h.clone());
                }
            }
            for ((provider, asset, debt), holdings) in alternatives {
                let h = &holdings[0];
                let r = crate::asset_groups::representation(&asset).expect("reviewed group member");
                let provider_id = ProviderId::from(provider.as_str());
                let representation_name =
                    if provider_id.0.eq_ignore_ascii_case("nexo") && asset == "USD" {
                        "Tokenized USD"
                    } else {
                        r.name
                    };
                let name = format!(
                    "{} · {}",
                    representation_name,
                    provider_display_name(
                        &provider_id,
                        self.metadata.providers.iter().find(|d| d.id == provider_id)
                    )
                );
                let variant = nav(
                    &self.request,
                    &holdings,
                    &self.contributions,
                    Selection::Asset(AssetId::str(&asset)),
                    name.clone(),
                    Some(r.ticker.into()),
                );
                let units = holdings.iter().fold(Units::from(0), |mut sum, h| {
                    sum += &h.quantity.units;
                    sum
                });
                entry.alternatives.push(AlternativeRepresentation {
                    name,
                    ticker: r.ticker.into(),
                    provider: provider_id,
                    role: if debt {
                        PositionRole::Liability
                    } else {
                        PositionRole::Asset
                    },
                    quantity: QuantityDto {
                        asset_id: AssetId::str(&asset),
                        units,
                        scale: h.quantity.scale,
                    },
                    value: variant
                        .estimated_current_value
                        .unwrap_or(variant.summary.current_value),
                });
            }
        }
        sort_nav(&mut out);
        out
    }
    pub fn allocation_tree(&self, selection: Selection) -> Result<AllocationTree> {
        build_tree(
            &self.request,
            &self.metadata,
            &self.holdings,
            &self.contributions,
            selection,
        )
    }
    pub fn details(&self, target: AllocationTarget) -> Result<Details> {
        let node = self
            .allocation
            .nodes
            .iter()
            .find(|n| n.target == target)
            .ok_or_else(|| anyhow::anyhow!("target is not a current allocation"))?
            .clone();
        let holdings: Vec<_> = self
            .holdings
            .iter()
            .filter(|h| node.holding_ids.contains(&h.id))
            .cloned()
            .collect();
        let original_account = match &target {
            AllocationTarget::Account(id) => self
                .metadata
                .accounts
                .iter()
                .find(|a| a.account.id == *id)
                .map(|a| a.account.clone()),
            _ => holdings
                .first()
                .filter(|h| holdings.iter().all(|x| x.account_id == h.account_id))
                .and_then(|h| {
                    self.metadata
                        .accounts
                        .iter()
                        .find(|a| a.account.id == h.account_id)
                        .map(|a| a.account.clone())
                }),
        };
        let subscription = match &target {
            AllocationTarget::Subscription(id) => self
                .metadata
                .subscriptions
                .iter()
                .find(|s| s.id == *id)
                .cloned(),
            _ => None,
        };
        let product = match &target {
            AllocationTarget::Product { product, .. } => self
                .metadata
                .products
                .iter()
                .find(|p| p.id == *product)
                .cloned(),
            _ => subscription.as_ref().and_then(|s| {
                self.metadata
                    .products
                    .iter()
                    .find(|p| p.id == s.product_id)
                    .cloned()
            }),
        };
        Ok(Details {
            node,
            holdings,
            original_account,
            product,
            subscription,
        })
    }
}

fn rate_scope_entity(scope: &RateScope) -> EntityScope {
    match scope {
        RateScope::Subscription(id) => EntityScope::Subscription {
            subscription_id: id.clone(),
        },
        RateScope::Product(id) => EntityScope::Product {
            product_id: id.clone(),
        },
        RateScope::Provider(id) | RateScope::ProviderAsset { provider: id, .. } => {
            EntityScope::Provider {
                provider_id: id.clone(),
            }
        }
        RateScope::Asset(id) => EntityScope::Asset {
            asset_id: id.clone(),
        },
        RateScope::AccountAsset { account, .. } => EntityScope::Account {
            account_id: account.clone(),
        },
        RateScope::BorrowingPosition(id) => EntityScope::Position {
            position_id: id.clone(),
        },
    }
}
