//! Offline refresh-lifecycle evidence: scheduling, one local re-read, restart
//! persistence and a single catch-up run. No provider or network access: the
//! exported asset is a USD-denominated unit, so even the price source is exact
//! without a market lookup.
use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use lib_app::{
    Application,
    config::*,
    refresh::{self, RefreshOutcome, RefreshReport, RefreshTrigger},
    startup::OpenAction,
    worker::{DatabaseWorker, StartupOutcome},
};
use lib_core::{Account, AssetId, DEFAULT_PORTFOLIO, PortfolioId, PositionId, PositionRole, Units};
use std::path::{Path, PathBuf};
use tempfile::TempDir;

const DAY: u64 = 86_400;
const NEXO_SOURCE: &str = "provider:NEXO";
const PRICES_SOURCE: &str = "prices";

fn at(seconds: i64) -> DateTime<Utc> {
    DateTime::from_timestamp(seconds, 0).unwrap()
}

fn nexo_csv(amount: &str) -> String {
    format!(
        "Transaction,Type,Input Currency,Input Amount,Output Currency,Output Amount,USD Equivalent,Fee,Fee Currency,Details,Date / Time (UTC)\n\
         id,Top up Crypto,USDX,{amount},USDX,{amount},$2,-,-,approved / deposit,2025-01-01 00:00:00\n"
    )
}

fn outcome<'a>(report: &'a RefreshReport, source: &str) -> &'a RefreshOutcome {
    &report
        .entries
        .iter()
        .find(|entry| entry.source == source)
        .unwrap_or_else(|| panic!("{source} missing from {report:?}"))
        .outcome
}

fn configured(dir: &TempDir, nexo: Option<&Path>) -> ResolvedConfig {
    ResolvedConfig {
        database: dir.path().join(DATABASE_FILENAME),
        config_file: dir.path().join("config.json"),
        config: AppConfig {
            nexo_csv_path: nexo.map(PathBuf::from),
            ..AppConfig::default()
        },
        legacy_database: None,
        cache_dir: dir.path().join("cache"),
    }
}

async fn open_worker(resolved: ResolvedConfig, action: OpenAction) -> Result<DatabaseWorker> {
    match DatabaseWorker::start(resolved, action).await? {
        StartupOutcome::Ready(worker) => Ok(worker),
        StartupOutcome::Missing(_) => anyhow::bail!("expected ready database"),
    }
}

async fn status(worker: &DatabaseWorker, source: &str) -> Result<refresh::RefreshStatus> {
    worker
        .refresh_status()
        .await?
        .into_iter()
        .find(|status| status.source == source)
        .with_context(|| format!("{source} is a configured refresh source"))
}

#[tokio::test]
async fn nexo_refresh_starts_a_schedule_reimports_and_survives_restart() -> Result<()> {
    let dir = TempDir::new()?;
    let export = dir.path().join("nexo.csv");
    std::fs::write(&export, nexo_csv("2"))?;
    let config = configured(&dir, Some(&export));
    // After the export's 2025 activity, so the imported ledger is in range.
    let now = at(1_780_000_000);

    // A first launch starts the interval without contacting or reading anything.
    let worker = open_worker(config.clone(), OpenAction::StartNew).await?;
    let plan = worker.refresh_plan(RefreshTrigger::Launch, now).await?;
    assert_eq!(plan.runs().count(), 0, "launch alone refreshes nothing");
    let report = worker.refresh_commit(plan, vec![], now).await?;
    assert_eq!(report.entries.len(), 1);
    assert!(matches!(
        outcome(&report, NEXO_SOURCE),
        RefreshOutcome::Scheduled
    ));
    let nexo = status(&worker, NEXO_SOURCE).await?;
    assert!(nexo.ready);
    assert_eq!(nexo.next_due, Some(now + Duration::seconds(DAY as i64)));
    assert_eq!(nexo.last_success, None);

    // Nothing is due on the following scheduled tick.
    assert!(
        worker
            .refresh_plan(RefreshTrigger::Scheduled, now + Duration::minutes(1))
            .await?
            .is_idle()
    );

    // An explicit manual run re-reads the local export exactly once.
    let report = worker.refresh(RefreshTrigger::Manual, now).await?;
    assert!(matches!(
        outcome(&report, NEXO_SOURCE),
        RefreshOutcome::Updated { detail } if detail == "1 transactions"
    ));
    assert!(report.failures().next().is_none());
    // The same bytes produce no second import; only the schedule moves on. The
    // newly imported USD-denominated unit also needs no market lookup.
    let report = worker
        .refresh(RefreshTrigger::Manual, now + Duration::seconds(1))
        .await?;
    assert!(matches!(
        outcome(&report, NEXO_SOURCE),
        RefreshOutcome::Unchanged
    ));
    assert!(matches!(
        outcome(&report, PRICES_SOURCE),
        RefreshOutcome::Updated { detail } if detail == "1 exact current prices"
    ));

    // Close/reopen: the schedule and freshness survive, and the imported ledger
    // is exactly the export's quantity.
    worker.close().await?;
    let app = Application::open(&config.database)?;
    assert_eq!(
        app.store().asset_units_at(now + Duration::seconds(2))?,
        vec![(AssetId::str("USDX"), Units::from(200_000_000))]
    );
    drop(app);
    let worker = open_worker(config.clone(), OpenAction::OpenExisting).await?;
    let nexo = status(&worker, NEXO_SOURCE).await?;
    assert_eq!(nexo.last_attempt, Some(now + Duration::seconds(1)));
    assert_eq!(nexo.last_success, Some(now + Duration::seconds(1)));
    assert_eq!(
        nexo.next_due,
        Some(now + Duration::seconds(1) + Duration::seconds(DAY as i64))
    );

    // A launch after the interval elapsed performs exactly one catch-up run,
    // and a changed export is imported in full.
    let plan = worker
        .refresh_plan(RefreshTrigger::Launch, now + Duration::days(2))
        .await?;
    assert_eq!(plan.runs().count(), 2);
    std::fs::write(&export, nexo_csv("3"))?;
    let mut outcomes = Vec::new();
    for item in plan.runs() {
        outcomes.extend(refresh::acquire(item).await);
    }
    let report = worker
        .refresh_commit(plan, outcomes, now + Duration::days(2))
        .await?;
    assert!(matches!(
        outcome(&report, NEXO_SOURCE),
        RefreshOutcome::Updated { detail } if detail == "1 transactions"
    ));
    worker.close().await?;
    let app = Application::open(&config.database)?;
    assert_eq!(
        app.store().asset_units_at(now + Duration::days(2))?,
        vec![(AssetId::str("USDX"), Units::from(300_000_000))]
    );
    Ok(())
}

#[tokio::test]
async fn candidates_report_prerequisites_instead_of_pretending_to_refresh() -> Result<()> {
    let dir = TempDir::new()?;
    let database = dir.path().join(DATABASE_FILENAME);
    let app = Application::open(&database)?;
    let store = app.store();
    for (id, provider) in [
        ("kraken:test", "kraken"),
        ("legacy:binance:spot", "binance"),
        ("wallet", "example"),
    ] {
        store.save_account(&Account {
            id: id.into(),
            portfolio_id: DEFAULT_PORTFOLIO.into(),
            provider_id: provider.into(),
            label: format!("{provider} account"),
            source_reference: None,
        })?;
    }
    store.save_account_import(
        &"kraken:test".into(),
        "fixture",
        "fixture-hash",
        &[(
            PositionId::from("kraken:test:ETH"),
            AssetId::str("ETH"),
            8,
            PositionRole::Asset,
        )],
        &[],
        &[],
    )?;
    let portfolio = PortfolioId::from(DEFAULT_PORTFOLIO);
    let candidates = refresh::candidates(
        store,
        &portfolio,
        &AssetId::str("USD"),
        dir.path(),
        Some(&dir.path().join("missing.csv")),
    )?;
    let prices = candidates
        .iter()
        .find(|candidate| candidate.source == PRICES_SOURCE)
        .context("held non-reporting assets need current prices")?;
    assert!(prices.prerequisite.is_none());
    let kraken = candidates
        .iter()
        .find(|candidate| candidate.source == "account:kraken:test")
        .context("a saved Kraken account is a refresh source")?;
    // Credentials come from the app's own environment; without them the source
    // stays visible and blocked rather than failing on every run.
    let credentials = ["KRAKEN_API_KEY_ID", "KRAKEN_API_SECRET"]
        .iter()
        .all(|name| std::env::var(name).is_ok_and(|value| !value.trim().is_empty()));
    assert_eq!(kraken.prerequisite.is_none(), credentials);
    assert_eq!(kraken.acquisition.is_some(), credentials);
    let binance = candidates
        .iter()
        .find(|candidate| candidate.source == "account:legacy:binance:spot")
        .context("a saved Binance account is listed")?;
    assert!(binance.prerequisite.is_some() && binance.acquisition.is_none());
    let nexo = candidates
        .iter()
        .find(|candidate| candidate.source == NEXO_SOURCE)
        .context("a configured but unreadable export stays visible and blocked")?;
    assert!(nexo.prerequisite.is_some() && nexo.acquisition.is_none());
    // Providers without a documented acquisition path are not offered as sources.
    assert!(
        candidates
            .iter()
            .all(|candidate| !candidate.source.contains("example"))
    );
    drop(app);
    Ok(())
}
