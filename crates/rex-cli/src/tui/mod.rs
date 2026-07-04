//! Interactive terminal UI entry.

mod app;
mod approval;
mod history_fetch;
mod motion;
mod session_picker;
mod state;
mod stream_task;
mod theme;
mod ui;
mod viewport;

use std::io::{self, IsTerminal, Write};
use std::path::PathBuf;

use rex_stream_ui::{LifecycleContext, LifecyclePhase, OperatorMessaging};

use crate::daemon_lifecycle::ensure_daemon_ready;
use crate::error::CliError;
use crate::lock_util::PidLock;
use crate::session_meta::{read_meta, session_log_path};
use crate::session_resume::{
    acquire_session_lock, list_closed_sessions, record_closed_session,
    resolve_last_available_session_id,
};
use crate::TuiLaunch;

/// Configuration for a single TUI run (new or resumed session).
pub struct TuiRunConfig {
    pub harness_session_id: String,
    pub session_title: String,
    pub resume: bool,
    pub session_lock: Option<PidLock>,
    pub workspace_root: PathBuf,
}

/// Run the Rex terminal harness (multi-pane TUI).
pub async fn run_tui(launch: TuiLaunch) -> Result<(), CliError> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err(CliError::Status(tonic::Status::failed_precondition(
            "rex requires an interactive terminal",
        )));
    }

    let msg = OperatorMessaging::lifecycle_message(LifecyclePhase::StartingSpawn, &LifecycleContext::default());
    writeln!(io::stderr(), "{msg}").map_err(CliError::Stdout)?;

    ensure_daemon_ready().await?;

    let ready = OperatorMessaging::lifecycle_message(LifecyclePhase::Ready, &LifecycleContext::default());
    writeln!(io::stderr(), "{ready}").map_err(CliError::Stdout)?;

    let workspace = crate::session_resume::workspace_root()?;
    let config = match launch {
        TuiLaunch::ContinuePicker => {
            let sessions = list_closed_sessions(&workspace)?;
            if sessions.is_empty() {
                return Err(CliError::NoSessionToContinue);
            }
            let basename = workspace
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("workspace")
                .to_string();
            let picked = session_picker::run_picker(&basename, &sessions).map_err(|err| {
                CliError::Status(tonic::Status::internal(err))
            })?;
            let Some(harness_session_id) = picked else {
                return Ok(());
            };
            build_resume_config(&workspace, harness_session_id).await?
        }
        other => resolve_launch_config(other, &workspace).await?,
    };

    let harness_session_id = config.harness_session_id.clone();
    let result = crate::tui::app::run(config).await.map_err(|err| {
        CliError::Status(tonic::Status::internal(err))
    });

    if let Some(log) = session_log_path(&workspace, &harness_session_id) {
        if log.is_file() {
            let _ = record_closed_session(&workspace, &harness_session_id);
        }
    }

    result
}

async fn resolve_launch_config(
    launch: TuiLaunch,
    workspace: &PathBuf,
) -> Result<TuiRunConfig, CliError> {
    match launch {
        TuiLaunch::New => {
            let harness_session_id = crate::harness_session::new_harness_session_id();
            let lock = acquire_session_lock(workspace, &harness_session_id)?;
            Ok(TuiRunConfig {
                harness_session_id,
                session_title: String::new(),
                resume: false,
                session_lock: Some(lock),
                workspace_root: workspace.clone(),
            })
        }
        TuiLaunch::Last => {
            let harness_session_id = resolve_last_available_session_id(workspace)?;
            build_resume_config(workspace, harness_session_id).await
        }
        TuiLaunch::ContinuePicker => unreachable!("handled in run_tui"),
    }
}

async fn build_resume_config(
    workspace: &PathBuf,
    harness_session_id: String,
) -> Result<TuiRunConfig, CliError> {
    let log = session_log_path(workspace, &harness_session_id).ok_or(CliError::SessionNotFound)?;
    if !log.is_file() {
        return Err(CliError::SessionNotFound);
    }
    let lock = acquire_session_lock(workspace, &harness_session_id)?;
    let meta = read_meta(workspace, &harness_session_id);
    Ok(TuiRunConfig {
        harness_session_id,
        session_title: meta.title,
        resume: true,
        session_lock: Some(lock),
        workspace_root: workspace.clone(),
    })
}
