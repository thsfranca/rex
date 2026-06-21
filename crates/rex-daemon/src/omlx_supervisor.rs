use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use reqwest::Client;
use rex_config::{
    effective_omlx_health_path, effective_omlx_port, is_managed_omlx, omlx_required,
    DEFAULT_OMLX_STARTUP_TIMEOUT_SECS,
};
use thiserror::Error;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tokio::time::{sleep, timeout};

const HEALTH_POLL: Duration = Duration::from_millis(100);

#[derive(Debug, Clone)]
pub struct OmlxSupervisorConfig {
    pub enabled: bool,
    pub base_url: String,
    pub command: String,
    pub model_dir: String,
    pub port: u16,
    pub health_path: String,
    pub startup_timeout: Duration,
    pub required: bool,
    pub discovery_on_ready: bool,
    pub rex_root: PathBuf,
}

impl OmlxSupervisorConfig {
    pub fn from_loaded(loaded: &rex_config::LoadedConfig) -> Self {
        let omlx = &loaded.effective.inference.omlx;
        let enabled = is_managed_omlx(omlx);
        let port = effective_omlx_port(omlx);
        let startup_secs = if omlx.startup_timeout_secs == 0 {
            DEFAULT_OMLX_STARTUP_TIMEOUT_SECS
        } else {
            omlx.startup_timeout_secs
        };
        let discovery_on_ready = omlx.discovery_on_ready.unwrap_or(true);
        Self {
            enabled,
            base_url: loaded.effective_openai_compat_base_url(),
            command: omlx.command.clone(),
            model_dir: omlx.model_dir.clone(),
            port,
            health_path: effective_omlx_health_path(omlx).to_string(),
            startup_timeout: Duration::from_secs(startup_secs),
            required: omlx_required(omlx),
            discovery_on_ready,
            rex_root: loaded.rex_root.clone(),
        }
    }
}

#[derive(Debug, Error)]
pub enum OmlxSupervisorError {
    #[error("oMLX command not found on PATH: {command}")]
    CommandMissing { command: String },
    #[error("failed to spawn oMLX: {0}")]
    Spawn(String),
    #[error("oMLX did not become healthy within startup budget")]
    StartupTimeout,
}

pub struct OmlxSupervisor {
    config: OmlxSupervisorConfig,
    child: Mutex<Option<Child>>,
    http: Client,
}

impl OmlxSupervisor {
    pub fn new(config: OmlxSupervisorConfig) -> Self {
        Self {
            config,
            child: Mutex::new(None),
            http: Client::builder()
                .timeout(Duration::from_secs(5))
                .build()
                .expect("omlx health http client"),
        }
    }

    pub fn config(&self) -> &OmlxSupervisorConfig {
        &self.config
    }

    pub async fn ensure_running(&self) -> Result<(), OmlxSupervisorError> {
        if !self.config.enabled {
            return Ok(());
        }
        if self.child.lock().await.is_some() && self.is_healthy().await {
            return Ok(());
        }
        self.restart().await
    }

    pub async fn is_healthy(&self) -> bool {
        if !self.config.enabled {
            return true;
        }
        health_check_url(
            &self.http,
            &health_url(&self.config.base_url, &self.config.health_path),
        )
        .await
        .unwrap_or(false)
    }

    pub async fn restart(&self) -> Result<(), OmlxSupervisorError> {
        self.stop().await;
        if !omlx_command_resolvable(&self.config.command) {
            return Err(OmlxSupervisorError::CommandMissing {
                command: self.config.command.clone(),
            });
        }
        println!(
            "omlx.lifecycle=spawn command={} port={}",
            self.config.command, self.config.port
        );
        let mut cmd = Command::new(&self.config.command);
        cmd.arg("serve")
            .arg("--port")
            .arg(self.config.port.to_string());
        if !self.config.model_dir.trim().is_empty() {
            cmd.arg("--model-dir").arg(&self.config.model_dir);
        }
        cmd.env("REX_ROOT", self.config.rex_root.display().to_string());
        cmd.stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        let child = cmd
            .spawn()
            .map_err(|e| OmlxSupervisorError::Spawn(e.to_string()))?;
        *self.child.lock().await = Some(child);

        let ready = timeout(self.config.startup_timeout, async {
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
                println!("omlx.health=ok base_url={}", self.config.base_url);
                if self.config.discovery_on_ready {
                    self.log_model_discovery().await;
                }
                Ok(())
            }
            _ => {
                self.stop().await;
                println!("omlx.lifecycle=failed reason=startup_timeout");
                Err(OmlxSupervisorError::StartupTimeout)
            }
        }
    }

    async fn log_model_discovery(&self) {
        let url = health_url(&self.config.base_url, "/v1/models");
        match self.http.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => {
                let count = resp
                    .json::<serde_json::Value>()
                    .await
                    .ok()
                    .and_then(|v| v.get("data").and_then(|d| d.as_array()).map(|a| a.len()))
                    .unwrap_or(0);
                println!("omlx.models.discovery=ok omlx.models.count={count}");
            }
            Ok(_) => println!("omlx.models.discovery=unreachable omlx.models.count=0"),
            Err(_) => println!("omlx.models.discovery=unreachable omlx.models.count=0"),
        }
    }

    pub async fn stop(&self) {
        if let Some(mut child) = self.child.lock().await.take() {
            let _ = child.kill().await;
            let _ = child.wait().await;
            println!("omlx.lifecycle=stopped port={}", self.config.port);
        }
    }
}

pub type SharedOmlxSupervisor = Arc<OmlxSupervisor>;

pub fn omlx_supervisor_from_config() -> SharedOmlxSupervisor {
    Arc::new(OmlxSupervisor::new(OmlxSupervisorConfig::from_loaded(
        &crate::settings::get(),
    )))
}

fn health_url(base_url: &str, health_path: &str) -> String {
    let path = health_path.trim();
    if path.starts_with("http://") || path.starts_with("https://") {
        return path.to_string();
    }
    let trimmed = base_url.trim_end_matches('/');
    let root = if trimmed.ends_with("/v1") {
        trimmed.trim_end_matches("/v1")
    } else {
        trimmed
    };
    if path.is_empty() {
        return format!("{root}/v1/models");
    }
    if path.starts_with('/') {
        format!("{root}{path}")
    } else {
        format!("{root}/{path}")
    }
}

async fn health_check_url(client: &Client, url: &str) -> Result<bool, reqwest::Error> {
    let resp = client.get(url).send().await?;
    Ok(resp.status().is_success())
}

fn omlx_command_resolvable(command: &str) -> bool {
    if command.contains('/') {
        return std::path::Path::new(command).is_file();
    }
    rex_config::sidecar_binary_resolvable(command)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_url_appends_v1_models() {
        assert_eq!(
            health_url("http://127.0.0.1:8000/v1", "/v1/models"),
            "http://127.0.0.1:8000/v1/models"
        );
    }
}
