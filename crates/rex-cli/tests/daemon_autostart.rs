//! Integration: CLI auto-starts detached daemon when socket is missing (R071).

use std::fs;
use std::path::PathBuf;

use assert_cmd::cargo::cargo_bin;
use rex_config::{DaemonSocketScope, RexConfig, REX_ROOT_ENV};
use serial_test::serial;
use tempfile::TempDir;

fn set_rex_bin_env() {
    std::env::set_var("CARGO_BIN_EXE_rex", cargo_bin("rex"));
}

fn mock_autostart_config(socket_path: &str) -> RexConfig {
    let mut cfg = RexConfig::defaults();
    cfg.daemon.socket = Some(socket_path.to_string());
    cfg.daemon.socket_scope = Some(DaemonSocketScope::Global);
    cfg.inference.runtime = "mock".to_string();
    cfg.sidecars.harness = Some("direct".to_string());
    cfg.sidecars.required = Some(false);
    if let Some(entry) = cfg.sidecars.list.first_mut() {
        entry.enabled = false;
    }
    cfg
}

struct RexRootGuard {
    _dir: TempDir,
    prev_rex_root: Option<String>,
    prev_cwd: PathBuf,
    socket_path: PathBuf,
}

impl RexRootGuard {
    fn new() -> Self {
        let dir = TempDir::new().expect("temp rex root");
        let socket_path = dir.path().join("rex-autostart.sock");
        let cfg = mock_autostart_config(
            socket_path
                .to_str()
                .expect("socket path must be utf-8"),
        );
        fs::write(
            dir.path().join("config.json"),
            serde_json::to_string_pretty(&cfg).expect("serialize config"),
        )
        .expect("write config");
        let prev_rex_root = std::env::var(REX_ROOT_ENV).ok();
        let prev_cwd = std::env::current_dir().expect("cwd");
        std::env::set_var(REX_ROOT_ENV, dir.path());
        std::env::set_current_dir(dir.path()).expect("chdir");
        Self {
            _dir: dir,
            prev_rex_root,
            prev_cwd,
            socket_path,
        }
    }
}

impl Drop for RexRootGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.socket_path);
        let _ = std::env::set_current_dir(&self.prev_cwd);
        match &self.prev_rex_root {
            Some(v) => std::env::set_var(REX_ROOT_ENV, v),
            None => std::env::remove_var(REX_ROOT_ENV),
        }
    }
}

#[tokio::test]
#[serial]
async fn ensure_starts_detached_daemon() {
    set_rex_bin_env();
    let guard = RexRootGuard::new();
    assert!(!guard.socket_path.exists());

    rex_cli::ensure_daemon_ready()
        .await
        .expect("ensure daemon");

    assert!(
        guard.socket_path.exists(),
        "daemon socket should exist after ensure"
    );
}
