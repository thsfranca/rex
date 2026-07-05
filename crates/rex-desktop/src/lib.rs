mod control;
mod stream;

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use control::{
    fetch_session_events as fetch_session_events_rpc, get_system_status as get_system_status_rpc,
    respond_to_tool_approval as respond_to_tool_approval_rpc, DaemonLifecycleEvent,
    FetchSessionEventsDto, SystemStatusDto, ToolApprovalResultDto,
};
use rex_cli::ensure_daemon_ready;
use rex_cli::CliError;
use rex_cli::DesktopLaunch;
use serde::Serialize;
use stream::{StreamEventDto, submit_prompt_stream};
use tauri::ipc::Channel;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem, Submenu},
    Emitter, Manager,
};

static SESSION_COUNTER: AtomicU64 = AtomicU64::new(0);

fn new_harness_session_id() -> String {
    let seq = SESSION_COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("hs-{}-{}-{}", std::process::id(), seq, nanos)
}

struct LaunchState {
    debug: bool,
}

#[derive(Serialize)]
struct LaunchOptionsDto {
    debug: bool,
}

#[tauri::command]
fn launch_options(state: tauri::State<LaunchState>) -> LaunchOptionsDto {
    LaunchOptionsDto {
        debug: state.debug,
    }
}

#[tauri::command]
async fn ensure_daemon() -> Result<SystemStatusDto, String> {
    ensure_daemon_ready()
        .await
        .map_err(|e: CliError| e.operator_message())?;
    get_system_status_rpc()
        .await
        .map_err(|e: CliError| e.operator_message())
}

#[tauri::command]
async fn get_system_status() -> Result<SystemStatusDto, String> {
    get_system_status_rpc()
        .await
        .map_err(|e: CliError| e.operator_message())
}

#[tauri::command]
async fn fetch_session_events(
    harness_session_id: String,
    before_sequence: u64,
    after_sequence: u64,
    limit: u32,
) -> Result<FetchSessionEventsDto, String> {
    fetch_session_events_rpc(
        harness_session_id,
        before_sequence,
        after_sequence,
        limit,
    )
    .await
    .map_err(|e: CliError| e.operator_message())
}

#[tauri::command]
async fn respond_to_tool_approval(
    approval_token: String,
    approved: bool,
    tool_call_id: String,
    harness_session_id: String,
) -> Result<ToolApprovalResultDto, String> {
    respond_to_tool_approval_rpc(
        approval_token,
        approved,
        tool_call_id,
        harness_session_id,
    )
    .await
    .map_err(|e: CliError| e.operator_message())
}

#[tauri::command]
async fn submit_prompt(
    prompt: String,
    mode: String,
    on_event: Channel<StreamEventDto>,
) -> Result<String, String> {
    let session_id = new_harness_session_id();
    let trace_id = format!("web-{}", session_id);
    submit_prompt_stream(prompt, mode, trace_id, session_id.clone(), on_event)
        .await
        .map_err(|e: CliError| e.operator_message())?;
    Ok(session_id)
}

fn spawn_daemon_lifecycle_monitor(app: &tauri::App) {
    let handle = app.handle().clone();
    tauri::async_runtime::spawn(async move {
        loop {
            let event = control::probe_daemon_lifecycle().await;
            let _ = handle.emit("daemon-lifecycle", event);
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });
}

pub fn build_app(debug: bool) -> tauri::Builder<tauri::Wry> {
    let mut builder = tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            launch_options,
            ensure_daemon,
            get_system_status,
            fetch_session_events,
            respond_to_tool_approval,
            submit_prompt,
        ])
        .setup(move |app| {
            app.manage(LaunchState { debug });
            spawn_daemon_lifecycle_monitor(app);
            let app_menu = build_menu(app)?;
            app.set_menu(app_menu)?;
            app.on_menu_event(|app, event| {
                let _ = app.emit("menu-action", event.id().0.clone());
            });
            Ok(())
        });

    #[cfg(feature = "e2e-testing")]
    {
        builder = builder.plugin(tauri_plugin_playwright::init_with_config(
            tauri_plugin_playwright::PluginConfig::new().socket_path(playwright_socket_path()),
        ));
    }

    builder
}

fn playwright_socket_path() -> String {
    std::env::var("TAURI_PLAYWRIGHT_SOCKET").unwrap_or_else(|_| "/tmp/rex-playwright.sock".into())
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
pub fn run(launch: DesktopLaunch) {
    build_app(launch.debug)
        .run(tauri::generate_context!())
        .expect("error while running Rex desktop");
}
