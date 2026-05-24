use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use thiserror::Error;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tokio::time::{sleep, timeout};

use crate::sidecar_client::{connect_sidecar, health_check};
use crate::sidecar_config::SidecarConfig;

const HEALTH_TIMEOUT: Duration = Duration::from_secs(2);
const STARTUP_TIMEOUT: Duration = Duration::from_secs(8);
const HEALTH_POLL: Duration = Duration::from_millis(100);

#[derive(Debug, Error)]
pub enum SupervisorError {
    #[error("sidecar binary not found at {path}")]
    BinaryMissing { path: String },
    #[error("failed to spawn sidecar: {0}")]
    Spawn(String),
    #[error("sidecar did not become healthy within startup budget")]
    StartupTimeout,
}

pub struct SidecarSupervisor {
    config: SidecarConfig,
    child: Mutex<Option<Child>>,
}

impl SidecarSupervisor {
    pub fn new(config: SidecarConfig) -> Self {
        Self {
            config,
            child: Mutex::new(None),
        }
    }

    pub fn config(&self) -> &SidecarConfig {
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
        matches!(
            timeout(HEALTH_TIMEOUT, async {
                let mut client = connect_sidecar(&self.config.socket_path).await.ok()?;
                let ok = health_check(&mut client).await.ok()?;
                Some(ok)
            })
            .await,
            Ok(Some(true))
        )
    }

    pub async fn restart(&self) -> Result<(), SupervisorError> {
        self.stop().await;
        if !self.config.binary.exists() {
            return Err(SupervisorError::BinaryMissing {
                path: self.config.binary.display().to_string(),
            });
        }
        println!(
            "sidecar.lifecycle=spawn binary={} socket={}",
            self.config.binary.display(),
            self.config.socket_path
        );
        let daemon_socket = std::env::var("REX_DAEMON_SOCKET")
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| crate::domain::SOCKET_PATH.to_string());
        let rex_home = rex_config::rex_home();
        let child = Command::new(&self.config.binary)
            .env("REX_SIDECAR_SOCKET", &self.config.socket_path)
            .env("REX_DAEMON_SOCKET", &daemon_socket)
            .env("REX_HOME", &rex_home)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| SupervisorError::Spawn(e.to_string()))?;
        *self.child.lock().await = Some(child);
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
                println!("sidecar.health=ok socket={}", self.config.socket_path);
                Ok(())
            }
            _ => {
                self.stop().await;
                Err(SupervisorError::StartupTimeout)
            }
        }
    }

    pub async fn stop(&self) {
        if let Some(mut child) = self.child.lock().await.take() {
            let _ = child.kill().await;
            let _ = child.wait().await;
            println!(
                "sidecar.lifecycle=stopped socket={}",
                self.config.socket_path
            );
        }
    }
}

pub type SharedSupervisor = Arc<SidecarSupervisor>;

pub fn supervisor_from_env() -> SharedSupervisor {
    Arc::new(SidecarSupervisor::new(SidecarConfig::from_env()))
}
