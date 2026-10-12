use anyhow::{Context, Result};
use chrono::Utc;
use lib_app::refresh::{RefreshOutcome, RefreshTrigger};
use lib_app::{
    Application,
    config::{AppConfig, DATABASE_FILENAME, ResolvedConfig},
    portfolio::Selection,
    startup::OpenAction,
};
use lib_core::{ui_metadata::*, *};
use money_tracker_desktop::{Session, Startup};
use serde_json::json;
fn resolved(dir: &std::path::Path) -> ResolvedConfig {
    ResolvedConfig {
        database: dir.join(DATABASE_FILENAME),
        config_file: dir.join("config.json"),
        config: AppConfig::default(),
        legacy_database: None,
        cache_dir: dir.join("cache"),
    }
}
fn seed(path: &std::path::Path) -> Result<()> {
    let app = Application::open(path)?;
    let s = app.store();
    let date = "2026-01-01T00:00:00Z".parse()?;
    s.save_account(&Account {
        id: "wallet".into(),
        portfolio_id: DEFAULT_PORTFOLIO.into(),
        provider_id: "example".into(),
        label: "My wallet".into(),
        source_reference: Some("wallet-reference".into()),
    })?;
    s.save_account(&Account {
        id: "dust-wallet".into(),
        portfolio_id: DEFAULT_PORTFOLIO.into(),
        provider_id: "tiny".into(),
        label: "Small wallet".into(),
        source_reference: None,
    })?;
    for (id, asset, scale, q, role) in [
        ("cash", "USD", 2, "1000", PositionRole::Asset),
        ("loan", "USD", 2, "100", PositionRole::Liability),
        (
            "unknown",
            "MYSTERY",
            18,
            "9007199254740993.000000000000000001",
            PositionRole::Asset,
        ),
        ("dust", "USD", 2, "0.5", PositionRole::Asset),
    ] {
        let e = TxnEffect {
            datetime: date,
            position_id: id.into(),
            amount: Units::from_decimal(q, scale)?,
        };
        s.save_account_import(
            &if id == "dust" {
                "dust-wallet"
            } else {
                "wallet"
            }
            .into(),
            "fixture",
            id,
            &[(id.into(), AssetId::str(asset), scale, role)],
            &[(
                id.into(),
                Transaction {
                    datetime: date,
                    inputs: if role == PositionRole::Liability {
                        vec![e.clone()]
                    } else {
                        vec![]
                    },
                    outputs: if role == PositionRole::Liability {
                        vec![]
                    } else {
                        vec![e]
                    },
                },
            )],
            &[],
        )?;
    }
    s.save_rate_evidence(&RateEvidence {
        id: "cash-rate".into(),
        portfolio_id: DEFAULT_PORTFOLIO.into(),
        scope: RateScope::AccountAsset {
            account: "wallet".into(),
            asset: AssetId::str("USD"),
        },
        effective_at: date,
        rate: Some(AnnualRate {
            fraction: "0.05".parse()?,
            basis: AnnualRateBasis::Apy,
            compounds_per_year: None,
            reward_asset: None,
        }),
        evidence: SourceEvidence {
            source: "fixture".into(),
            reference: "terms".into(),
            observed_at: date,
            updated_at: date,
        },
    })?;
    Ok(())
}
#[tokio::test]
async fn real_wire_fixture_scopes_unknowns_debt_and_exact_quantities() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let config = resolved(temp.path());
    seed(&config.database)?;
    let mut session = Session::new(config);
    let ready = session.startup().await?;
    let Startup::Ready { data, .. } = &ready else {
        panic!()
    };
    assert_eq!(data.holding_count, 4);
    assert_eq!(data.dust_count, 1); // Dust grouping preserves the whole asset total.
    assert_eq!(
        data.overview.current_value.clone().status_for_test(),
        "partial"
    );
    let usd = Selection::Asset(AssetId::str("USD"));
    let tree = session.tree(data.snapshot_id, usd.clone())?;
    let account = tree
        .nodes
        .iter()
        .find(|n| n.node.name == "My wallet")
        .unwrap();
    let d = session.details(data.snapshot_id, usd.clone(), &account.node.id)?;
    assert!(
        d.holdings
            .iter()
            .all(|h| h.quantity.asset_id == AssetId::str("USD"))
    );
    assert!(d.holdings.iter().any(|h| h.role == PositionRole::Liability));
    let mystery = session.tree(data.snapshot_id, Selection::Asset(AssetId::str("MYSTERY")))?;
    let unknown = session.details(
        data.snapshot_id,
        Selection::Asset(AssetId::str("MYSTERY")),
        &mystery.roots[0],
    )?;
    assert_eq!(
        unknown.holdings[0].quantity.units.to_string(),
        "9007199254740993000000000000000001"
    );
    let provider = session.tree(data.snapshot_id, Selection::Provider("example".into()))?;
    assert!(provider.nodes.iter().all(|n| !matches!(
        n.node.target,
        lib_app::portfolio::AllocationTarget::Provider(_)
    )));
    if let Some(destination) = std::env::var_os("UPDATE_DESKTOP_FIXTURE") {
        let mut trees = serde_json::Map::new();
        let mut details = serde_json::Map::new();
        for sel in data
            .assets
            .iter()
            .chain(&data.providers)
            .map(|e| e.selection.clone())
            .chain([Selection::Dust])
        {
            let t = session.tree(data.snapshot_id, sel.clone())?;
            for n in &t.nodes {
                details.insert(
                    format!("{}:{}", serde_json::to_string(&sel)?, n.node.id),
                    serde_json::to_value(session.details(
                        data.snapshot_id,
                        sel.clone(),
                        &n.node.id,
                    )?)?,
                );
            }
            trees.insert(serde_json::to_string(&sel)?, serde_json::to_value(t)?);
        }
        std::fs::write(
            destination,
            serde_json::to_vec_pretty(&json!({"startup":ready,"trees":trees,"details":details}))?,
        )?;
    }
    session.close().await?;
    Ok(())
}
fn amount(
    figure: &lib_app::dto::FigureDto<lib_app::portfolio::CalculatedMoney>,
) -> Option<(&'static str, &'static str)> {
    // Only the deterministic fixture ratios are compared here.
    match figure {
        lib_app::dto::FigureDto::Known { value, .. }
        | lib_app::dto::FigureDto::Partial { value, .. } => match (
            value.amount.numerator().as_str(),
            value.amount.denominator().as_str(),
        ) {
            ("50", "1") => Some(("50", "1")),
            ("60", "1") => Some(("60", "1")),
            _ => None,
        },
        _ => None,
    }
}

trait Status {
    fn status_for_test(&self) -> &str;
}
impl<T> Status for lib_app::dto::FigureDto<T> {
    fn status_for_test(&self) -> &str {
        match self {
            Self::Known { .. } => "known",
            Self::Partial { .. } => "partial",
            Self::Unknown { .. } => "unknown",
            Self::NotApplicable => "not_applicable",
        }
    }
}
/// Stage 8 evidence: a configured source starts unscheduled, a manual run
/// re-reads it offline, and the store stays authoritative across a restart.
#[tokio::test]
async fn refresh_starts_unscheduled_imports_exactly_and_survives_restart() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let export = temp.path().join("nexo.csv");
    // A USD-denominated unit needs no market lookup, so this stays offline.
    std::fs::write(
        &export,
        "Transaction,Type,Input Currency,Input Amount,Output Currency,Output Amount,USD Equivalent,Fee,Fee Currency,Details,Date / Time (UTC)\n\
         id,Top up Crypto,USDX,2,USDX,2,$2,-,-,approved / deposit,2025-01-01 00:00:00\n",
    )?;
    let mut config = resolved(temp.path());
    config.config.nexo_csv_path = Some(export);
    seed(&config.database)?;
    let mut session = Session::new(config.clone());
    let Startup::Ready { data, .. } = session.startup().await? else {
        panic!()
    };
    // The configured export is a source; the held MYSTERY asset has no reviewed
    // price mapping and is never fabricated.
    let sources: Vec<_> = data.refresh.iter().map(|s| s.source.clone()).collect();
    assert_eq!(
        sources,
        vec!["prices".to_string(), "provider:NEXO".to_string()]
    );
    assert!(data.refresh.iter().all(|s| s.next_due.is_none()));

    // A launch starts the interval without contacting or reading anything.
    let now = Utc::now();
    let plan = session.refresh_plan(RefreshTrigger::Launch, now).await?;
    assert_eq!(plan.runs().count(), 0);
    let (startup, report) = session.refresh_commit(plan, Vec::new(), now).await?;
    assert!(
        report
            .entries
            .iter()
            .all(|e| matches!(e.outcome, RefreshOutcome::Scheduled))
    );
    let Startup::Ready { data, .. } = &startup else {
        panic!()
    };
    assert!(data.refresh.iter().all(|s| s.next_due.is_some()));

    // A manual run re-reads the export; the same bytes are not imported twice.
    let (startup, report) = session.refresh(RefreshTrigger::Manual, now).await?;
    assert!(report.entries.iter().any(|e| e.source == "provider:NEXO"));
    assert!(matches!(
        report
            .entries
            .iter()
            .find(|e| e.source == "provider:NEXO")
            .map(|e| &e.outcome),
        Some(RefreshOutcome::Updated { detail }) if detail == "1 transactions"
    ));
    let Startup::Ready { data, .. } = &startup else {
        panic!()
    };
    let nexo = data
        .refresh
        .iter()
        .find(|s| s.source == "provider:NEXO")
        .context("Nexo source")?;
    assert_eq!(nexo.last_success, Some(now));
    session.close().await?;

    // Reopening restores the schedule and freshness from the database.
    let mut reopened = Session::new(config);
    let Startup::Ready { data, .. } = reopened.startup().await? else {
        panic!()
    };
    let nexo = data
        .refresh
        .iter()
        .find(|s| s.source == "provider:NEXO")
        .context("Nexo source after restart")?;
    assert_eq!(nexo.last_success, Some(now));
    let interval = reopened.settings().await?.refresh_interval_seconds;
    assert_eq!(
        nexo.next_due,
        Some(now + chrono::Duration::seconds(interval as i64))
    );
    reopened.close().await?;
    Ok(())
}

/// Stage 7 evidence: a manual rate edit changes the projection, Undo restores the
/// source figure, and both the action log and compensations survive a restart.
#[tokio::test]
async fn manual_rate_edit_then_undo_restores_source_earnings() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let config = resolved(temp.path());
    seed(&config.database)?;
    let mut session = Session::new(config.clone());
    let Startup::Ready { data, .. } = session.startup().await? else {
        panic!()
    };
    assert_eq!(
        amount(&data.overview.gross_annual_earnings),
        Some(("50", "1"))
    );
    let scope = RateScope::AccountAsset {
        account: "wallet".into(),
        asset: AssetId::str("USD"),
    };
    let rate = AnnualRate {
        fraction: "0.06".parse()?,
        basis: AnnualRateBasis::Apy,
        compounds_per_year: None,
        reward_asset: None,
    };
    let Startup::Ready { data, .. } = session
        .apply_edit(lib_app::manual::ManualEdit::Rate {
            scope: scope.clone(),
            rate: Some(rate),
            effective_at: Utc::now(),
            pinned: false,
            clear: false,
        })
        .await?
    else {
        panic!()
    };
    assert_eq!(
        amount(&data.overview.gross_annual_earnings),
        Some(("60", "1"))
    );
    let undoable = data
        .edits
        .actions
        .iter()
        .find(|a| !a.undone)
        .context("logged action")?;
    let Startup::Ready { data, .. } = session.undo_action(undoable.id.clone()).await? else {
        panic!()
    };
    assert_eq!(
        amount(&data.overview.gross_annual_earnings),
        Some(("50", "1"))
    );
    assert_eq!(data.edits.actions.len(), 2);
    assert!(
        data.edits
            .actions
            .iter()
            .any(|a| a.undone && a.id == undoable.id)
    );
    assert!(
        data.edits
            .actions
            .iter()
            .any(|a| a.undo_of.as_deref() == Some(undoable.id.as_str()))
    );
    session.close().await?;
    // Reopening keeps the source figure and the full action history.
    let mut reopened = Session::new(config);
    let Startup::Ready { data, .. } = reopened.startup().await? else {
        panic!()
    };
    assert_eq!(
        amount(&data.overview.gross_annual_earnings),
        Some(("50", "1"))
    );
    assert_eq!(data.edits.actions.len(), 2);
    assert!(
        data.edits
            .actions
            .iter()
            .any(|a| a.undo_of.as_deref() == Some(undoable.id.as_str()))
    );
    reopened.close().await?;
    Ok(())
}
/// Dust classification, spam exclusion and local grouping change only presentation.
#[tokio::test]
async fn dust_spam_and_grouping_controls_change_the_projection() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let config = resolved(temp.path());
    seed(&config.database)?;
    let mut session = Session::new(config.clone());
    let Startup::Ready { data, .. } = session.startup().await? else {
        panic!()
    };
    assert_eq!(data.dust_count, 1);
    let Startup::Ready { data, .. } = session
        .apply_edit(lib_app::manual::ManualEdit::Dust {
            provider: "tiny".into(),
            asset: AssetId::str("USD"),
            mode: DustMode::Significant,
        })
        .await?
    else {
        panic!()
    };
    assert_eq!(data.dust_count, 0);
    let Startup::Ready { data, .. } = session
        .apply_edit(lib_app::manual::ManualEdit::Spam {
            asset: AssetId::str("MYSTERY"),
            excluded: true,
            reason: "unwanted airdrop".into(),
        })
        .await?
    else {
        panic!()
    };
    assert!(!data.assets.iter().any(|e| e.name == "MYSTERY"));
    // A local group becomes a valid parent and reparents the account.
    session
        .apply_edit(lib_app::manual::ManualEdit::Group {
            provider: "example".into(),
            name: "Fixed terms".into(),
            parent: None,
        })
        .await?;
    let Startup::Ready { data, .. } = session
        .apply_edit(lib_app::manual::ManualEdit::Move {
            account: "wallet".into(),
            parent: None,
            source_default: false,
        })
        .await?
    else {
        panic!()
    };
    let group = data
        .edits
        .accounts
        .iter()
        .find(|a| a.account.label == "Fixed terms")
        .map(|a| a.account.id.clone())
        .context("group account")?;
    let Startup::Ready { data, .. } = session
        .apply_edit(lib_app::manual::ManualEdit::Move {
            account: "wallet".into(),
            parent: Some(group.clone()),
            source_default: false,
        })
        .await?
    else {
        panic!()
    };
    let tree = session.tree(data.snapshot_id, Selection::Asset(AssetId::str("USD")))?;
    let wallet = tree
        .nodes
        .iter()
        .find(|n| n.node.name == "My wallet")
        .context("wallet node")?;
    let parent = wallet
        .node
        .parent
        .as_ref()
        .and_then(|id| tree.nodes.iter().find(|n| &n.node.id == id))
        .context("parent node")?;
    assert!(matches!(
        parent.node.target,
        lib_app::portfolio::AllocationTarget::Account(_)
    ));
    assert_eq!(parent.node.name, "Fixed terms");
    session.close().await?;
    Ok(())
}

#[tokio::test]
#[ignore = "offline smoke on temporary copies of private imported databases"]
async fn cached_imported_portfolios_are_browsable() -> Result<()> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let mut populated = 0;
    for relative in [
        ".cache/test-data/test-nexo_05-10-2026.duckdb",
        ".cache/test-data/test-binance-api-v1.duckdb",
        ".cache/test-data/binance/test-binance-statement-v1.duckdb",
    ] {
        let source = root.join(relative);
        let before = std::fs::read(&source)?;
        let temp = tempfile::tempdir()?;
        let mut session = Session::new(resolved(temp.path()));
        let Startup::Ready { data, .. } = session
            .open(OpenAction::CopyExisting(source.clone()))
            .await?
        else {
            panic!()
        };
        if data.holding_count > 0 {
            populated += 1;
        }
        for entry in data.assets.iter().chain(&data.providers) {
            let t = session.tree(data.snapshot_id, entry.selection.clone())?;
            for node in &t.nodes {
                let d =
                    session.details(data.snapshot_id, entry.selection.clone(), &node.node.id)?;
                assert!(!d.holdings.is_empty());
            }
        }
        session.close().await?;
        assert_eq!(std::fs::read(source)?, before);
    }
    assert!(
        populated >= 2,
        "expected populated Nexo and Binance statement copies"
    );
    Ok(())
}
