use anyhow::Result;
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
