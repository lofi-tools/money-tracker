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
    pub historical_effective_apy: FigureDto<ExactRatio>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CurrentHolding {
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
    Provider(ProviderId),
    Dust,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NavigationEntry {
    pub selection: Selection,
    pub name: String,
    pub ticker: Option<String>,
    /// Only same-asset quantities, partitioned by economic role.
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
        unknown(dedup(source_apy_issues))
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
    Summary {
        calculation_basis: SUMMARY_BASIS.into(),
        current_value: subtract(quote, gross.clone(), liabilities.clone()),
        gross_assets: gross,
        liabilities,
        gross_annual_earnings: earning,
        annual_borrowing_costs: costs,
        net_annual_earnings: net,
        current_apy: apy,
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
            let dust = match state.value {
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
            provider_ids.insert(h.provider_id.0.clone());
        }
        let mut assets = vec![];
        for id in asset_ids {
            let display = store.asset_display(&AssetId::str(&id))?;
            assets.push(nav(
                &request,
                &holdings,
                &contributions,
                Selection::Asset(AssetId::str(&id)),
                display
                    .as_ref()
                    .and_then(|d| {
                        d.name
                            .clone()
                            .or(d.ticker.clone())
                            .or(d.unresolved_label.clone())
                    })
                    .unwrap_or(id),
                display.and_then(|d| d.ticker),
            ));
        }
        let mut providers = vec![];
        for id in provider_ids {
            let name = store
                .provider_display(&id.as_str().into())?
                .and_then(|d| d.name)
                .unwrap_or(id.clone());
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
            "Dust".into(),
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
    #[allow(clippy::too_many_arguments)]
    fn make_holding(
        &self,
        request: &PortfolioRequest,
        metadata: &UiMetadataDto,
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
            metadata
                .assets
                .iter()
                .find(|d| d.id == quantity.asset_id)
                .and_then(|d| {
                    d.name
                        .clone()
                        .or(d.ticker.clone())
                        .or(d.unresolved_label.clone())
                })
                .unwrap_or(quantity.asset_id.0.clone())
        };
        Ok(CurrentHolding {
            id,
            account_id: account.clone(),
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
            if let Some(r) = self.store().selected_rate(p, &scope, cutoff)? {
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
            projection.assets
        } else {
            projection.providers
        };
        let search = search.to_lowercase();
        nav.retain(|n| {
            n.name.to_lowercase().contains(&search)
                || n.ticker
                    .as_ref()
                    .is_some_and(|t| t.to_lowercase().contains(&search))
        });
        if by_asset && "dust".contains(&search) && projection.holdings.iter().any(|h| h.is_dust) {
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
        Selection::Provider(id) => h.provider_id == *id,
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
    if let Selection::Asset(asset) = &selection {
        for role in [PositionRole::Asset, PositionRole::Liability] {
            let group: Vec<_> = group.iter().filter(|h| h.role == role).collect();
            if let Some(h) = group.first() {
                let mut units = Units::from(0);
                for h in &group {
                    units += &h.quantity.units
                }
                quantities.push((
                    role,
                    QuantityDto {
                        asset_id: asset.clone(),
                        units,
                        scale: h.quantity.scale,
                    },
                ));
            }
        }
    }
    NavigationEntry {
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
        amount(&b.summary.current_value)
            .cmp(&amount(&a.summary.current_value))
            .then(a.name.cmp(&b.name))
            .then_with(|| format!("{:?}", a.selection).cmp(&format!("{:?}", b.selection)))
    });
}
fn node_id(t: &AllocationTarget) -> String {
    serde_json::to_string(t).expect("serializable target")
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
        let pt = AllocationTarget::Provider(h.provider_id.clone());
        let pname = metadata
            .providers
            .iter()
            .find(|p| p.id == h.provider_id)
            .and_then(|p| p.name.clone())
            .unwrap_or(h.provider_id.0.clone());
        add(pt.clone(), None, pname);
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
                        c.local_label.clone()
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
    Ok(AllocationTree {
        selection,
        roots,
        nodes: nodes.into_values().collect(),
    })
}
impl PortfolioProjection {
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
