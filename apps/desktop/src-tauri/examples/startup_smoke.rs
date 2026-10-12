//! Headless desktop startup smoke test.
//!
//! Opens a temporary database through the real `Session` the Tauri host uses:
//! missing-database chooser state → explicit creation → empty snapshot → close.
//! It never touches a configured portfolio, provider credential or cached
//! response, so the three-system CI lanes can run it. The GUI window itself
//! still needs a desktop session on each system.
//!
//! usage: startup_smoke [DIRECTORY]
use anyhow::{Result, ensure};
use chrono::Utc;
use lib_app::{
    config::{DATABASE_FILENAME, ResolvedConfig},
    portfolio::Selection,
    refresh::RefreshTrigger,
    startup::OpenAction,
};
use money_tracker_desktop::{Session, Startup};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    ensure!(args.len() <= 1, "usage: startup_smoke [DIRECTORY]");
    let base = match args.first() {
        Some(directory) => PathBuf::from(directory),
        None => std::env::temp_dir().join(format!("money-tracker-smoke-{}", std::process::id())),
    };
    let mut resolved = ResolvedConfig::system(None, None)?;
    // Redirect every local path: the smoke test never reads or writes the
    // developer's or runner's configured database, config or cache.
    resolved.database = base.join(DATABASE_FILENAME);
    resolved.config_file = base.join("config.json");
    resolved.cache_dir = base.join("cache");
    resolved.config.data_dir = None;
    resolved.config.active_portfolio = None;
    resolved.config.nexo_csv_path = None;
    resolved.legacy_database = None;

    let mut session = Session::new(resolved);
    ensure!(
        matches!(session.startup().await?, Startup::Missing { .. }),
        "a fresh location offers the explicit chooser instead of creating a database"
    );
    let Startup::Ready { data, settings } = session.open(OpenAction::StartNew).await? else {
        anyhow::bail!("explicit creation did not open a database");
    };
    ensure!(data.holding_count == 0, "a new database starts empty");
    ensure!(
        data.refresh.is_empty(),
        "an empty portfolio configures no refresh source"
    );
    // With no configured source the scheduler plans nothing and contacts nothing.
    ensure!(
        session
            .refresh_plan(RefreshTrigger::Scheduled, Utc::now())
            .await?
            .is_idle(),
        "an empty portfolio must not schedule provider work"
    );
    ensure!(
        session
            .tree(data.snapshot_id, Selection::Overview)?
            .nodes
            .is_empty(),
        "an empty snapshot has no allocations"
    );
    session.close().await?;
    std::fs::remove_dir_all(&base).ok();
    println!(
        "Desktop startup smoke passed: missing → new → empty snapshot → close (contract {}, reporting {})",
        data.contract_version, settings.reporting_currency.0
    );
    Ok(())
}
