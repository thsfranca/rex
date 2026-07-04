use std::io;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CliError {
    #[error("invalid endpoint configuration: {0}")]
    Endpoint(#[from] tonic::transport::Error),
    #[error("transport error: {0}")]
    Status(#[from] tonic::Status),
    #[error("failed to connect to daemon at {socket_path}: {source}")]
    DaemonConnect {
        socket_path: String,
        source: tonic::transport::Error,
    },
    #[error("timed out while waiting for daemon stream chunk after {seconds}s")]
    StreamTimeout { seconds: u64 },
    #[error("daemon is unavailable at {socket_path}{suffix}")]
    DaemonUnavailable {
        socket_path: String,
        suffix: String,
    },
    #[error("failed to write to stdout: {0}")]
    Stdout(#[from] io::Error),
    #[error("could not resolve workspace from current working directory")]
    WorkspaceNotConfigured,
    #[error("daemon at {socket_path} is bound to {reported}; expected {expected}; restart the daemon for this workspace")]
    WorkspaceMismatch {
        socket_path: String,
        expected: String,
        reported: String,
    },
    #[error("no closed session is available to continue")]
    NoSessionToContinue,
    #[error("every recent session is still open in another terminal")]
    AllSessionsOpen,
    #[error("session transcript could not be restored")]
    SessionNotFound,
    #[error("could not acquire session lock; another terminal may have this chat open")]
    SessionLockFailed,
}

impl CliError {
    pub fn product_code(&self) -> Option<&'static str> {
        match self {
            Self::WorkspaceNotConfigured => Some("workspace_not_configured"),
            Self::WorkspaceMismatch { .. } => Some("workspace_mismatch"),
            Self::StreamTimeout { .. } => Some("stream_timeout"),
            Self::NoSessionToContinue => Some("no_session_to_continue"),
            Self::AllSessionsOpen => Some("all_sessions_open"),
            Self::SessionNotFound => Some("session_not_found"),
            Self::SessionLockFailed => Some("session_lock_failed"),
            Self::DaemonUnavailable { .. } => Some("daemon_unavailable"),
            _ => None,
        }
    }

    pub fn operator_message(&self) -> String {
        if let Some(code) = self.product_code() {
            format!("{code}: {self}")
        } else {
            self.to_string()
        }
    }

    pub fn daemon_spawn_failed(log_path: &std::path::Path, reason: String) -> Self {
        Self::DaemonUnavailable {
            socket_path: String::new(),
            suffix: format!(
                "; could not start Rex: {reason} — see {}",
                log_path.display()
            ),
        }
    }

    pub fn daemon_ready_timeout(log_path: &std::path::Path, timeout_secs: u64) -> Self {
        Self::DaemonUnavailable {
            socket_path: String::new(),
            suffix: format!(
                "; Rex did not become ready within {timeout_secs}s — see {}",
                log_path.display()
            ),
        }
    }

    pub fn workspace_not_configured() -> Self {
        Self::WorkspaceNotConfigured
    }

    pub fn workspace_mismatch(
        socket_path: &str,
        expected: String,
        reported: String,
    ) -> Self {
        Self::WorkspaceMismatch {
            socket_path: socket_path.to_string(),
            expected,
            reported,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resume_errors_expose_stable_codes() {
        assert_eq!(
            CliError::NoSessionToContinue.product_code(),
            Some("no_session_to_continue")
        );
        assert!(CliError::NoSessionToContinue
            .operator_message()
            .starts_with("no_session_to_continue:"));
        assert_eq!(
            CliError::AllSessionsOpen.product_code(),
            Some("all_sessions_open")
        );
    }
}
