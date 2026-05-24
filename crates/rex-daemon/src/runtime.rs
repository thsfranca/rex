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
use crate::approvals::approval_gate_from_env;
use crate::domain::{DAEMON_VERSION, SOCKET_PATH};
use crate::policy::PolicyEngine;
use crate::service::RexDaemonService;
use crate::supervisor::{supervisor_from_env, SupervisorError};

#[derive(Debug, Error)]
pub enum DaemonRuntimeError {
    #[error("inference runtime configuration: {0}")]
    InferenceConfig(String),
    #[error("failed to remove stale socket at {path}: {source}")]
    SocketCleanup { path: String, source: io::Error },
    #[error("failed to bind daemon socket at {path}: {source}")]
    SocketBind { path: String, source: io::Error },
    #[error("daemon transport failure: {0}")]
    Transport(#[from] tonic::transport::Error),
    #[error("sidecar supervisor: {0}")]
    Sidecar(#[from] SupervisorError),
}

pub async fn run_daemon() -> Result<(), DaemonRuntimeError> {
    if let Ok(config) = rex_config::load_merged() {
        rex_config::apply_to_env(&config);
    }
    run_daemon_on_socket(
        std::env::var("REX_DAEMON_SOCKET")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .unwrap_or_else(|| SOCKET_PATH.to_string())
            .as_str(),
    )
    .await
}

pub async fn run_daemon_on_socket(socket_path: &str) -> Result<(), DaemonRuntimeError> {
    remove_stale_socket(socket_path)?;
    let listener =
        UnixListener::bind(socket_path).map_err(|source| DaemonRuntimeError::SocketBind {
            path: socket_path.to_string(),
            source,
        })?;
    let incoming = UnixListenerStream::new(listener);
    let runtime = runtime_from_env().map_err(|message| {
        eprintln!("rex-daemon inference runtime failed: {message}");
        DaemonRuntimeError::InferenceConfig(message)
    })?;
    let approval_gate = approval_gate_from_env();
    let sidecar = supervisor_from_env();
    if sidecar.config().enabled {
        if let Err(err) = sidecar.ensure_running().await {
            let config = sidecar.config();
            if config.required {
                return Err(DaemonRuntimeError::Sidecar(err));
            }
            eprintln!("rex-daemon sidecar optional start failed: {err}");
        }
    }
    let service = RexDaemonService::with_components(
        Instant::now(),
        runtime,
        PolicyEngine::with_default_layers(),
        approval_gate,
        sidecar.clone(),
    );

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
