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
    #[error("daemon is unavailable at {socket_path}; start rex-daemon and retry")]
    DaemonUnavailable { socket_path: String },
    #[error("daemon interrupted the stream before completion")]
    StreamInterrupted,
    #[error("daemon stream ended without completion marker")]
    StreamIncomplete,
}
