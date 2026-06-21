use std::io;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use rex_config::ConfigError;
use rex_proto::rex::v1::rex_service_server::RexServiceServer;
use thiserror::Error;
use tokio::net::UnixListener;
use tokio::signal;
use tokio::sync::oneshot;
use tokio_stream::wrappers::UnixListenerStream;
use tonic::transport::Server;

use crate::activity::ActivityTracker;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShutdownReason {
    Signal,
    IdleShutdown,
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
    let started_at = Instant::now();
    let activity = Arc::new(ActivityTracker::new(started_at));
    let idle_shutdown_secs = settings::get().daemon_idle_shutdown_secs();
    let (idle_shutdown_tx, idle_shutdown_rx) = oneshot::channel();
    if idle_shutdown_secs > 0 {
        let activity_for_watcher = activity.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(1)).await;
                if activity_for_watcher.should_shutdown(idle_shutdown_secs) {
                    let idle_seconds = activity_for_watcher.idle_seconds();
                    println!(
                        "rex-daemon event=idle_shutdown idle_seconds={idle_seconds} budget_secs={idle_shutdown_secs}"
                    );
                    let _ = idle_shutdown_tx.send(());
                    break;
                }
            }
        });
    } else {
        drop(idle_shutdown_tx);
    }
    let service = RexDaemonService::with_components(
        started_at,
        runtime,
        PolicyEngine::with_default_layers(),
        approval_gate,
        sidecar.clone(),
        activity,
    );
    let workspace_log = match settings::get().resolve_workspace_root() {
        Ok(root) => format!("workspace.root={}", root.display()),
        Err(_) => "workspace.error=not_configured".to_string(),
    };
    println!(
        "rex-daemon event=listen socket={} inference_runtime={} daemon_version={} idle_shutdown_secs={idle_shutdown_secs} {workspace_log}",
        socket_path,
        RuntimeKind::from_config().log_label(),
        DAEMON_VERSION
    );
    let shutdown_reason = Arc::new(std::sync::Mutex::new(ShutdownReason::Signal));
    let reason_for_shutdown = shutdown_reason.clone();
    let idle_rx = if idle_shutdown_secs > 0 {
        Some(idle_shutdown_rx)
    } else {
        None
    };
    Server::builder()
        .add_service(RexServiceServer::new(service))
        .serve_with_incoming_shutdown(incoming, shutdown_trigger(idle_rx, reason_for_shutdown))
        .await?;
    sidecar.stop().await;
    gateway.stop().await;
    remove_stale_socket(socket_path)?;
    let reason = *shutdown_reason
        .lock()
        .expect("shutdown reason mutex should not be poisoned");
    let reason_label = match reason {
        ShutdownReason::Signal => "signal",
        ShutdownReason::IdleShutdown => "idle_shutdown",
    };
    println!(
        "rex-daemon event=shutdown socket={} reason={reason_label}",
        socket_path
    );

    Ok(())
}

async fn shutdown_trigger(
    idle_rx: Option<oneshot::Receiver<()>>,
    reason: Arc<std::sync::Mutex<ShutdownReason>>,
) {
    match idle_rx {
        Some(rx) => {
            tokio::select! {
                _ = signal::ctrl_c() => {}
                _ = rx => {
                    *reason.lock().expect("shutdown reason mutex should not be poisoned") =
                        ShutdownReason::IdleShutdown;
                }
            }
        }
        None => {
            let _ = signal::ctrl_c().await;
        }
    }
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
