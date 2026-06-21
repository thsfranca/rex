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
    #[error("daemon interrupted the stream before completion")]
    StreamInterrupted,
    #[error("daemon stream ended without completion marker")]
    StreamIncomplete,
    #[error("sidecar required but unavailable: {detail}")]
    SidecarUnavailable { detail: String },
    #[error("inference runtime not configured: {detail}")]
    InferenceConfig { detail: String },
    #[error("failed to write NDJSON to stdout: {0}")]
    Stdout(#[from] io::Error),
    #[error("agent execution requires approval; re-run with --approval-id <id> or --yes")]
    ApprovalRequired,
    #[error("agent execution approval denied")]
    ApprovalDenied,
}

impl CliError {
    pub fn daemon_unavailable_manual(socket_path: &str) -> Self {
        Self::DaemonUnavailable {
            socket_path: socket_path.to_string(),
            suffix: "; run `rex daemon` or remove --no-daemon-autostart to auto-start".to_string(),
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
}
