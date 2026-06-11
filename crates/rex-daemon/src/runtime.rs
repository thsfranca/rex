use std::io;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use rex_config::ConfigError;
use rex_proto::rex::v1::rex_service_server::RexServiceServer;
use thiserror::Error;
use tokio::net::UnixListener;
use tokio::signal;
use tokio_stream::wrappers::UnixListenerStream;
use tonic::transport::Server;

use crate::adapters::{runtime_from_config, RuntimeKind};
use crate::approvals::approval_gate_from_config;
use crate::domain::DAEMON_VERSION;
use crate::gateway_supervisor::{gateway_supervisor_from_config, GatewaySupervisorError};
use crate::policy::PolicyEngine;
use crate::service::RexDaemonService;
use crate::settings;
use crate::sidecar_config::sidecar_harness_direct;
use crate::supervisor::{supervisor_from_config, SupervisorError};

#[derive(Debug, Error)]
pub enum DaemonRuntimeError {
    #[error("configuration: {0}")]
    Config(#[from] ConfigError),
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
    #[error("inference gateway supervisor: {0}")]
    Gateway(#[from] GatewaySupervisorError),
}

pub async fn run_daemon() -> Result<(), DaemonRuntimeError> {
    ensure_settings_loaded()?;
    let socket = settings::get().daemon_socket().to_string();
    run_daemon_on_socket(&socket).await
}

pub async fn run_daemon_on_socket(socket_path: &str) -> Result<(), DaemonRuntimeError> {
    ensure_settings_loaded()?;
    let gateway = gateway_supervisor_from_config();
    if gateway.config().enabled {
        if let Err(err) = gateway.ensure_running().await {
            if gateway.config().required {
                return Err(DaemonRuntimeError::Gateway(err));
            }
            eprintln!("rex-daemon gateway optional start failed: {err}");
        }
    }
    remove_stale_socket(socket_path)?;
    let listener =
        UnixListener::bind(socket_path).map_err(|source| DaemonRuntimeError::SocketBind {
            path: socket_path.to_string(),
            source,
        })?;
    let incoming = UnixListenerStream::new(listener);
    let runtime = runtime_from_config().map_err(|message| {
        eprintln!("rex-daemon inference runtime failed: {message}");
        DaemonRuntimeError::InferenceConfig(message)
    })?;
    let approval_gate = approval_gate_from_config();
    let sidecar = supervisor_from_config();
    if !sidecar_harness_direct() && sidecar.host_config().enabled {
        if let Err(err) = sidecar.ensure_running().await {
            let config = sidecar.host_config();
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
    let workspace_log = match settings::get().resolve_workspace_root() {
        Ok(root) => format!("workspace.root={}", root.display()),
        Err(_) => "workspace.error=not_configured".to_string(),
    };
    println!(
        "rex-daemon event=listen socket={} inference_runtime={} daemon_version={} {workspace_log}",
        socket_path,
        RuntimeKind::from_config().log_label(),
        DAEMON_VERSION
    );
    Server::builder()
        .add_service(RexServiceServer::new(service))
        .serve_with_incoming_shutdown(incoming, shutdown_signal())
        .await?;
    sidecar.stop().await;
    gateway.stop().await;
    remove_stale_socket(socket_path)?;
    println!(
        "rex-daemon event=shutdown socket={} reason=signal",
        socket_path
    );

    Ok(())
}

fn ensure_settings_loaded() -> Result<(), ConfigError> {
    if settings::is_initialized() {
        return Ok(());
    }
    let mut loaded = rex_config::load()?;
    loaded.apply_effective_openai_compat_base_url();
    settings::init(Arc::new(loaded));
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
