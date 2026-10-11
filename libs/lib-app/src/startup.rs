//! Explicit, offline database startup and source-preserving installation.
use crate::{Application, config::ResolvedConfig};
use anyhow::{Context, Result, ensure};
use duckdb::{AccessMode, Config, Connection};
use lib_core::{DEFAULT_PORTFOLIO, PortfolioId};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub enum OpenAction {
    OpenExisting,
    StartNew,
    CopyExisting(PathBuf),
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MissingDatabase {
    pub destination: PathBuf,
    pub legacy_database: Option<PathBuf>,
}

pub(crate) enum Opened {
    Missing(MissingDatabase),
    Ready(Application, PortfolioId),
}

fn select_portfolio(app: &Application, selected: Option<&PortfolioId>) -> Result<PortfolioId> {
    let portfolios = app.store().portfolios()?;
    if let Some(id) = selected {
        ensure!(
            portfolios.iter().any(|p| &p.id == id),
            "configured portfolio {} is not in the database",
            id.0
        );
        return Ok(id.clone());
    }
    ensure!(
        portfolios.len() == 1 && portfolios[0].id.0 == DEFAULT_PORTFOLIO,
        "database requires explicit active_portfolio configuration; portfolios are never merged"
    );
    Ok(DEFAULT_PORTFOLIO.into())
}

pub(crate) fn open(resolved: &ResolvedConfig, action: OpenAction) -> Result<Opened> {
    let path = &resolved.database;
    if matches!(action, OpenAction::OpenExisting) && !path.try_exists()? {
        let legacy_database = match &resolved.legacy_database {
            Some(p) if p.try_exists()? => Some(p.clone()),
            _ => None,
        };
        return Ok(Opened::Missing(MissingDatabase {
            destination: path.clone(),
            legacy_database,
        }));
    }
    if !matches!(action, OpenAction::OpenExisting) {
        ensure!(
            !path.try_exists()?,
            "refusing to overwrite existing database {}",
            path.display()
        );
        install(resolved, action)?;
    }
    ensure!(
        path.is_file(),
        "database is not a regular file: {}",
        path.display()
    );
    ensure!(
        std::fs::metadata(path)?.len() > 0,
        "database is empty or invalid: {}",
        path.display()
    );
    {
        let validation =
            Connection::open_with_flags(path, Config::default().access_mode(AccessMode::ReadOnly)?)
                .context(
                    "cannot open database read-only for validation; close other CLI/app processes",
                )?;
        validate_tracker_database(&validation)?;
    }
    let app = Application::open(path).with_context(|| {
        format!(
            "opening database {}; close other CLI/app processes if it is in use",
            path.display()
        )
    })?;
    let portfolio = select_portfolio(&app, resolved.config.active_portfolio.as_ref())?;
    Ok(Opened::Ready(app, portfolio))
}

fn install(resolved: &ResolvedConfig, action: OpenAction) -> Result<()> {
    let parent = resolved
        .database
        .parent()
        .context("database has no parent directory")?;
    std::fs::create_dir_all(parent)?;
    let temp = tempfile::NamedTempFile::new_in(parent)?;
    // DuckDB expects a missing file rather than a zero-byte placeholder.
    let temp_path = temp.into_temp_path();
    std::fs::remove_file(&temp_path)?;
    match action {
        OpenAction::CopyExisting(source) => {
            ensure!(
                source.is_file() && std::fs::metadata(&source)?.len() > 0,
                "copy source is not a valid database file"
            );
            let mut wal = source.as_os_str().to_owned();
            wal.push(".wal");
            ensure!(
                !Path::new(&wal).try_exists()?,
                "source has a WAL; close its writer cleanly before copying"
            );
            // Hold DuckDB's read-only lock throughout the physical copy. This also
            // refuses an active writer; never checkpoint or migrate the source.
            let source_lock = Connection::open_with_flags(
                &source,
                Config::default().access_mode(AccessMode::ReadOnly)?,
            )
            .context("cannot lock copy source read-only; close other CLI/app processes")?;
            validate_tracker_database(&source_lock)?;
            std::fs::copy(&source, &temp_path)?;
            // The installed copy belongs to this application, even when the
            // original backup was read-only. Never change source permissions.
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&temp_path, std::fs::Permissions::from_mode(0o600))?;
            }
            #[cfg(windows)]
            {
                let mut permissions = std::fs::metadata(&temp_path)?.permissions();
                permissions.set_readonly(false);
                std::fs::set_permissions(&temp_path, permissions)?;
            }
        }
        OpenAction::StartNew => {}
        OpenAction::OpenExisting => unreachable!(),
    }
    {
        let app = Application::open(&temp_path)
            .context("validating temporary database and migrations")?;
        select_portfolio(&app, resolved.config.active_portfolio.as_ref())?;
        // Drop/checkpoint before installing: the source and existing destination
        // remain untouched if migration or portfolio validation fails.
    }
    std::fs::File::open(&temp_path)?.sync_all()?;
    temp_path
        .persist_noclobber(&resolved.database)
        .map_err(|e| e.error)
        .context("installing database without overwriting an existing destination")?;
    Ok(())
}

fn validate_tracker_database(connection: &Connection) -> Result<()> {
    let count: i64 = connection.query_row("SELECT COUNT(*) FROM information_schema.tables WHERE table_schema='main' AND table_name IN ('transactions', 'positions', 'transaction_effects')", [], |r| r.get(0))?;
    ensure!(
        count == 3,
        "file is not a Money Tracker database (ledger tables missing)"
    );
    Ok(())
}
