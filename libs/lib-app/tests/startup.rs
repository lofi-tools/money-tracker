use anyhow::Result;
use lib_app::{
    Application,
    config::*,
    startup::OpenAction,
    worker::{DatabaseWorker, StartupOutcome},
};
use lib_core::{Account, AssetId, DEFAULT_PORTFOLIO, Portfolio, Store};
use tempfile::TempDir;

fn locations(dir: &TempDir, windows: bool) -> Locations {
    Locations::for_platform(
        &dir.path().join("home"),
        &dir.path().join("config"),
        &dir.path().join("local"),
        &dir.path().join("old"),
        windows,
    )
}
fn resolve(dir: &TempDir) -> Result<ResolvedConfig> {
    ResolvedConfig::resolve(locations(dir, false), None, None, None)
}
async fn ready(resolved: ResolvedConfig, action: OpenAction) -> Result<DatabaseWorker> {
    match DatabaseWorker::start(resolved, action).await? {
        StartupOutcome::Ready(worker) => Ok(worker),
        StartupOutcome::Missing(_) => anyhow::bail!("expected ready database"),
    }
}

#[test]
fn paths_config_precedence_and_native_defaults() -> Result<()> {
    let dir = TempDir::new()?;
    let loc = locations(&dir, false);
    assert_eq!(
        loc.default_data_dir,
        dir.path().join("home/.data/money-tracker")
    );
    assert_eq!(
        loc.config_file,
        dir.path().join("config/money-tracker/config.json")
    );
    assert_eq!(
        locations(&dir, true).default_data_dir,
        dir.path().join("local/money-tracker")
    );
    let config = AppConfig {
        data_dir: Some(dir.path().join("configured")),
        ..Default::default()
    };
    config.save(&loc.config_file)?;
    let resolved = ResolvedConfig::resolve(loc.clone(), None, None, None)?;
    assert_eq!(
        resolved.database,
        dir.path().join("configured/portfolio.duckdb")
    );
    let resolved = ResolvedConfig::resolve(loc.clone(), None, None, Some(dir.path().join("env")))?;
    assert_eq!(resolved.database, dir.path().join("env/portfolio.duckdb"));
    let resolved = ResolvedConfig::resolve(
        loc.clone(),
        None,
        Some(dir.path().join("flag")),
        Some(dir.path().join("env")),
    )?;
    assert_eq!(resolved.database, dir.path().join("flag/portfolio.duckdb"));
    assert!(
        ResolvedConfig::resolve(
            loc.clone(),
            Some(dir.path().join("missing.json")),
            None,
            None
        )
        .is_err()
    );
    assert!(ResolvedConfig::resolve(loc.clone(), None, Some("".into()), None).is_err());
    std::fs::write(&loc.config_file, r#"{"data_dr":"typo"}"#)?;
    assert!(ResolvedConfig::resolve(loc, None, None, None).is_err());
    Ok(())
}
#[test]
fn settings_exact_validation_and_persistence() -> Result<()> {
    let dir = TempDir::new()?;
    let path = dir.path().join("settings.json");
    let config = AppConfig::default();
    assert_eq!(config.settings.privacy_scale.as_str(), "0.1");
    assert_eq!(config.settings.dust_threshold.as_str(), "1");
    assert_eq!(config.settings.refresh_interval_seconds, 86400);
    assert!(!config.settings.refresh_on_open);
    config.save(&path)?;
    assert_eq!(AppConfig::load(&path, true)?, config);
    for scale in ["0", "-0.1", "1", "1.1"] {
        let mut invalid = config.clone();
        invalid.settings.privacy_scale = scale.parse()?;
        assert!(invalid.save(&path).is_err());
        assert_eq!(AppConfig::load(&path, true)?, config);
    }
    let mut valid = config.clone();
    valid.settings.privacy_scale = "0.00000000000000000000000000000001".parse()?;
    valid.settings.dust_threshold = "12345678901234567890.1234567890123456789".parse()?;
    valid.save(&path)?;
    assert_eq!(AppConfig::load(&path, true)?, valid);
    for text in [
        r#"{"version":2}"#,
        r#"{"data_dir":""}"#,
        r#"{"settings":{"refresh_interval_seconds":0}}"#,
        r#"{"settings":{"dust_threshold":"-1"}}"#,
        r#"{"settings":{"privacy_scale":0.1}}"#,
        r#"{"active_portfolio":" "}"#,
    ] {
        std::fs::write(&path, text)?;
        assert!(AppConfig::load(&path, true).is_err());
    }
    Ok(())
}

#[tokio::test]
async fn missing_explains_legacy_without_creating_or_fetching() -> Result<()> {
    let dir = TempDir::new()?;
    let resolved = resolve(&dir)?;
    let legacy = resolved.legacy_database.clone().unwrap();
    std::fs::create_dir_all(legacy.parent().unwrap())?;
    drop(Application::open(&legacy)?);
    let bytes = std::fs::read(&legacy)?;
    match DatabaseWorker::start(resolved.clone(), OpenAction::OpenExisting).await? {
        StartupOutcome::Missing(missing) => {
            assert_eq!(missing.destination, resolved.database);
            assert_eq!(missing.legacy_database, Some(legacy.clone()));
        }
        _ => panic!("expected explicit onboarding"),
    }
    assert!(!resolved.database.try_exists()?);
    assert_eq!(std::fs::read(&legacy)?, bytes);
    let worker = ready(resolved.clone(), OpenAction::StartNew).await?;
    assert_eq!(worker.portfolio_id().0, DEFAULT_PORTFOLIO);
    let ledger = worker.ledger_balances(chrono::Utc::now()).await?;
    assert!(ledger.accounts.is_empty() && ledger.positions.is_empty());
    worker.close().await?;
    assert!(worker.settings().await.is_err());
    ready(resolved, OpenAction::OpenExisting)
        .await?
        .close()
        .await?;
    Ok(())
}

#[tokio::test]
async fn copy_preserves_source_and_refuses_overwrite_or_invalid_files() -> Result<()> {
    let dir = TempDir::new()?;
    let source = dir.path().join("source.duckdb");
    let store = Store::open(&source)?;
    store.save_account(&Account {
        id: "saved".into(),
        portfolio_id: DEFAULT_PORTFOLIO.into(),
        provider_id: "nexo".into(),
        label: "existing".into(),
        source_reference: Some("original".into()),
    })?;
    // Read-only copy validation refuses the same-process active writer too.
    assert!(
        DatabaseWorker::start(resolve(&dir)?, OpenAction::CopyExisting(source.clone()))
            .await
            .is_err()
    );
    drop(store);
    let bytes = std::fs::read(&source)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&source, std::fs::Permissions::from_mode(0o400))?;
    }
    let worker = ready(resolve(&dir)?, OpenAction::CopyExisting(source.clone())).await?;
    assert_eq!(
        worker.ledger_balances(chrono::Utc::now()).await?.accounts[0]
            .source_reference
            .as_deref(),
        Some("original")
    );
    worker.close().await?;
    assert_eq!(std::fs::read(&source)?, bytes);
    let dest = resolve(&dir)?.database;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&source)?.permissions().mode() & 0o777,
            0o400
        );
        assert_eq!(
            std::fs::metadata(&dest)?.permissions().mode() & 0o777,
            0o600
        );
    }
    let dest_bytes = std::fs::read(&dest)?;
    for action in [
        OpenAction::StartNew,
        OpenAction::CopyExisting(source.clone()),
    ] {
        assert!(DatabaseWorker::start(resolve(&dir)?, action).await.is_err());
        assert_eq!(std::fs::read(&dest)?, dest_bytes);
    }
    let invalid_dir = TempDir::new()?;
    let bad = invalid_dir.path().join("invalid");
    std::fs::write(&bad, b"not a DuckDB")?;
    assert!(
        DatabaseWorker::start(resolve(&invalid_dir)?, OpenAction::CopyExisting(bad))
            .await
            .is_err()
    );
    assert!(!resolve(&invalid_dir)?.database.try_exists()?);
    let empty = invalid_dir.path().join("empty");
    std::fs::write(&empty, b"")?;
    assert!(
        DatabaseWorker::start(resolve(&invalid_dir)?, OpenAction::CopyExisting(empty))
            .await
            .is_err()
    );
    let other = invalid_dir.path().join("other.duckdb");
    drop(duckdb::Connection::open(&other)?);
    assert!(
        DatabaseWorker::start(resolve(&invalid_dir)?, OpenAction::CopyExisting(other))
            .await
            .is_err()
    );
    assert!(!resolve(&invalid_dir)?.database.try_exists()?);
    Ok(())
}

#[tokio::test]
async fn wal_corrupt_database_and_portfolio_errors_do_not_install_empty_replacements() -> Result<()>
{
    let dir = TempDir::new()?;
    let resolved = resolve(&dir)?;
    std::fs::create_dir_all(resolved.database.parent().unwrap())?;
    std::fs::write(&resolved.database, b"bad")?;
    assert!(
        DatabaseWorker::start(resolved.clone(), OpenAction::OpenExisting)
            .await
            .is_err()
    );
    assert_eq!(std::fs::read(&resolved.database)?, b"bad");
    std::fs::remove_file(&resolved.database)?;
    let source = dir.path().join("source.duckdb");
    drop(Store::open(&source)?);
    let wal = dir.path().join("source.duckdb.wal");
    std::fs::write(&wal, b"pending")?;
    assert!(
        DatabaseWorker::start(resolved.clone(), OpenAction::CopyExisting(source.clone()))
            .await
            .is_err()
    );
    assert!(!resolved.database.try_exists()?);
    std::fs::remove_file(wal)?;
    let store = Store::open(&source)?;
    store.save_portfolio(&Portfolio {
        id: "family".into(),
        label: "family".into(),
        reporting_asset: AssetId::str("EUR"),
    })?;
    drop(store);
    let bytes = std::fs::read(&source)?;
    assert!(
        DatabaseWorker::start(resolved.clone(), OpenAction::CopyExisting(source.clone()))
            .await
            .is_err()
    );
    assert!(!resolved.database.try_exists()?);
    assert_eq!(std::fs::read(&source)?, bytes);
    let mut scoped = resolved.clone();
    scoped.config.active_portfolio = Some("family".into());
    let worker = ready(scoped.clone(), OpenAction::CopyExisting(source)).await?;
    assert_eq!(worker.portfolio_id().0, "family");
    assert_eq!(
        worker
            .ledger_balances(chrono::Utc::now())
            .await?
            .context
            .portfolio_id
            .0,
        "family"
    );
    worker.close().await?;
    scoped.config.active_portfolio = Some("missing".into());
    assert!(
        DatabaseWorker::start(scoped, OpenAction::OpenExisting)
            .await
            .is_err()
    );
    // An invalid configured ID cannot install a fresh empty database either.
    let fresh = TempDir::new()?;
    let mut invalid = resolve(&fresh)?;
    invalid.config.active_portfolio = Some("missing".into());
    assert!(
        DatabaseWorker::start(invalid.clone(), OpenAction::StartNew)
            .await
            .is_err()
    );
    assert!(!invalid.database.try_exists()?);
    Ok(())
}

#[tokio::test]
async fn worker_settings_survive_restart_and_concurrent_reads_are_serialized() -> Result<()> {
    let dir = TempDir::new()?;
    let resolved = resolve(&dir)?;
    let worker = ready(resolved, OpenAction::StartNew).await?;
    let mut settings = worker.settings().await?;
    settings.theme = Theme::Dark;
    settings.privacy_mode = PrivacyMode::Scaled;
    settings.privacy_scale = "0.02".parse()?;
    settings.reporting_currency = AssetId::str("EUR");
    settings.refresh_on_open = true;
    worker.save_settings(settings.clone()).await?;
    let (a, b) = tokio::join!(
        worker.ledger_balances(chrono::Utc::now()),
        worker.settings()
    );
    assert!(a?.positions.is_empty());
    assert_eq!(b?, settings);
    let mut invalid = settings.clone();
    invalid.refresh_interval_seconds = 0;
    assert!(worker.save_settings(invalid).await.is_err());
    assert_eq!(worker.settings().await?, settings);
    let (a, b) = tokio::join!(worker.close(), worker.close());
    a?;
    b?;
    let worker = ready(resolve(&dir)?, OpenAction::OpenExisting).await?;
    assert_eq!(worker.settings().await?, settings);
    worker.close().await?;
    Ok(())
}

#[tokio::test]
async fn copied_legacy_ledger_migrates_idempotently_without_touching_source() -> Result<()> {
    let dir = TempDir::new()?;
    let source = dir.path().join("legacy.duckdb");
    {
        let connection = duckdb::Connection::open(&source)?;
        connection.execute_batch("CREATE TABLE positions (position_id VARCHAR PRIMARY KEY, asset_id VARCHAR NOT NULL, owned BOOLEAN NOT NULL);
            CREATE TABLE transaction_effects (transaction_id VARCHAR NOT NULL, position_id VARCHAR NOT NULL, amount BIGINT NOT NULL, effect_order INTEGER NOT NULL);
            CREATE TABLE transactions (id VARCHAR PRIMARY KEY, datetime TIMESTAMP NOT NULL);
            CREATE TABLE asset_scales (asset_id VARCHAR PRIMARY KEY, decimals INTEGER NOT NULL);
            CREATE TABLE imported_files (source VARCHAR NOT NULL, content_hash VARCHAR NOT NULL, transaction_count BIGINT NOT NULL, completed_at TIMESTAMP NOT NULL, PRIMARY KEY(source, content_hash));
            INSERT INTO asset_scales VALUES ('USD',2);
            INSERT INTO imported_files VALUES ('NEXO','nexo-hash',1,'2025-01-01'), ('BINANCE','binance-hash',1,'2025-01-01');
            INSERT INTO positions VALUES ('NEXO:USD','USD',true), ('NEXO-CREDIT:USD','USD',false), ('BINANCE:spot:USD','USD',true), ('BINANCE:external:fees:USD','USD',false);
            INSERT INTO transactions VALUES ('NEXO:1','2025-01-01'), ('BINANCE:1','2025-01-01');
            INSERT INTO transaction_effects VALUES ('NEXO:1','NEXO:USD',9223372036854775807,0), ('NEXO:1','NEXO-CREDIT:USD',-2000,1), ('BINANCE:1','BINANCE:spot:USD',9223372036854775807,0), ('BINANCE:1','BINANCE:external:fees:USD',-100,1);")?;
    }
    let bytes = std::fs::read(&source)?;
    let resolved = resolve(&dir)?;
    let worker = ready(resolved.clone(), OpenAction::CopyExisting(source.clone())).await?;
    let expected = worker.ledger_balances(chrono::Utc::now()).await?.positions;
    assert_eq!(expected.len(), 3);
    assert_eq!(
        expected
            .iter()
            .find(|p| p.position_id.0 == "NEXO-CREDIT:USD")
            .unwrap()
            .quantity
            .units
            .to_string(),
        "-2000"
    );
    assert_eq!(
        expected
            .iter()
            .find(|p| p.position_id.0 == "BINANCE:spot:USD")
            .unwrap()
            .quantity
            .units
            .to_string(),
        "9223372036854775807"
    );
    worker.close().await?;
    for _ in 0..2 {
        let worker = ready(resolved.clone(), OpenAction::OpenExisting).await?;
        assert_eq!(
            worker.ledger_balances(chrono::Utc::now()).await?.positions,
            expected
        );
        worker.close().await?;
        let store = Store::open(&resolved.database)?;
        assert!(store.has_imported_file("NEXO", "nexo-hash")?);
        assert!(store.has_imported_file("BINANCE", "binance-hash")?);
    }
    assert_eq!(std::fs::read(source)?, bytes);
    Ok(())
}

#[tokio::test]
async fn metadata_and_local_actions_use_the_selected_worker_portfolio_after_restart() -> Result<()>
{
    use lib_core::ui_metadata::*;
    let dir = TempDir::new()?;
    let resolved = resolve(&dir)?;
    std::fs::create_dir_all(resolved.database.parent().unwrap())?;
    {
        let store = Store::open(&resolved.database)?;
        store.save_account(&Account {
            id: "visible".into(),
            portfolio_id: DEFAULT_PORTFOLIO.into(),
            provider_id: "nexo".into(),
            label: "Provider name".into(),
            source_reference: Some("provider-reference".into()),
        })?;
    }
    let worker = ready(resolved.clone(), OpenAction::OpenExisting).await?;
    let target = LocalTarget::Account {
        id: "visible".into(),
    };
    let action = worker
        .apply_manual_change(
            target.clone(),
            0,
            Some(LocalValue::Account(AccountCustomization {
                local_label: Some("Local name".into()),
                ..Default::default()
            })),
        )
        .await?;
    let metadata = worker.ui_metadata().await?;
    assert_eq!(metadata.accounts[0].account.label, "Provider name");
    assert_eq!(metadata.local_state[0].1, action.after);
    assert!(metadata.subscriptions.is_empty() && metadata.products.is_empty());
    worker.close().await?;
    let worker = ready(resolved, OpenAction::OpenExisting).await?;
    assert_eq!(worker.manual_actions().await?, vec![action.clone()]);
    let undo = worker.undo_manual_action(action.id).await?;
    assert_eq!(worker.ui_metadata().await?.local_state[0].1, undo.after);
    worker.close().await?;
    Ok(())
}

#[tokio::test]
async fn portfolio_projection_uses_worker_settings_and_selected_portfolio() -> Result<()> {
    use lib_app::{dto::FigureDto, exact::ExactRatio};
    let dir = TempDir::new()?;
    let resolved = resolve(&dir)?;
    let worker = ready(resolved, OpenAction::StartNew).await?;
    let projection = worker
        .portfolio_projection("2026-01-20T00:00:00Z".parse()?)
        .await?;
    assert_eq!(projection.request.reporting_asset, AssetId::str("USD"));
    assert_eq!(projection.request.dust_threshold.as_str(), "1");
    assert_eq!(projection.request.context.portfolio_id.0, DEFAULT_PORTFOLIO);
    assert!(
        matches!(projection.overview.current_value,FigureDto::Known{value,..} if value.amount==ExactRatio::zero())
    );
    let mut settings = worker.settings().await?;
    settings.reporting_currency = AssetId::str("EUR");
    settings.dust_threshold = "2".parse()?;
    worker.save_settings(settings).await?;
    let projection = worker
        .portfolio_projection("2026-01-20T00:00:00Z".parse()?)
        .await?;
    assert_eq!(projection.request.reporting_asset, AssetId::str("EUR"));
    assert_eq!(projection.request.dust_threshold.as_str(), "2");
    worker.close().await?;
    Ok(())
}
