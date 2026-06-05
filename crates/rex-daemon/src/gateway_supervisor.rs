use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use reqwest::Client;
use rex_config::{
    effective_gateway_port, gateway_required, is_managed_gateway, resolve_gateway_config_path,
    DEFAULT_GATEWAY_STARTUP_TIMEOUT_SECS,
};
use thiserror::Error;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tokio::time::{sleep, timeout};

const HEALTH_POLL: Duration = Duration::from_millis(100);

#[derive(Debug, Clone)]
pub struct GatewaySupervisorConfig {
    pub enabled: bool,
    pub base_url: String,
    pub command: String,
    pub config_path: PathBuf,
    pub port: u16,
    pub startup_timeout: Duration,
    pub required: bool,
    pub discovery_on_ready: bool,
    pub rex_root: PathBuf,
}

impl GatewaySupervisorConfig {
    pub fn from_loaded(loaded: &rex_config::LoadedConfig) -> Self {
        let gateway = &loaded.effective.inference.gateway;
        let enabled = is_managed_gateway(gateway);
        let port = effective_gateway_port(gateway);
        let startup_secs = if gateway.startup_timeout_secs == 0 {
            DEFAULT_GATEWAY_STARTUP_TIMEOUT_SECS
        } else {
            gateway.startup_timeout_secs
        };
        let discovery_on_ready = gateway.ollama.discovery_on_ready.unwrap_or(true);
        Self {
            enabled,
            base_url: loaded.effective_openai_compat_base_url(),
            command: gateway.command.clone(),
            config_path: resolve_gateway_config_path(gateway, &loaded.rex_root),
            port,
            startup_timeout: Duration::from_secs(startup_secs),
            required: gateway_required(gateway),
            discovery_on_ready,
            rex_root: loaded.rex_root.clone(),
        }
    }
}

#[derive(Debug, Error)]
pub enum GatewaySupervisorError {
    #[error("gateway command not found on PATH: {command}")]
    CommandMissing { command: String },
    #[error("failed to spawn gateway: {0}")]
    Spawn(String),
    #[error("gateway did not become healthy within startup budget")]
    StartupTimeout,
}

pub struct GatewaySupervisor {
    config: GatewaySupervisorConfig,
    child: Mutex<Option<Child>>,
    http: Client,
}

impl GatewaySupervisor {
    pub fn new(config: GatewaySupervisorConfig) -> Self {
        Self {
            config,
            child: Mutex::new(None),
            http: Client::builder()
                .timeout(Duration::from_secs(5))
                .build()
                .expect("gateway health http client"),
        }
    }

    pub fn config(&self) -> &GatewaySupervisorConfig {
        &self.config
    }

    pub async fn ensure_running(&self) -> Result<(), GatewaySupervisorError> {
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
        health_check_url(&self.http, &models_url(&self.config.base_url))
            .await
            .unwrap_or(false)
    }

    pub async fn restart(&self) -> Result<(), GatewaySupervisorError> {
        self.stop().await;
        if !gateway_command_resolvable(&self.config.command) {
            return Err(GatewaySupervisorError::CommandMissing {
                command: self.config.command.clone(),
            });
        }
        println!(
            "gateway.lifecycle=spawn command={} port={} config={}",
            self.config.command,
            self.config.port,
            self.config.config_path.display()
        );
        let mut cmd = Command::new(&self.config.command);
        if uses_litellm_cli(&self.config.command) {
            cmd.arg("--config")
                .arg(&self.config.config_path)
                .arg("--port")
                .arg(self.config.port.to_string());
        }
        apply_gateway_env_file(&mut cmd, &rex_config::gateway_env_path());
        cmd.env("REX_ROOT", self.config.rex_root.display().to_string());
        cmd.stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        let child = cmd
            .spawn()
            .map_err(|e| GatewaySupervisorError::Spawn(e.to_string()))?;
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
                println!("gateway.health=ok base_url={}", self.config.base_url);
                if self.config.discovery_on_ready {
                    self.log_model_discovery().await;
                }
                Ok(())
            }
            _ => {
                self.stop().await;
                println!("gateway.lifecycle=failed reason=startup_timeout");
                Err(GatewaySupervisorError::StartupTimeout)
            }
        }
    }

    async fn log_model_discovery(&self) {
        let url = models_url(&self.config.base_url);
        match self.http.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => {
                let count = resp
                    .json::<serde_json::Value>()
                    .await
                    .ok()
                    .and_then(|v| v.get("data").and_then(|d| d.as_array()).map(|a| a.len()))
                    .unwrap_or(0);
                let ollama = if count > 0 { "ok" } else { "empty" };
                println!("gateway.ollama.discovery={ollama} gateway.models.count={count}");
            }
            Ok(_) => println!("gateway.ollama.discovery=unreachable gateway.models.count=0"),
            Err(_) => println!("gateway.ollama.discovery=unreachable gateway.models.count=0"),
        }
    }

    pub async fn stop(&self) {
        if let Some(mut child) = self.child.lock().await.take() {
            let _ = child.kill().await;
            let _ = child.wait().await;
            println!("gateway.lifecycle=stopped port={}", self.config.port);
        }
    }
}

pub type SharedGatewaySupervisor = Arc<GatewaySupervisor>;

pub fn gateway_supervisor_from_config() -> SharedGatewaySupervisor {
    Arc::new(GatewaySupervisor::new(
        GatewaySupervisorConfig::from_loaded(&crate::settings::get()),
    ))
}

fn models_url(base_url: &str) -> String {
    let trimmed = base_url.trim_end_matches('/');
    if trimmed.ends_with("/v1") {
        format!("{trimmed}/models")
    } else {
        format!("{trimmed}/v1/models")
    }
}

async fn health_check_url(client: &Client, url: &str) -> Result<bool, reqwest::Error> {
    let resp = client.get(url).send().await?;
    Ok(resp.status().is_success())
}

fn uses_litellm_cli(command: &str) -> bool {
    let base = command
        .rsplit('/')
        .next()
        .unwrap_or(command)
        .rsplit('\\')
        .next()
        .unwrap_or(command);
    base.eq_ignore_ascii_case("litellm")
}

fn gateway_command_resolvable(command: &str) -> bool {
    if command.contains('/') {
        return std::path::Path::new(command).is_file();
    }
    rex_config::sidecar_binary_resolvable(command)
}

fn apply_gateway_env_file(cmd: &mut Command, path: &std::path::Path) {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return;
    };
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        cmd.env(key.trim(), value.trim().trim_matches('"'));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn models_url_appends_v1_models() {
        assert_eq!(
            models_url("http://127.0.0.1:4000/v1"),
            "http://127.0.0.1:4000/v1/models"
        );
    }
}
