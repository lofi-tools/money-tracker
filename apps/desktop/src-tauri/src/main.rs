#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use lib_app::{
    config::{ResolvedConfig, Settings},
    portfolio::Selection,
    startup::OpenAction,
};
use money_tracker_desktop::{NodeDetails, Session, Startup};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{Manager, State};
use tauri_plugin_dialog::DialogExt;
use tokio::sync::Mutex;
struct Host {
    session: Mutex<Option<Session>>,
    exiting: AtomicBool,
    closed: AtomicBool,
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
#[tauri::command]
async fn startup(state: State<'_, Host>) -> Result<Startup, String> {
    safe(session(&state).await?.as_mut().unwrap().startup().await)
}
#[tauri::command]
async fn start_new(state: State<'_, Host>) -> Result<Startup, String> {
    safe(
        session(&state)
            .await?
            .as_mut()
            .unwrap()
            .open(OpenAction::StartNew)
            .await,
    )
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
    safe(
        session(&state)
            .await?
            .as_mut()
            .unwrap()
            .open(OpenAction::CopyExisting(path))
            .await,
    )
    .map(Some)
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
        })
        .invoke_handler(tauri::generate_handler![
            startup,
            start_new,
            copy_database,
            allocation,
            details,
            save_settings,
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
    app.run(|app, event| {
        if let tauri::RunEvent::ExitRequested { api, .. } = event {
            if !app.state::<Host>().closed.load(Ordering::SeqCst) {
                api.prevent_exit();
                tauri::async_runtime::spawn(shutdown(app.clone()));
            }
        }
    });
}
