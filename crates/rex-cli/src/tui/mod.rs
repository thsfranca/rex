//! Interactive terminal UI entry.

mod app;
mod approval;
mod history_fetch;
mod motion;
mod state;
mod stream_task;
mod theme;
mod ui;
mod viewport;

use std::io::{self, IsTerminal, Write};

use rex_stream_ui::{LifecycleContext, LifecyclePhase, OperatorMessaging};

use crate::daemon_lifecycle::ensure_daemon_ready;
use crate::error::CliError;

/// Run the Rex terminal harness (multi-pane TUI).
pub async fn run_tui() -> Result<(), CliError> {
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

    crate::tui::app::run().await.map_err(|err| {
        CliError::Status(tonic::Status::internal(err))
    })
}
