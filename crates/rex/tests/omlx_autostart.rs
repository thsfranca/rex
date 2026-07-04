//! Integration: CLI auto-start with managed oMLX (R071 + oMLX supervisor).

use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::Command;
use std::thread;

use assert_cmd::cargo::cargo_bin;
use rex_config::{DaemonSocketScope, RexConfig, REX_ROOT_ENV};
use serial_test::serial;
use tempfile::TempDir;

const MODELS_JSON: &str = "{\"object\":\"list\",\"data\":[{\"id\":\"test-model\"}]}";

fn spawn_models_fixture() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture");
    let port = listener.local_addr().expect("addr").port();
    let body = MODELS_JSON.to_string();
    thread::spawn(move || {
        for mut stream in listener.incoming().flatten() {
            let body = body.clone();
            thread::spawn(move || {
                let mut buf = [0u8; 512];
                let _ = stream.read(&mut buf);
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(response.as_bytes());
            });
        }
    });
    port
}

fn write_sleep_stub(root: &std::path::Path) -> String {
    let stub = root.join("omlx-stub.sh");
    fs::write(&stub, "#!/bin/sh\nexec sleep 300\n").expect("write stub");
    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(&stub).expect("meta").permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&stub, perms).expect("chmod");
    stub.display().to_string()
}

fn managed_omlx_autostart_config(socket_path: &str, port: u16, omlx_command: &str) -> RexConfig {
    let mut cfg = RexConfig::defaults();
    cfg.daemon.socket = Some(socket_path.to_string());
    cfg.daemon.socket_scope = Some(DaemonSocketScope::Global);
    cfg.daemon.ready_timeout_secs = 15;
    cfg.daemon.log_path = "daemon.log".to_string();
    cfg.inference.runtime = "http-openai-compat".to_string();
    cfg.inference.omlx.mode = "managed".to_string();
    cfg.inference.omlx.port = port;
    cfg.inference.omlx.command = omlx_command.to_string();
    cfg.inference.omlx.startup_timeout_secs = 5;
    cfg.inference.omlx.required = Some(true);
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
    log_path: PathBuf,
}

impl RexRootGuard {
    fn new(port: u16) -> Self {
        let dir = TempDir::new().expect("temp rex root");
        let socket_path = dir.path().join("rex-omlx-autostart.sock");
        let stub = write_sleep_stub(dir.path());
        let cfg = managed_omlx_autostart_config(
            socket_path.to_str().expect("socket utf-8"),
            port,
            &stub,
        );
        fs::write(
            dir.path().join("config.json"),
            serde_json::to_string_pretty(&cfg).expect("serialize config"),
        )
        .expect("write config");
        let prev_rex_root = std::env::var(REX_ROOT_ENV).ok();
        std::env::set_var(REX_ROOT_ENV, dir.path());
        let log_path = dir.path().join("daemon.log");
        Self {
            _dir: dir,
            prev_rex_root,
            socket_path,
            log_path,
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
fn status_autostarts_daemon_with_managed_omlx() {
    let port = spawn_models_fixture();
    let guard = RexRootGuard::new(port);
    assert!(!guard.socket_path.exists());

    let output = Command::new(cargo_bin("rex"))
        .current_dir(guard._dir.path())
        .env(REX_ROOT_ENV, guard._dir.path())
        .args(["__rex_internal_status"])
        .output()
        .expect("run rex status");

    assert!(
        output.status.success(),
        "status failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(guard.socket_path.exists(), "daemon socket should exist");
    let log = fs::read_to_string(&guard.log_path).unwrap_or_default();
    assert!(
        log.contains("omlx.health=ok"),
        "expected omlx health in daemon log, got: {log}"
    );
}

#[test]
#[serial]
fn status_autostart_fails_when_omlx_command_missing() {
    let port = spawn_models_fixture();
    let dir = TempDir::new().expect("temp rex root");
    let socket_path = dir.path().join("rex-omlx-fail.sock");
    let mut cfg = managed_omlx_autostart_config(
        socket_path.to_str().expect("socket utf-8"),
        port,
        "rex-omlx-missing-binary-xyz",
    );
    cfg.daemon.ready_timeout_secs = 3;
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
        .args(["__rex_internal_status"])
        .output()
        .expect("run rex status");

    if let Some(v) = prev_rex_root {
        std::env::set_var(REX_ROOT_ENV, v);
    } else {
        std::env::remove_var(REX_ROOT_ENV);
    }

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("did not become ready") || stderr.contains("unavailable"),
        "expected ready timeout, got: {stderr}"
    );
    assert!(!socket_path.exists());
}
