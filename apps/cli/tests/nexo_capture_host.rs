use anyhow::Result;
use lib_app::{
    Application,
    capture_bridge::*,
    config::*,
    startup::OpenAction,
    worker::{DatabaseWorker, StartupOutcome},
};
use std::process::{Command, Stdio};
#[tokio::test]
async fn actual_native_process_delivers_json_to_existing_database_worker() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let csv = dir.path().join("nexo.csv");
    std::fs::write(
        &csv,
        concat!(
            "Transaction,Type,Input Currency,Input Amount,Output Currency,Output Amount,USD Equivalent,Fee,Fee Currency,Details,Date / Time (UTC)\n",
            "deposit,Top up Crypto,ETH,100,ETH,100,$1000,-,-,deposit,2026-01-01 00:00:00\n",
            "lock,Locking Term Deposit,ETH,-40,ETH,40,$400,-,-,lock,2026-01-02 00:00:00\n"
        ),
    )?;
    let db = dir.path().join("portfolio.duckdb");
    let app = Application::open(&db)?;
    lib_app::adapters::nexo::NexoSvc::from_path(csv).import_into(app.store())?;
    drop(app);
    let pair = Pairing {
        version: 1,
        connection_id: "a".repeat(64),
        extension_id: "b".repeat(32),
        database: db.clone(),
        portfolio: lib_core::DEFAULT_PORTFOLIO.into(),
        account: "legacy:nexo".into(),
        tracker_executable: None,
    };
    let inbox = root(&db)?;
    private_dir(&inbox)?;
    let config = inbox.join("pairing.json");
    atomic_private(&config, &serde_json::to_vec(&pair)?)?;
    let loc = Locations::for_platform(
        dir.path(),
        dir.path(),
        dir.path(),
        dir.path(),
        dir.path(),
        false,
    );
    let mut resolved = ResolvedConfig::resolve(loc, None, None, None)?;
    resolved.database = db.clone();
    let StartupOutcome::Ready(worker) =
        DatabaseWorker::start(resolved, OpenAction::OpenExisting).await?
    else {
        panic!("DB missing")
    };
    let mut child = Command::new(env!("CARGO_BIN_EXE_nexo-capture-host"))
        .args([
            "--pairing",
            config.to_str().unwrap(),
            &format!("chrome-extension://{}/", pair.extension_id),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;
    let envelope = Envelope {
        version: 1,
        connection_id: pair.connection_id,
        payload: include_str!("../../../libs/lib-app/tests/fixtures/nexo_capture.json").into(),
    };
    write_frame(child.stdin.as_mut().unwrap(), &envelope)?;
    let mut found = false;
    for _ in 0..100 {
        if !worker.ingest_browser_inbox().await?.is_empty() {
            found = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    assert!(found, "native host did not enqueue capture");
    let mut stdout = child.stdout.take().unwrap();
    let bytes = tokio::task::spawn_blocking(move || read_frame(&mut stdout))
        .await??
        .unwrap();
    let ack: Ack = serde_json::from_slice(&bytes)?;
    assert!(matches!(ack,Ack::Committed{receipt} if receipt.matched==1));
    drop(child.stdin.take());
    assert!(child.wait()?.success());
    let views = worker.nexo_capture_views().await?;
    assert_eq!(views.len(), 1);
    worker.close().await?;
    // New process replay uses the same durable commit, even after a missing acknowledgement.
    let mut replay = Command::new(env!("CARGO_BIN_EXE_nexo-capture-host"))
        .args([
            "--pairing",
            config.to_str().unwrap(),
            &format!("chrome-extension://{}/", pair.extension_id),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;
    write_frame(replay.stdin.as_mut().unwrap(), &envelope)?;
    let bytes = read_frame(replay.stdout.as_mut().unwrap())?.unwrap();
    assert!(matches!(
        serde_json::from_slice::<Ack>(&bytes)?,
        Ack::Committed { .. }
    ));
    drop(replay.stdin.take());
    assert!(replay.wait()?.success());
    Ok(())
}
