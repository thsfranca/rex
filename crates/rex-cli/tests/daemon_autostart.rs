//! Integration: CLI auto-starts detached daemon when socket is missing (R071).

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use assert_cmd::cargo::cargo_bin;
use rex_config::{DaemonSocketScope, RexConfig, REX_ROOT_ENV};
use serial_test::serial;
use tempfile::TempDir;

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
        std::env::set_var(REX_ROOT_ENV, dir.path());
        Self {
            _dir: dir,
            prev_rex_root,
            socket_path,
        }
    }
}

impl Drop for RexRootGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.socket_path);
        match &self.prev_rex_root {
            Some(v) => std::env::set_var(REX_ROOT_ENV, v),
            None => std::env::remove_var(REX_ROOT_ENV),
        }
    }
}

#[test]
#[serial]
fn status_autostarts_detached_daemon_by_default() {
    let guard = RexRootGuard::new();
    assert!(!guard.socket_path.exists());

    let output = Command::new(cargo_bin("rex"))
        .current_dir(guard._dir.path())
        .env(REX_ROOT_ENV, guard._dir.path())
        .args(["status"])
        .output()
        .expect("run rex status");

    assert!(
        output.status.success(),
        "status failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        guard.socket_path.exists(),
        "daemon socket should exist after auto-start"
    );
    assert!(output.stdout.windows(15).any(|w| w == b"daemon_version:"));
}

#[test]
#[serial]
fn status_no_autostart_flag_fails_when_daemon_missing() {
    let guard = RexRootGuard::new();
    assert!(!guard.socket_path.exists());

    let output = Command::new(cargo_bin("rex"))
        .current_dir(guard._dir.path())
        .env(REX_ROOT_ENV, guard._dir.path())
        .args(["status", "--no-daemon-autostart"])
        .output()
        .expect("run rex status");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("daemon is unavailable"),
        "expected daemon_unavailable message, got: {stderr}"
    );
    assert!(!guard.socket_path.exists());
}

#[test]
#[serial]
fn config_auto_start_false_skips_spawn() {
    let dir = TempDir::new().expect("temp rex root");
    let socket_path = dir.path().join("rex-optout.sock");
    let mut cfg = mock_autostart_config(
        socket_path
            .to_str()
            .expect("socket path must be utf-8"),
    );
    cfg.daemon.auto_start = Some(false);
    fs::write(
        dir.path().join("config.json"),
        serde_json::to_string_pretty(&cfg).expect("serialize config"),
    )
    .expect("write config");

    let prev_rex_root = std::env::var(REX_ROOT_ENV).ok();
    std::env::set_var(REX_ROOT_ENV, dir.path());

    let output = Command::new(cargo_bin("rex"))
        .current_dir(dir.path())
        .env(REX_ROOT_ENV, dir.path())
        .args(["status"])
        .output()
        .expect("run rex status");

    if let Some(v) = prev_rex_root {
        std::env::set_var(REX_ROOT_ENV, v);
    } else {
        std::env::remove_var(REX_ROOT_ENV);
    }

    assert!(!output.status.success());
    assert!(!socket_path.exists());
}
