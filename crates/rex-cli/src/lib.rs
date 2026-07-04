mod daemon_lifecycle;
mod domain;
mod error;
mod harness_session;
pub mod lock_util;
mod session_meta;
pub mod session_resume;
mod transport;
mod tui;

use std::process::ExitCode;
use std::time::Duration;

use rex_proto::rex::v1::{GetSystemStatusRequest, GetSystemStatusResponse};

use crate::domain::REQUEST_TIMEOUT_SECONDS;
use crate::transport::connect_client;

pub use daemon_lifecycle::ensure_daemon_ready;
pub use error::CliError;
pub use lock_util::PidLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TuiLaunch {
    New,
    ContinuePicker,
    Last,
}

/// Run the interactive terminal workspace (product entry).
pub async fn run_tui_main(launch: TuiLaunch) -> ExitCode {
    match tui::run_tui(launch).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("Error: {}", err.operator_message());
            ExitCode::from(1)
        }
    }
}

/// Ensure the daemon is up, then return `GetSystemStatus` (for integration tests).
pub async fn system_status() -> Result<GetSystemStatusResponse, CliError> {
    ensure_daemon_ready().await?;
    let mut client = connect_client(None).await?;
    let mut request = tonic::Request::new(GetSystemStatusRequest {});
    request.set_timeout(Duration::from_secs(REQUEST_TIMEOUT_SECONDS));
    let response = client.get_system_status(request).await?;
    Ok(response.into_inner())
}
