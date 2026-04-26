use std::io;
use std::path::Path;
use std::time::Instant;

use rex_proto::rex::v1::rex_service_server::RexServiceServer;
use thiserror::Error;
use tokio::net::UnixListener;
use tokio::signal;
use tokio_stream::wrappers::UnixListenerStream;
use tonic::transport::Server;

use crate::adapters::{runtime_from_env, RuntimeKind};
use crate::domain::{DAEMON_VERSION, SOCKET_PATH};
use crate::service::RexDaemonService;

#[derive(Debug, Error)]
pub enum DaemonRuntimeError {
    #[error("failed to remove stale socket at {path}: {source}")]
    SocketCleanup { path: String, source: io::Error },
    #[error("failed to bind daemon socket at {path}: {source}")]
    SocketBind { path: String, source: io::Error },
    #[error("daemon transport failure: {0}")]
    Transport(#[from] tonic::transport::Error),
}

pub async fn run_daemon() -> Result<(), DaemonRuntimeError> {
    run_daemon_on_socket(SOCKET_PATH).await
}

pub async fn run_daemon_on_socket(socket_path: &str) -> Result<(), DaemonRuntimeError> {
    remove_stale_socket(socket_path)?;
    let listener =
        UnixListener::bind(socket_path).map_err(|source| DaemonRuntimeError::SocketBind {
            path: socket_path.to_string(),
            source,
        })?;
    let incoming = UnixListenerStream::new(listener);
    let runtime = runtime_from_env();
    let service = RexDaemonService::with_runtime(Instant::now(), runtime);

    println!(
        "rex-daemon event=listen socket={} inference_runtime={} daemon_version={}",
        socket_path,
        RuntimeKind::from_env().log_label(),
        DAEMON_VERSION
    );
    Server::builder()
        .add_service(RexServiceServer::new(service))
        .serve_with_incoming_shutdown(incoming, shutdown_signal())
        .await?;
    remove_stale_socket(socket_path)?;
    println!(
        "rex-daemon event=shutdown socket={} reason=signal",
        socket_path
    );

    Ok(())
}

fn remove_stale_socket(path: &str) -> Result<(), DaemonRuntimeError> {
    let socket_path = Path::new(path);
    if socket_path.exists() {
        std::fs::remove_file(socket_path).map_err(|source| DaemonRuntimeError::SocketCleanup {
            path: path.to_string(),
            source,
        })?;
    }
    Ok(())
}

async fn shutdown_signal() {
    let _ = signal::ctrl_c().await;
}
