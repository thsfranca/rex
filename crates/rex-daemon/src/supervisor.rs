use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use thiserror::Error;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tokio::time::{sleep, timeout};

use crate::capability_client::{capability_health_check, connect_capability};
use crate::sidecar_client::{connect_sidecar, health_check};
use crate::sidecar_config::{SidecarConfig, SidecarFleetConfig, SidecarProcessConfig};

const HEALTH_TIMEOUT: Duration = Duration::from_secs(2);
const STARTUP_TIMEOUT: Duration = Duration::from_secs(8);
const HEALTH_POLL: Duration = Duration::from_millis(100);

#[derive(Debug, Error)]
pub enum SupervisorError {
    #[error("sidecar binary not found at {path}")]
    BinaryMissing { path: String },
    #[error("failed to spawn sidecar {name}: {message}")]
    Spawn { name: String, message: String },
    #[error("sidecar {name} did not become healthy within startup budget")]
    StartupTimeout { name: String },
}

pub struct SidecarProcessSupervisor {
    config: SidecarProcessConfig,
    child: Mutex<Option<Child>>,
}

impl SidecarProcessSupervisor {
    pub fn new(config: SidecarProcessConfig) -> Self {
        Self {
            config,
            child: Mutex::new(None),
        }
    }

    pub fn config(&self) -> &SidecarProcessConfig {
        &self.config
    }

    pub async fn ensure_running(&self) -> Result<(), SupervisorError> {
        if !self.config.enabled {
            return Ok(());
        }
        if self.is_healthy().await {
            return Ok(());
        }
        self.restart().await
    }

    pub async fn is_healthy(&self) -> bool {
        if !self.config.enabled {
            return true;
        }
        let socket = self.config.socket_path.clone();
        let is_capability = self.config.is_capability;
        matches!(
            timeout(HEALTH_TIMEOUT, async move {
                if is_capability {
                    let mut client = connect_capability(&socket).await.ok()?;
                    let ok = capability_health_check(&mut client).await.ok()?;
                    Some(ok)
                } else {
                    let mut client = connect_sidecar(&socket).await.ok()?;
                    let ok = health_check(&mut client).await.ok()?;
                    Some(ok)
                }
            })
            .await,
            Ok(Some(true))
        )
    }

    pub async fn restart(&self) -> Result<(), SupervisorError> {
        self.stop().await;
        let binary = self.config.binary.to_string_lossy();
        if !rex_config::sidecar_binary_resolvable(binary.as_ref()) {
            let mut path = self.config.binary.display().to_string();
            if let Some(hint) = rex_config::sidecar_install_hint(binary.as_ref()) {
                path = format!("{path}; {hint}");
            }
            return Err(SupervisorError::BinaryMissing { path });
        }
        let role = if self.config.is_capability {
            "capability"
        } else {
            "host"
        };
        println!(
            "sidecar.lifecycle=spawn role={role} name={} binary={} socket={}",
            self.config.name,
            self.config.binary.display(),
            self.config.socket_path
        );
        let daemon_socket = crate::settings::get().daemon_socket().to_string();
        let rex_root = crate::settings::get().rex_root.clone();
        let proto_gen = rex_root.join("proto").join("gen");
        let pythonpath = std::env::var_os("PYTHONPATH")
            .map(|existing| format!("{}:{}", proto_gen.display(), existing.to_string_lossy()))
            .unwrap_or_else(|| proto_gen.display().to_string());
        let child = Command::new(&self.config.binary)
            .env("REX_ROOT", rex_root.display().to_string())
            .env("PYTHONPATH", pythonpath)
            .env("REX_SIDECAR_SOCKET", &self.config.socket_path)
            .env("REX_DAEMON_SOCKET", &daemon_socket)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|err| SupervisorError::Spawn {
                name: self.config.name.clone(),
                message: err.to_string(),
            })?;
        *self.child.lock().await = Some(child);
        let name = self.config.name.clone();
        let ready = timeout(STARTUP_TIMEOUT, async {
            loop {
                if self.is_healthy().await {
                    return true;
                }
                sleep(HEALTH_POLL).await;
            }
        })
        .await;
        match ready {
            Ok(true) => {
                println!(
                    "sidecar.health=ok role={role} name={} socket={}",
                    self.config.name, self.config.socket_path
                );
                Ok(())
            }
            _ => {
                self.stop().await;
                Err(SupervisorError::StartupTimeout { name })
            }
        }
    }

    pub async fn stop(&self) {
        if let Some(mut child) = self.child.lock().await.take() {
            let _ = child.kill().await;
            let _ = child.wait().await;
            println!(
                "sidecar.lifecycle=stopped name={} socket={}",
                self.config.name, self.config.socket_path
            );
        }
        remove_stale_socket_if_present(&self.config.socket_path);
    }
}

pub struct SidecarFleet {
    host: SidecarProcessSupervisor,
    capabilities: Vec<SidecarProcessSupervisor>,
}

impl SidecarFleet {
    pub fn new(config: SidecarFleetConfig) -> Self {
        let SidecarFleetConfig { host, capabilities } = config;
        Self {
            host: SidecarProcessSupervisor::new(host),
            capabilities: capabilities
                .into_iter()
                .map(SidecarProcessSupervisor::new)
                .collect(),
        }
    }

    pub fn host_config(&self) -> SidecarConfig {
        SidecarConfig::from(self.host.config())
    }

    /// Compatibility alias for host-only callers.
    pub fn config(&self) -> SidecarConfig {
        self.host_config()
    }

    #[allow(dead_code)]
    pub fn capabilities(&self) -> &[SidecarProcessSupervisor] {
        &self.capabilities
    }

    pub async fn ensure_running(&self) -> Result<(), SupervisorError> {
        self.host.ensure_running().await?;
        for capability in &self.capabilities {
            capability.ensure_running().await?;
        }
        Ok(())
    }

    pub async fn stop(&self) {
        for capability in &self.capabilities {
            capability.stop().await;
        }
        self.host.stop().await;
    }
}

fn remove_stale_socket_if_present(socket_path: &str) {
    let path = Path::new(socket_path);
    if path.exists() {
        let _ = std::fs::remove_file(path);
    }
}

pub type SharedSidecarFleet = Arc<SidecarFleet>;

/// Legacy alias used by service and runtime.
pub type SharedSupervisor = SharedSidecarFleet;
#[allow(dead_code)]
pub type SidecarSupervisor = SidecarFleet;

pub fn supervisor_from_config() -> SharedSidecarFleet {
    Arc::new(SidecarFleet::new(SidecarFleetConfig::from_config(
        &crate::settings::get(),
    )))
}
