//! Integration: concurrent ensure_daemon_ready shares one daemon (parallel harness PR1).

use std::fs;
use std::path::PathBuf;

use assert_cmd::cargo::cargo_bin;
use rex_config::{DaemonSocketScope, RexConfig, REX_ROOT_ENV};
use serial_test::serial;
use tempfile::TempDir;

fn set_rex_bin_env() {
    std::env::set_var("CARGO_BIN_EXE_rex", cargo_bin("rex"));
}

struct RexRootGuard {
    _dir: TempDir,
    prev_rex_root: Option<String>,
    prev_cwd: PathBuf,
    workspace_root: PathBuf,
}

impl RexRootGuard {
    fn new() -> Self {
        let dir = TempDir::new().expect("temp rex root");
        let workspace = dir.path().join("workspace");
        fs::create_dir_all(workspace.join(".rex")).expect("mkdir project");

        let mut global = RexConfig::defaults();
        global.daemon.socket_scope = Some(DaemonSocketScope::PerWorkspace);
        global.inference.runtime = "mock".to_string();
        global.sidecars.harness = Some("direct".to_string());
        global.sidecars.required = Some(false);
        if let Some(entry) = global.sidecars.list.first_mut() {
            entry.enabled = false;
        }
        fs::write(
            dir.path().join("config.json"),
            serde_json::to_string_pretty(&global).expect("serialize"),
        )
        .expect("write global config");

        let overlay = RexConfig {
            version: 1,
            daemon: rex_config::DaemonConfig {
                socket_scope: Some(DaemonSocketScope::PerWorkspace),
                ..Default::default()
            },
            ..Default::default()
        };
        fs::write(
            workspace.join(".rex/config.json"),
            serde_json::to_string_pretty(&overlay).expect("serialize"),
        )
        .expect("write project config");

        let prev_rex_root = std::env::var(REX_ROOT_ENV).ok();
        let prev_cwd = std::env::current_dir().expect("cwd");
        std::env::set_var(REX_ROOT_ENV, dir.path());
        std::env::set_current_dir(&workspace).expect("chdir workspace");

        Self {
            _dir: dir,
            prev_rex_root,
            prev_cwd,
            workspace_root: workspace,
        }
    }
}

impl Drop for RexRootGuard {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.prev_cwd);
        match &self.prev_rex_root {
            Some(v) => std::env::set_var(REX_ROOT_ENV, v),
            None => std::env::remove_var(REX_ROOT_ENV),
        }
    }
}

#[tokio::test]
#[serial]
async fn concurrent_ensure_daemon_ready_single_socket() {
    set_rex_bin_env();
    let guard = RexRootGuard::new();

    let (first, second) = tokio::join!(
        rex_cli::ensure_daemon_ready(),
        rex_cli::ensure_daemon_ready(),
    );
    first.expect("first ensure");
    second.expect("second ensure");

    let status = rex_cli::system_status()
        .await
        .expect("status after concurrent ensure");
    let expected = guard
        .workspace_root
        .canonicalize()
        .unwrap_or_else(|_| guard.workspace_root.clone());
    assert_eq!(status.workspace_root, expected.display().to_string());

    let sockets_dir = guard._dir.path().join("sockets");
    let sock_count = fs::read_dir(&sockets_dir)
        .expect("sockets dir")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "sock"))
        .count();
    assert_eq!(
        sock_count, 1,
        "parallel ensure must not spawn duplicate workspace daemons"
    );
}
