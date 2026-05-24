#![allow(dead_code)]

use std::path::PathBuf;
use std::sync::Arc;

use rex_config::{LoadedConfig, RexConfig};
use tempfile::TempDir;

pub struct RexRootGuard {
    _dir: TempDir,
    prev_rex_root: Option<String>,
}

impl Drop for RexRootGuard {
    fn drop(&mut self) {
        match &self.prev_rex_root {
            Some(v) => std::env::set_var("REX_ROOT", v),
            None => std::env::remove_var("REX_ROOT"),
        }
    }
}

pub fn install_rex_config(cfg: RexConfig) -> RexRootGuard {
    let dir = TempDir::new().expect("temp rex root");
    std::fs::write(
        dir.path().join("config.json"),
        serde_json::to_string_pretty(&cfg).expect("serialize config"),
    )
    .expect("write config.json");
    let prev_rex_root = std::env::var("REX_ROOT").ok();
    std::env::set_var("REX_ROOT", dir.path());
    RexRootGuard {
        _dir: dir,
        prev_rex_root,
    }
}

pub fn loaded_from_config(cfg: RexConfig, rex_root: &std::path::Path) -> Arc<LoadedConfig> {
    Arc::new(LoadedConfig {
        rex_root: rex_root.to_path_buf(),
        global_path: Some(rex_root.join("config.json")),
        project_path: None,
        effective: cfg,
    })
}

pub fn mock_e2e_config() -> RexConfig {
    let mut cfg = RexConfig::defaults();
    cfg.inference.runtime = "mock".to_string();
    cfg.sidecars.harness = Some("direct".to_string());
    cfg.sidecars.required = Some(false);
    if let Some(entry) = cfg.sidecars.list.first_mut() {
        entry.enabled = false;
    }
    cfg
}

pub fn mock_e2e_with_approvals(enabled: bool) -> RexConfig {
    let mut cfg = mock_e2e_config();
    cfg.agent.approvals_enabled = Some(enabled);
    cfg
}

pub fn cursor_cli_e2e_config(command: &str) -> RexConfig {
    let mut cfg = mock_e2e_config();
    cfg.inference.runtime = "cursor-cli".to_string();
    cfg.inference.cursor_cli.command = Some(command.to_string());
    cfg
}

pub fn product_path_config(
    daemon_socket: &str,
    sidecar_socket: &str,
    workspace: &str,
    http_base_url: &str,
    sidecar_binary: &str,
) -> RexConfig {
    let mut cfg = RexConfig::defaults();
    cfg.daemon.socket = daemon_socket.to_string();
    cfg.inference.runtime = "http-openai-compat".to_string();
    cfg.inference.openai_compat.base_url = http_base_url.to_string();
    cfg.inference.openai_compat.api_key = None;
    cfg.sidecars.harness = None;
    cfg.sidecars.required = Some(true);
    cfg.sidecars.active = "stub".to_string();
    cfg.sidecars.list = vec![rex_config::SidecarEntry {
        name: "stub".to_string(),
        binary: sidecar_binary.to_string(),
        enabled: true,
        socket: sidecar_socket.to_string(),
    }];
    cfg.workspace.root = workspace.to_string();
    cfg
}

pub fn sidecar_required_missing_binary_config(
    daemon_socket: &str,
    sidecar_socket: &str,
    missing_binary: &str,
) -> RexConfig {
    let mut cfg = mock_e2e_config();
    cfg.daemon.socket = daemon_socket.to_string();
    cfg.sidecars.harness = None;
    cfg.sidecars.required = Some(true);
    if let Some(entry) = cfg.sidecars.list.first_mut() {
        entry.enabled = true;
        entry.binary = missing_binary.to_string();
        entry.socket = sidecar_socket.to_string();
    }
    cfg
}

pub fn rex_root_path(guard: &RexRootGuard) -> PathBuf {
    guard._dir.path().to_path_buf()
}
