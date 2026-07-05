mod daemon_lifecycle;
mod domain;
mod error;
mod harness_session;
pub mod lock_util;
mod session_meta;
pub mod session_resume;
pub mod transport;

use std::time::Duration;

use rex_proto::rex::v1::{GetSystemStatusRequest, GetSystemStatusResponse};

use crate::domain::REQUEST_TIMEOUT_SECONDS;
use crate::transport::connect_client;

pub use daemon_lifecycle::ensure_daemon_ready;
pub use error::CliError;
pub use harness_session::{insert_metadata as insert_harness_session_metadata, new_harness_session_id};
pub use lock_util::PidLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopSession {
    New,
    ContinuePicker,
    Last,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DesktopLaunch {
    pub session: DesktopSession,
    pub debug: bool,
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
