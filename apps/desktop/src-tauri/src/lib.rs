//! Narrow desktop boundary: one owner, one cached snapshot, no arbitrary paths or SQL.
use anyhow::{Result, ensure};
use chrono::{DateTime, Utc};
use lib_app::{
    config::{ResolvedConfig, Settings},
    dto::{FigureDto, IssueDto, QuantityDto},
    portfolio::{CalculatedMoney, NavigationEntry, PortfolioProjection, Selection, Summary},
    startup::{MissingDatabase, OpenAction},
    worker::{DatabaseWorker, StartupOutcome},
};
use lib_core::{
    PositionRole,
    ui_metadata::{SelectedPrice, SelectedRate, Subscription},
};
use serde::Serialize;

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
}
#[derive(Serialize)]
pub struct HoldingDetails {
    pub label: String,
    pub role: PositionRole,
    pub quantity: QuantityDto,
    pub value: FigureDto<CalculatedMoney>,
    pub annual_earnings: FigureDto<CalculatedMoney>,
    pub annual_borrowing_cost: FigureDto<CalculatedMoney>,
    pub subscription: Option<Subscription>,
    pub rate: Option<SelectedRate>,
    pub price: Option<SelectedPrice>,
    pub verification_current: bool,
    pub issues: Vec<IssueDto>,
}
#[derive(Serialize)]
pub struct NodeDetails {
    pub name: String,
    pub summary: Summary,
    pub original_name: Option<String>,
    pub source_reference: Option<String>,
    pub holdings: Vec<HoldingDetails>,
    pub issues: Vec<IssueDto>,
}

#[derive(Serialize)]
pub struct DesktopNode {
    #[serde(flatten)]
    pub node: lib_app::portfolio::AllocationNode,
    pub liability: bool,
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
    pub async fn snapshot(&mut self) -> Result<Snapshot> {
        let p = self.worker()?.portfolio_projection(Utc::now()).await?;
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
                    DesktopNode { node, liability }
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
                    label: h.label,
                    role: h.role,
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
