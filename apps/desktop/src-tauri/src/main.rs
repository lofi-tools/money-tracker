#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use chrono::Utc;
use lib_app::{
    config::{DATABASE_FILENAME, ResolvedConfig, Settings},
    portfolio::Selection,
    refresh::RefreshTrigger,
    startup::OpenAction,
};
use money_tracker_desktop::{NodeDetails, RefreshGate, RefreshUpdate, Session, Startup};
use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};
use tauri::{Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;
use tokio::sync::Mutex;
/// Scheduled refresh while the app runs; an idle tick does no provider work.
const SCHEDULER_TICK: std::time::Duration = std::time::Duration::from_secs(60);
const REFRESH_EVENT: &str = "refresh";
struct Host {
    session: Mutex<Option<Session>>,
    exiting: AtomicBool,
    closed: AtomicBool,
    /// One in-flight refresh; a trigger arriving during one coalesces into it.
    refresh: RefreshGate,
    /// Set only by the automated launch acceptance described below.
    smoke: Option<LaunchSmoke>,
    /// Evidence that the embedded frontend loaded and reached the host.
    frontend_ready: AtomicBool,
}
/// Automated launch acceptance for the real windowed host.
///
/// `MONEY_TRACKER_LAUNCH_SMOKE=<directory>` points the configuration file,
/// database and cache at a throwaway directory, records what the launch
/// achieved in `<directory>/launch.json` once the window exists, then exits;
/// `MONEY_TRACKER_LAUNCH_SMOKE_ACTION=create` additionally performs the
/// explicit “start new” the missing-database chooser offers. Nothing in normal
/// use sets these, so an ordinary launch is unchanged.
#[derive(Clone)]
struct LaunchSmoke {
    base: PathBuf,
    create: bool,
}
impl LaunchSmoke {
    fn from_env() -> Option<Self> {
        let base = std::env::var_os("MONEY_TRACKER_LAUNCH_SMOKE").map(PathBuf::from)?;
        Some(Self {
            create: std::env::var("MONEY_TRACKER_LAUNCH_SMOKE_ACTION").as_deref() == Ok("create"),
            base,
        })
    }
    fn report(&self) -> PathBuf {
        self.base.join("launch.json")
    }
    fn database(&self) -> PathBuf {
        self.base.join(DATABASE_FILENAME)
    }
}
/// Local paths for this process; the launch acceptance never touches a
/// configured portfolio, provider credential or cached response.
fn local_config(state: &Host) -> anyhow::Result<ResolvedConfig> {
    let mut resolved = ResolvedConfig::system(None, None)?;
    if let Some(smoke) = &state.smoke {
        resolved.database = smoke.database();
        resolved.config_file = smoke.base.join("config.json");
        resolved.cache_dir = smoke.base.join("cache");
        resolved.config.data_dir = None;
        resolved.config.active_portfolio = None;
        resolved.config.nexo_csv_path = None;
        resolved.legacy_database = None;
    }
    Ok(resolved)
}
// Errors never forward database/provider diagnostics that might contain private amounts.
fn safe<T>(r: anyhow::Result<T>) -> Result<T, String> {
    r.map_err(|_|"Unable to complete this local operation. Check the configuration, close other database writers, and retry.".into())
}
async fn session(state: &Host) -> Result<tokio::sync::MutexGuard<'_, Option<Session>>, String> {
    if state.exiting.load(Ordering::SeqCst) {
        return Err("Application is closing.".into());
    }
    let mut guard = state.session.lock().await;
    if guard.is_none() {
        *guard = Some(Session::new(safe(local_config(state))?));
    }
    Ok(guard)
}
/// One refresh pass: plan briefly, acquire without holding the session lock, then
/// commit. Returns `None` when an equivalent pass is already running, so launch,
/// catch-up and scheduled triggers coalesce instead of stacking provider work.
async fn run_refresh(
    host: &Host,
    trigger: RefreshTrigger,
) -> Result<Option<RefreshUpdate>, String> {
    if host.exiting.load(Ordering::SeqCst) {
        return Ok(None);
    }
    match host.refresh.run(refresh_pass(host, trigger)).await {
        Some(pass) => pass,
        None => Ok(None),
    }
}
/// The body of one refresh pass while the gate guarantees it is the only one.
async fn refresh_pass(
    host: &Host,
    trigger: RefreshTrigger,
) -> Result<Option<RefreshUpdate>, String> {
    let now = Utc::now();
    let plan = {
        let mut guard = host.session.lock().await;
        let Some(session) = guard.as_mut() else {
            return Ok(None);
        };
        safe(session.refresh_plan(trigger, now).await)?
    };
    let mut outcomes = Vec::new();
    for item in plan.runs() {
        if let Some(outcome) = lib_app::refresh::acquire(item).await {
            outcomes.push(outcome);
        }
    }
    if host.exiting.load(Ordering::SeqCst) {
        return Ok(None);
    }
    let (startup, report) = {
        let mut guard = host.session.lock().await;
        let Some(session) = guard.as_mut() else {
            return Ok(None);
        };
        safe(session.refresh_commit(plan, outcomes, now).await)?
    };
    Ok(Some(RefreshUpdate { startup, report }))
}
/// Refresh after opening or installing a database. Launch work contacts nothing
/// unless refresh-on-open is enabled or a scheduled interval was missed; it still
/// starts the interval for sources that were never scheduled.
async fn launch_refresh(app: tauri::AppHandle) {
    let Ok(Some(update)) = run_refresh(&app.state::<Host>(), RefreshTrigger::Launch).await else {
        return;
    };
    if update.report.changed() {
        let _ = app.emit(REFRESH_EVENT, &update);
    }
}
fn spawn_launch_refresh(app: &tauri::AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(launch_refresh(app));
}
async fn scheduler(app: tauri::AppHandle) {
    loop {
        tokio::time::sleep(SCHEDULER_TICK).await;
        if app.state::<Host>().exiting.load(Ordering::SeqCst) {
            break;
        }
        let Ok(Some(update)) = run_refresh(&app.state::<Host>(), RefreshTrigger::Scheduled).await
        else {
            continue;
        };
        if update.report.changed() {
            let _ = app.emit(REFRESH_EVENT, &update);
        }
    }
}
#[tauri::command]
async fn startup(app: tauri::AppHandle, state: State<'_, Host>) -> Result<Startup, String> {
    // The frontend issues this on mount, so it is the host's evidence that the
    // bundled view actually loaded and can call Rust.
    state.frontend_ready.store(true, Ordering::SeqCst);
    let open = safe(session(&state).await?.as_mut().unwrap().startup().await)?;
    if matches!(open, Startup::Ready { .. }) {
        spawn_launch_refresh(&app);
    }
    Ok(open)
}
#[tauri::command]
async fn start_new(app: tauri::AppHandle, state: State<'_, Host>) -> Result<Startup, String> {
    let open = safe(
        session(&state)
            .await?
            .as_mut()
            .unwrap()
            .open(OpenAction::StartNew)
            .await,
    )?;
    spawn_launch_refresh(&app);
    Ok(open)
}
#[tauri::command]
async fn copy_database(
    app: tauri::AppHandle,
    state: State<'_, Host>,
) -> Result<Option<Startup>, String> {
    let dialog = app
        .dialog()
        .file()
        .set_title("Copy an existing Money Tracker database")
        .add_filter("Money Tracker database", &["duckdb"]);
    let selected = tauri::async_runtime::spawn_blocking(move || dialog.blocking_pick_file())
        .await
        .map_err(|_| "File selection failed.".to_string())?;
    let Some(selected) = selected else {
        return Ok(None);
    };
    let path = selected
        .into_path()
        .map_err(|_| "Choose a local database file.".to_string())?;
    let open = safe(
        session(&state)
            .await?
            .as_mut()
            .unwrap()
            .open(OpenAction::CopyExisting(path))
            .await,
    )?;
    spawn_launch_refresh(&app);
    Ok(Some(open))
}
#[tauri::command]
async fn allocation(
    state: State<'_, Host>,
    snapshot_id: u64,
    selection: Selection,
) -> Result<money_tracker_desktop::DesktopTree, String> {
    safe(
        session(&state)
            .await?
            .as_ref()
            .unwrap()
            .tree(snapshot_id, selection),
    )
}
#[tauri::command]
async fn details(
    state: State<'_, Host>,
    snapshot_id: u64,
    selection: Selection,
    node_id: String,
) -> Result<NodeDetails, String> {
    safe(
        session(&state)
            .await?
            .as_ref()
            .unwrap()
            .details(snapshot_id, selection, &node_id),
    )
}
#[tauri::command]
async fn refresh(
    state: State<'_, Host>,
    trigger: Option<RefreshTrigger>,
) -> Result<Option<RefreshUpdate>, String> {
    run_refresh(&state, trigger.unwrap_or(RefreshTrigger::Manual)).await
}
#[tauri::command]
async fn manual_edit(
    state: State<'_, Host>,
    edit: lib_app::manual::ManualEdit,
) -> Result<Startup, String> {
    safe(
        session(&state)
            .await?
            .as_mut()
            .unwrap()
            .apply_edit(edit)
            .await,
    )
}
#[tauri::command]
async fn undo_action(state: State<'_, Host>, id: String) -> Result<Startup, String> {
    safe(
        session(&state)
            .await?
            .as_mut()
            .unwrap()
            .undo_action(id)
            .await,
    )
}
#[tauri::command]
async fn save_settings(state: State<'_, Host>, settings: Settings) -> Result<Startup, String> {
    safe(
        session(&state)
            .await?
            .as_mut()
            .unwrap()
            .save_settings(settings)
            .await,
    )
}
/// The one shutdown path: refuse new work, close the database and end the
/// process. Only the launch acceptance exits with a non-zero status.
async fn shutdown_with(app: &tauri::AppHandle, status: i32) {
    let host = app.state::<Host>();
    if host.exiting.swap(true, Ordering::SeqCst) {
        return;
    }
    if let Some(s) = host.session.lock().await.as_mut() {
        let _ = s.close().await;
    }
    host.closed.store(true, Ordering::SeqCst);
    app.exit(status);
}
async fn shutdown(app: tauri::AppHandle) {
    shutdown_with(&app, 0).await
}
#[tauri::command]
async fn quit(app: tauri::AppHandle) {
    shutdown(app).await
}
/// One startup state as machine-readable evidence; no amounts are included.
fn startup_evidence(startup: &Startup) -> serde_json::Value {
    match startup {
        Startup::Missing { database, settings } => serde_json::json!({
            "status": "missing",
            "destination": database.destination,
            "legacy_database": database.legacy_database,
            "reporting_currency": settings.reporting_currency.0,
        }),
        Startup::Ready { data, settings } => serde_json::json!({
            "status": "ready",
            "contract_version": data.contract_version,
            "snapshot_id": data.snapshot_id,
            "holding_count": data.holding_count,
            "dust_count": data.dust_count,
            "refresh_sources": data.refresh.len(),
            "reporting_currency": settings.reporting_currency.0,
            "theme": settings.theme,
            "privacy_mode": settings.privacy_mode,
            "privacy_scale": settings.privacy_scale.as_str(),
            "dust_threshold": settings.dust_threshold.as_str(),
        }),
    }
}
/// Records what a real windowed launch achieved, then closes down. A missing
/// window or a failed startup state exits non-zero, so the caller asserts on
/// both the report and the process status.
async fn report_launch(app: tauri::AppHandle, report: PathBuf) {
    let window = app.get_webview_window("main").map(|window| {
        let size = window.inner_size().ok();
        serde_json::json!({
            "label": window.label(),
            "title": window.title().unwrap_or_default(),
            "created": true,
            "visible": window.is_visible().unwrap_or(false),
            "width": size.map(|s| s.width),
            "height": size.map(|s| s.height),
        })
    });
    let state = app.state::<Host>();
    let mut initial = None;
    let mut created = None;
    let mut failure: Option<String> = None;
    match session(&state).await {
        Ok(mut guard) => {
            let session = guard.as_mut().expect("session created above");
            match session.startup().await {
                Ok(startup) => {
                    let create_when_missing = state.smoke.as_ref().is_some_and(|s| s.create);
                    initial = Some(startup_evidence(&startup));
                    if create_when_missing && matches!(startup, Startup::Missing { .. }) {
                        match session.open(OpenAction::StartNew).await {
                            Ok(startup) => created = Some(startup_evidence(&startup)),
                            Err(e) => failure = Some(format!("explicit creation failed: {e}")),
                        }
                    }
                }
                Err(e) => failure = Some(format!("startup failed: {e}")),
            }
        }
        Err(e) => failure = Some(e),
    }
    // The embedded bundle signals a loaded view by invoking `startup`; wait
    // briefly for that instead of guessing at the webview's state.
    let mut frontend_ready = false;
    for _ in 0..200 {
        if state.frontend_ready.load(Ordering::SeqCst) {
            frontend_ready = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    let failure =
        failure.or_else(|| (!frontend_ready).then(|| "the frontend never reached the host".into()));
    let database = state.smoke.as_ref().map(LaunchSmoke::database);
    let ok = window.is_some() && failure.is_none();
    let evidence = serde_json::json!({
        "ok": ok,
        "exit_code": if ok { 0 } else { 1 },
        "window": window,
        "frontend_ready": frontend_ready,
        "database": {
            "path": database,
            "exists": database.as_ref().is_some_and(|p| p.is_file()),
        },
        "initial": initial,
        "created": created,
        "failure": failure,
    });
    let _ = std::fs::create_dir_all(report.parent().unwrap_or(Path::new(".")));
    if let Ok(bytes) = serde_json::to_vec_pretty(&evidence) {
        let _ = std::fs::write(&report, bytes);
    }
    shutdown_with(&app, if ok { 0 } else { 1 }).await;
}
fn main() {
    let smoke = LaunchSmoke::from_env();
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(Host {
            session: Mutex::new(None),
            exiting: AtomicBool::new(false),
            closed: AtomicBool::new(false),
            refresh: RefreshGate::new(),
            smoke: smoke.clone(),
            frontend_ready: AtomicBool::new(false),
        })
        .invoke_handler(tauri::generate_handler![
            startup,
            start_new,
            copy_database,
            allocation,
            details,
            save_settings,
            refresh,
            manual_edit,
            undo_action,
            quit
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let app = window.app_handle().clone();
                tauri::async_runtime::spawn(shutdown(app));
            }
        })
        .build(tauri::generate_context!())
        .expect("building desktop host");
    // Scheduled daily refresh while the app runs; nothing survives window close.
    tauri::async_runtime::spawn(scheduler(app.handle().clone()));
    let report = smoke.map(|smoke| smoke.report());
    app.run(move |app, event| match event {
        // The launch acceptance waits for the real window before measuring.
        tauri::RunEvent::Ready if report.is_some() => {
            let app = app.clone();
            let report = report.clone().expect("report path checked above");
            tauri::async_runtime::spawn(async move { report_launch(app, report).await });
        }
        tauri::RunEvent::ExitRequested { api, .. }
            if !app.state::<Host>().closed.load(Ordering::SeqCst) =>
        {
            api.prevent_exit();
            tauri::async_runtime::spawn(shutdown(app.clone()));
        }
        _ => {}
    });
}
