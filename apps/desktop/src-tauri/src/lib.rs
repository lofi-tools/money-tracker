//! Narrow desktop boundary: one owner, one cached snapshot, no arbitrary paths or SQL.
use anyhow::{Result, ensure};
use chrono::{DateTime, Utc};
use lib_app::{
    config::{ResolvedConfig, Settings},
    dto::UiAccountDto,
    manual::ManualEdit,
    portfolio::{
        AllocationTarget, CalculatedMoney, CurrentHolding, NavigationEntry, PortfolioProjection,
        Selection, Summary,
    },
    startup::{MissingDatabase, OpenAction},
    worker::{DatabaseWorker, StartupOutcome},
};
use lib_core::{
    AccountId, PositionRole, ProviderId,
    ui_metadata::{
        LocalRecord, LocalTarget, LocalValue, ManualAction, NodeKind, RateScope, SelectedPrice,
        SelectedRate, Subscription,
    },
};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Startup {
    Missing {
        database: MissingDatabase,
        settings: Settings,
    },
    Ready {
        data: Snapshot,
        settings: Settings,
    },
}
/// One typed local edit target with its current revision and stored value.
#[derive(Clone, Serialize)]
pub struct EditSlot {
    pub target: LocalTarget,
    pub revision: i64,
    pub value: Option<LocalValue>,
}
/// Rate/price overrides for leaf holding rows in an allocation table.
#[derive(Clone, Serialize, Default)]
pub struct NodeEdits {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate: Option<EditSlot>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<EditSlot>,
    /// Selected quote the price editor starts from, when one is known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_price: Option<SelectedPrice>,
}
/// Every manual control available for one current holding.
#[derive(Clone, Serialize)]
pub struct HoldingEdits {
    pub rate: Option<EditSlot>,
    pub price: Option<EditSlot>,
    pub dust: Option<EditSlot>,
    pub spam: EditSlot,
    pub name: Option<EditSlot>,
}
#[derive(Clone, Serialize)]
pub struct AccountOption {
    pub id: Option<AccountId>,
    pub name: String,
    pub group: bool,
}
/// Account grouping controls: rename, move and the valid-parent candidates.
#[derive(Clone, Serialize)]
pub struct NodeAccount {
    pub id: AccountId,
    pub provider_id: ProviderId,
    pub parent: Option<AccountId>,
    pub group: bool,
    pub source_label: String,
    pub local_label: Option<String>,
    pub slot: EditSlot,
    pub parent_options: Vec<AccountOption>,
}
#[derive(Clone, Serialize)]
pub struct ActionEntry {
    pub id: String,
    pub at: DateTime<Utc>,
    pub undone: bool,
    pub target: LocalTarget,
    pub undo_of: Option<String>,
    /// Resulting local revision; Undo is offered only while it still matches.
    pub revision: i64,
    pub before: Option<LocalValue>,
    pub after: Option<LocalValue>,
}
/// Persisted local state and the ordered action log behind Undo.
#[derive(Clone, Serialize)]
pub struct EditContext {
    pub accounts: Vec<UiAccountDto>,
    pub local_state: Vec<(LocalTarget, LocalRecord)>,
    pub actions: Vec<ActionEntry>,
}
#[derive(Clone, Serialize)]
pub struct Snapshot {
    pub contract_version: u32,
    pub snapshot_id: u64,
    pub cutoff: DateTime<Utc>,
    pub overview: Summary,
    pub assets: Vec<NavigationEntry>,
    pub providers: Vec<NavigationEntry>,
    pub dust: NavigationEntry,
    pub holding_count: usize,
    pub dust_count: usize,
    pub edits: EditContext,
}
#[derive(Serialize)]
pub struct HoldingDetails {
    pub label: String,
    pub role: PositionRole,
    pub provider_id: ProviderId,
    pub quantity: lib_app::dto::QuantityDto,
    pub value: lib_app::dto::FigureDto<CalculatedMoney>,
    pub annual_earnings: lib_app::dto::FigureDto<CalculatedMoney>,
    pub annual_borrowing_cost: lib_app::dto::FigureDto<CalculatedMoney>,
    pub subscription: Option<Subscription>,
    pub rate: Option<SelectedRate>,
    pub price: Option<SelectedPrice>,
    pub verification_current: bool,
    pub issues: Vec<lib_app::dto::IssueDto>,
    pub edits: HoldingEdits,
}
#[derive(Serialize)]
pub struct NodeDetails {
    pub name: String,
    pub summary: Summary,
    pub original_name: Option<String>,
    pub source_reference: Option<String>,
    pub holdings: Vec<HoldingDetails>,
    pub issues: Vec<lib_app::dto::IssueDto>,
    pub account: Option<NodeAccount>,
}

#[derive(Serialize)]
pub struct DesktopNode {
    #[serde(flatten)]
    pub node: lib_app::portfolio::AllocationNode,
    pub liability: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edits: Option<NodeEdits>,
}
#[derive(Serialize)]
pub struct DesktopTree {
    pub selection: Selection,
    pub roots: Vec<String>,
    pub nodes: Vec<DesktopNode>,
    pub initially_expanded: Vec<String>,
}

pub struct Session {
    resolved: ResolvedConfig,
    worker: Option<DatabaseWorker>,
    projection: Option<PortfolioProjection>,
    snapshot_id: u64,
    closed: bool,
}
impl Session {
    pub fn new(resolved: ResolvedConfig) -> Self {
        Self {
            resolved,
            worker: None,
            projection: None,
            snapshot_id: 0,
            closed: false,
        }
    }
    pub async fn open(&mut self, action: OpenAction) -> Result<Startup> {
        ensure!(!self.closed, "session closed");
        ensure!(self.worker.is_none(), "database already open");
        match DatabaseWorker::start(self.resolved.clone(), action).await? {
            StartupOutcome::Missing(database) => Ok(Startup::Missing {
                database,
                settings: self.resolved.config.settings.clone(),
            }),
            StartupOutcome::Ready(worker) => {
                self.worker = Some(worker);
                let data = self.snapshot().await?;
                let settings = self.settings().await?;
                Ok(Startup::Ready { data, settings })
            }
        }
    }
    pub async fn startup(&mut self) -> Result<Startup> {
        if self.worker.is_some() {
            Ok(Startup::Ready {
                data: self.snapshot().await?,
                settings: self.settings().await?,
            })
        } else {
            self.open(OpenAction::OpenExisting).await
        }
    }
    fn worker(&self) -> Result<&DatabaseWorker> {
        ensure!(!self.closed, "session closed");
        self.worker
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("database not open"))
    }
    pub async fn settings(&self) -> Result<Settings> {
        self.worker()?.settings().await
    }
    pub async fn save_settings(&mut self, settings: Settings) -> Result<Startup> {
        self.worker()?.save_settings(settings).await?;
        Ok(Startup::Ready {
            data: self.snapshot().await?,
            settings: self.settings().await?,
        })
    }
    pub async fn refresh_prices(&mut self) -> Result<Startup> {
        self.worker()?.refresh_prices().await?;
        Ok(Startup::Ready {
            data: self.snapshot().await?,
            settings: self.settings().await?,
        })
    }
    /// One typed manual edit; the snapshot is rebuilt so stale IDs are rejected.
    pub async fn apply_edit(&mut self, edit: ManualEdit) -> Result<Startup> {
        self.worker()?.apply_manual_edit(edit).await?;
        Ok(Startup::Ready {
            data: self.snapshot().await?,
            settings: self.settings().await?,
        })
    }
    /// Compensating action for one logged manual change.
    pub async fn undo_action(&mut self, id: String) -> Result<Startup> {
        self.worker()?.undo_manual_action(id).await?;
        Ok(Startup::Ready {
            data: self.snapshot().await?,
            settings: self.settings().await?,
        })
    }
    pub async fn snapshot(&mut self) -> Result<Snapshot> {
        let p = self.worker()?.portfolio_projection(Utc::now()).await?;
        let actions = self.worker()?.manual_actions().await?;
        self.snapshot_id = self
            .snapshot_id
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("snapshot limit"))?;
        let out = Snapshot {
            contract_version: p.contract_version,
            snapshot_id: self.snapshot_id,
            cutoff: p.request.context.cutoff,
            overview: p.overview.clone(),
            assets: p
                .assets
                .iter()
                .filter(|entry| {
                    let Selection::Asset(asset) = &entry.selection else {
                        return true;
                    };
                    // An asset significant anywhere keeps its full cross-provider total.
                    // Only wholly dust assets fold into the Dust navigation entry.
                    !p.holdings
                        .iter()
                        .filter(|h| &h.quantity.asset_id == asset)
                        .all(|h| h.is_dust)
                })
                .cloned()
                .collect(),
            providers: p.providers.clone(),
            dust: p.dust.clone(),
            holding_count: p.holdings.len(),
            dust_count: p.holdings.iter().filter(|h| h.is_dust).count(),
            edits: edit_context(&p, &actions),
        };
        self.projection = Some(p);
        Ok(out)
    }
    fn projection(&self, id: u64) -> Result<&PortfolioProjection> {
        ensure!(!self.closed && id == self.snapshot_id, "stale snapshot");
        self.projection
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("snapshot not ready"))
    }
    pub fn tree(&self, id: u64, selection: Selection) -> Result<DesktopTree> {
        let p = self.projection(id)?;
        let mut t = p.allocation_tree(selection.clone())?;
        if matches!(selection, Selection::Provider(_)) {
            let providers: Vec<_> = t
                .nodes
                .iter()
                .filter(|n| matches!(n.target, lib_app::portfolio::AllocationTarget::Provider(_)))
                .map(|n| n.id.clone())
                .collect();
            t.nodes.retain(|n| !providers.contains(&n.id));
            for node in &mut t.nodes {
                if node
                    .parent
                    .as_ref()
                    .is_some_and(|id| providers.contains(id))
                {
                    node.parent = None;
                }
            }
            t.roots = t
                .roots
                .into_iter()
                .flat_map(|id| {
                    p.allocation
                        .nodes
                        .iter()
                        .find(|n| n.id == id)
                        .map(|n| n.children.clone())
                        .unwrap_or_default()
                })
                .collect();
        }
        let initially_expanded = t
            .nodes
            .iter()
            .filter(|n| {
                matches!(n.target, lib_app::portfolio::AllocationTarget::Provider(_))
                    || (matches!(n.target, lib_app::portfolio::AllocationTarget::Account(_))
                        && n.parent.as_ref().is_none_or(|parent| {
                            t.nodes.iter().any(|p| {
                                p.id == *parent
                                    && matches!(
                                        p.target,
                                        lib_app::portfolio::AllocationTarget::Provider(_)
                                    )
                            })
                        }))
            })
            .map(|n| n.id.clone())
            .collect();
        Ok(DesktopTree {
            selection: t.selection,
            roots: t.roots,
            initially_expanded,
            nodes: t
                .nodes
                .into_iter()
                .map(|node| {
                    let liability = !node.holding_ids.is_empty()
                        && p.holdings
                            .iter()
                            .filter(|h| node.holding_ids.contains(&h.id))
                            .all(|h| h.role == PositionRole::Liability);
                    // Leaf holding rows carry their own rate/price override controls;
                    // rollup rows display derived figures that cannot be edited directly.
                    let edits = match &node.target {
                        AllocationTarget::Holding(id) => p
                            .holdings
                            .iter()
                            .find(|h| &h.id == id)
                            .map(|h| node_edits(p, h)),
                        _ => None,
                    };
                    DesktopNode {
                        node,
                        liability,
                        edits,
                    }
                })
                .collect(),
        })
    }
    pub fn details(&self, id: u64, selection: Selection, node_id: &str) -> Result<NodeDetails> {
        let p = self.projection(id)?;
        // Resolve IDs inside the selected tree, not through a frontend-supplied target.
        let tree = p.allocation_tree(selection)?;
        let node = tree
            .nodes
            .iter()
            .find(|n| n.id == node_id)
            .ok_or_else(|| anyhow::anyhow!("allocation not in selection"))?;
        let d = p.details(node.target.clone())?;
        Ok(NodeDetails {
            name: node.name.clone(),
            summary: node.summary.clone(),
            original_name: d.original_account.as_ref().map(|a| a.label.clone()),
            source_reference: d.original_account.and_then(|a| a.source_reference),
            issues: node.issues.clone(),
            holdings: d
                .holdings
                .into_iter()
                .filter(|h| node.holding_ids.contains(&h.id))
                .map(|h| HoldingDetails {
                    edits: holding_edits(p, &h),
                    label: h.label,
                    role: h.role,
                    provider_id: h.provider_id.clone(),
                    quantity: h.quantity,
                    value: h.value,
                    annual_earnings: h.annual_earnings,
                    annual_borrowing_cost: h.annual_borrowing_cost,
                    subscription: h.subscription,
                    rate: h.rate,
                    price: h.price,
                    verification_current: h.verification_current,
                    issues: h.issues,
                })
                .collect(),
            account: account_controls(p, &node.target),
        })
    }
    pub async fn close(&mut self) -> Result<()> {
        self.closed = true;
        self.projection = None;
        if let Some(w) = self.worker.take() {
            w.close().await?;
        }
        Ok(())
    }
}

/// Local-state lookup keyed by the serialized target, so slots carry revisions.
fn local_index(p: &PortfolioProjection) -> Vec<(String, LocalRecord)> {
    p.metadata
        .local_state
        .iter()
        .map(|(target, record)| {
            (
                serde_json::to_string(target).unwrap_or_default(),
                record.clone(),
            )
        })
        .collect()
}
fn slot(index: &[(String, LocalRecord)], target: LocalTarget) -> EditSlot {
    let key = serde_json::to_string(&target).unwrap_or_default();
    let record = index
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, r)| r.clone())
        .unwrap_or(LocalRecord {
            revision: 0,
            value: None,
        });
    EditSlot {
        target,
        revision: record.revision,
        value: record.value,
    }
}
/// The editable scope for a holding: the resolved evidence scope when one applies,
/// otherwise the finest valid manual override scope for this position.
fn rate_scope_for(h: &CurrentHolding) -> Option<RateScope> {
    if let Some(scope) = &h.rate_scope {
        return Some(scope.clone());
    }
    if h.role == PositionRole::Liability {
        return (h.position_ids.len() == 1)
            .then(|| RateScope::BorrowingPosition(h.position_ids[0].clone()));
    }
    if let Some(subscription) = &h.subscription {
        return Some(RateScope::Subscription(subscription.id.clone()));
    }
    Some(RateScope::AccountAsset {
        account: h.account_id.clone(),
        asset: h.quantity.asset_id.clone(),
    })
}
fn price_target(h: &CurrentHolding, reporting: &lib_core::AssetId) -> Option<LocalTarget> {
    (h.quantity.asset_id != *reporting).then(|| LocalTarget::Price {
        asset: h.quantity.asset_id.clone(),
        quote: reporting.clone(),
    })
}
fn node_edits(p: &PortfolioProjection, h: &CurrentHolding) -> NodeEdits {
    let index = local_index(p);
    NodeEdits {
        rate: rate_scope_for(h).map(|scope| slot(&index, LocalTarget::Rate { scope })),
        price: price_target(h, &p.request.reporting_asset).map(|target| slot(&index, target)),
        selected_price: h.price.clone(),
    }
}
fn holding_edits(p: &PortfolioProjection, h: &CurrentHolding) -> HoldingEdits {
    let index = local_index(p);
    let dust = slot(
        &index,
        LocalTarget::Dust {
            provider: h.provider_id.clone(),
            asset: h.quantity.asset_id.clone(),
        },
    );
    HoldingEdits {
        rate: rate_scope_for(h).map(|scope| slot(&index, LocalTarget::Rate { scope })),
        price: price_target(h, &p.request.reporting_asset).map(|target| slot(&index, target)),
        dust: (h.role == PositionRole::Asset).then_some(dust),
        spam: slot(
            &index,
            LocalTarget::Spam {
                asset: h.quantity.asset_id.clone(),
            },
        ),
        name: h
            .subscription
            .as_ref()
            .map(|s| slot(&index, LocalTarget::Subscription { id: s.id.clone() })),
    }
}
fn local_label(p: &PortfolioProjection, id: &AccountId) -> Option<String> {
    p.metadata
        .local_state
        .iter()
        .find_map(|(target, record)| match (target, &record.value) {
            (LocalTarget::Account { id: target }, Some(LocalValue::Account(custom)))
                if *target == *id =>
            {
                custom.local_label.clone()
            }
            _ => None,
        })
}
fn account_name(p: &PortfolioProjection, id: &AccountId) -> String {
    local_label(p, id).unwrap_or_else(|| {
        p.metadata
            .accounts
            .iter()
            .find(|a| &a.account.id == id)
            .map(|a| a.account.label.clone())
            .unwrap_or_else(|| id.0.clone())
    })
}
fn is_descendant(candidate: &AccountId, ancestor: &AccountId, accounts: &[UiAccountDto]) -> bool {
    let mut current = Some(candidate.clone());
    while let Some(id) = current {
        if id == *ancestor {
            return true;
        }
        current = accounts
            .iter()
            .find(|a| a.account.id == id)
            .and_then(|a| a.parent.clone());
    }
    false
}
/// Rename/move controls and the cycle-free valid-parent candidates for an account.
fn account_controls(p: &PortfolioProjection, target: &AllocationTarget) -> Option<NodeAccount> {
    let AllocationTarget::Account(id) = target else {
        return None;
    };
    let entry = p.metadata.accounts.iter().find(|a| &a.account.id == id)?;
    let index = local_index(p);
    let same_provider: Vec<_> = p
        .metadata
        .accounts
        .iter()
        .filter(|a| a.account.provider_id == entry.account.provider_id)
        .collect();
    let mut parent_options = vec![AccountOption {
        id: None,
        name: "Top level".into(),
        group: false,
    }];
    parent_options.extend(
        same_provider
            .iter()
            .filter(|a| {
                &a.account.id != id && !is_descendant(&a.account.id, id, &p.metadata.accounts)
            })
            .map(|a| AccountOption {
                id: Some(a.account.id.clone()),
                name: account_name(p, &a.account.id),
                group: matches!(a.structure.kind, NodeKind::Group),
            }),
    );
    Some(NodeAccount {
        id: id.clone(),
        provider_id: entry.account.provider_id.clone(),
        parent: entry.parent.clone(),
        group: matches!(entry.structure.kind, NodeKind::Group),
        source_label: entry.account.label.clone(),
        local_label: local_label(p, id),
        slot: slot(&index, LocalTarget::Account { id: id.clone() }),
        parent_options,
    })
}
fn edit_context(p: &PortfolioProjection, actions: &[ManualAction]) -> EditContext {
    let undone: BTreeMap<_, _> = actions
        .iter()
        .filter_map(|a| a.undo_of.clone().map(|id| (id, a.id.clone())))
        .collect();
    EditContext {
        accounts: p.metadata.accounts.clone(),
        local_state: p.metadata.local_state.clone(),
        actions: actions
            .iter()
            .rev()
            .map(|a| ActionEntry {
                id: a.id.clone(),
                at: a.at,
                undone: undone.contains_key(&a.id),
                target: a.target.clone(),
                undo_of: a.undo_of.clone(),
                revision: a.after.revision,
                before: a.before.value.clone(),
                after: a.after.value.clone(),
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lib_app::config::{AppConfig, DATABASE_FILENAME};
    #[tokio::test]
    async fn missing_new_snapshot_settings_and_close() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let resolved = ResolvedConfig {
            database: temp.path().join(DATABASE_FILENAME),
            config_file: temp.path().join("config.json"),
            config: AppConfig::default(),
            legacy_database: None,
        };
        let mut s = Session::new(resolved.clone());
        assert!(matches!(s.startup().await?, Startup::Missing { .. }));
        assert!(!resolved.database.exists());
        let Startup::Ready { data, .. } = s.open(OpenAction::StartNew).await? else {
            panic!()
        };
        assert_eq!(data.holding_count, 0);
        assert!(
            s.tree(data.snapshot_id, Selection::Overview)?
                .nodes
                .is_empty()
        );
        assert!(s.tree(data.snapshot_id + 1, Selection::Overview).is_err());
        assert!(
            s.details(data.snapshot_id, Selection::Overview, "unknown")
                .is_err()
        );
        let mut settings = Settings::default();
        settings.theme = lib_app::config::Theme::Dark;
        assert!(matches!(
            s.save_settings(settings.clone()).await?,
            Startup::Ready { .. }
        ));
        assert!(s.tree(data.snapshot_id, Selection::Overview).is_err());
        assert_eq!(s.settings().await?, settings);
        s.close().await?;
        assert!(s.startup().await.is_err());
        s.close().await?;
        Ok(())
    }
    #[tokio::test]
    async fn copy_preserves_source_and_refuses_overwrite() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let source = temp.path().join("source.duckdb");
        drop(lib_app::Application::open(&source)?);
        let bytes = std::fs::read(&source)?;
        let resolved = ResolvedConfig {
            database: temp.path().join(DATABASE_FILENAME),
            config_file: temp.path().join("config.json"),
            config: AppConfig::default(),
            legacy_database: None,
        };
        let mut s = Session::new(resolved.clone());
        assert!(matches!(
            s.open(OpenAction::CopyExisting(source.clone())).await?,
            Startup::Ready { .. }
        ));
        assert!(s.open(OpenAction::StartNew).await.is_err());
        s.close().await?;
        assert_eq!(std::fs::read(source)?, bytes);
        let mut reopened = Session::new(resolved);
        assert!(matches!(reopened.startup().await?, Startup::Ready { .. }));
        reopened.close().await?;
        Ok(())
    }
}
