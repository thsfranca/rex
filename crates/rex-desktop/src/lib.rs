mod stream;

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use rex_cli::ensure_daemon_ready;
use rex_cli::CliError;
use stream::{StreamEventDto, submit_prompt_stream};
use tauri::ipc::Channel;
use tauri::{menu::{Menu, MenuItem, PredefinedMenuItem, Submenu}, Manager};

static SESSION_COUNTER: AtomicU64 = AtomicU64::new(0);

fn new_harness_session_id() -> String {
    let seq = SESSION_COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("hs-{}-{}-{}", std::process::id(), seq, nanos)
}

#[tauri::command]
async fn ensure_daemon() -> Result<(), String> {
    ensure_daemon_ready()
        .await
        .map_err(|e: CliError| e.operator_message())
}

#[tauri::command]
async fn submit_prompt(
    prompt: String,
    mode: String,
    on_event: Channel<StreamEventDto>,
) -> Result<(), String> {
    let session_id = new_harness_session_id();
    let trace_id = format!("web-{}", session_id);
    submit_prompt_stream(prompt, mode, trace_id, session_id, on_event)
        .await
        .map_err(|e: CliError| e.operator_message())
}

pub fn build_app() -> tauri::Builder<tauri::Wry> {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![ensure_daemon, submit_prompt])
        .setup(|app| {
            let app_menu = build_menu(app)?;
            app.set_menu(app_menu)?;
            Ok(())
        })
}

fn build_menu(app: &tauri::App) -> tauri::Result<Menu<tauri::Wry>> {
    let session_new = MenuItem::with_id(app, "session_new", "New Session", true, None::<&str>)?;
    let session_continue =
        MenuItem::with_id(app, "session_continue", "Continue…", true, None::<&str>)?;
    let session_last = MenuItem::with_id(app, "session_last", "Last Session", true, None::<&str>)?;
    let session_menu = Submenu::with_items(
        app,
        "Session",
        true,
        &[&session_new, &session_continue, &session_last],
    )?;

    let view_reload = MenuItem::with_id(app, "view_reload", "Reload", true, None::<&str>)?;
    let help_about = MenuItem::with_id(app, "help_about", "About Rex", true, None::<&str>)?;
    let view_menu = Submenu::with_items(app, "View", true, &[&view_reload])?;

    let help_menu = Submenu::with_items(app, "Help", true, &[&help_about])?;

    let app_submenu = Submenu::with_items(
        app,
        "Rex",
        true,
        &[
            &PredefinedMenuItem::about(app, None, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::hide(app, None)?,
            &PredefinedMenuItem::hide_others(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::quit(app, None)?,
        ],
    )?;

    Menu::with_items(
        app,
        &[
            &app_submenu,
            &session_menu,
            &view_menu,
            &help_menu,
        ],
    )
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    build_app()
        .run(tauri::generate_context!())
        .expect("error while running Rex desktop");
}
