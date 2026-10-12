#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use chrono::Utc;
use lib_app::{
    config::{ResolvedConfig, Settings},
    portfolio::Selection,
    refresh::RefreshTrigger,
    startup::OpenAction,
};
use money_tracker_desktop::{NodeDetails, RefreshGate, RefreshUpdate, Session, Startup};
use std::sync::atomic::{AtomicBool, Ordering};
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
        *guard = Some(Session::new(safe(ResolvedConfig::system(None, None))?));
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
async fn shutdown(app: tauri::AppHandle) {
    let host = app.state::<Host>();
    if host.exiting.swap(true, Ordering::SeqCst) {
        return;
    }
    if let Some(s) = host.session.lock().await.as_mut() {
        let _ = s.close().await;
    }
    host.closed.store(true, Ordering::SeqCst);
    app.exit(0);
}
#[tauri::command]
async fn quit(app: tauri::AppHandle) {
    shutdown(app).await
}
fn main() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(Host {
            session: Mutex::new(None),
            exiting: AtomicBool::new(false),
            closed: AtomicBool::new(false),
            refresh: RefreshGate::new(),
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
    app.run(|app, event| {
        if let tauri::RunEvent::ExitRequested { api, .. } = event {
            if !app.state::<Host>().closed.load(Ordering::SeqCst) {
                api.prevent_exit();
                tauri::async_runtime::spawn(shutdown(app.clone()));
            }
        }
    });
}
