//! Interactive terminal UI entry (R073).

mod app;

use std::io::{self, IsTerminal, Write};
use std::process::ExitCode;

use rex_stream_ui::{LifecycleContext, LifecyclePhase, OperatorMessaging};

use crate::daemon_lifecycle::{ensure_daemon_ready, EnsureOptions};
use crate::error::CliError;

/// Run the Rex terminal harness (multi-pane TUI).
pub async fn run_tui(no_daemon_autostart: bool) -> Result<(), CliError> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err(CliError::Status(tonic::Status::failed_precondition(
            "rex tui requires an interactive terminal",
        )));
    }

    let msg = OperatorMessaging::lifecycle_message(LifecyclePhase::StartingSpawn, &LifecycleContext::default());
    writeln!(io::stderr(), "{msg}").map_err(CliError::Stdout)?;

    ensure_daemon_ready(EnsureOptions {
        no_autostart: no_daemon_autostart,
    })
    .await?;

    let ready = OperatorMessaging::lifecycle_message(LifecyclePhase::Ready, &LifecycleContext::default());
    writeln!(io::stderr(), "{ready}").map_err(CliError::Stdout)?;

    crate::tui::app::run().await.map_err(|err| {
        CliError::Status(tonic::Status::internal(err.to_string()))
    })
}

pub async fn run_tui_exit_code(no_daemon_autostart: bool) -> ExitCode {
    match run_tui(no_daemon_autostart).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("Error: {err}");
            ExitCode::from(1)
        }
    }
}

/// Whether TTY `complete` should delegate to the TUI.
pub fn should_delegate_tty_to_tui(no_ui: bool) -> bool {
    if no_ui {
        return false;
    }
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return false;
    }
    rex_config::load_merged()
        .map(|loaded| loaded.effective.cli.ui.should_use_tui(true, no_ui))
        .unwrap_or(true)
}
