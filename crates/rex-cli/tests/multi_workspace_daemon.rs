//! Integration: per-workspace daemon sockets (R075).

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use assert_cmd::cargo::cargo_bin;
use rex_config::{DaemonSocketScope, RexConfig, REX_ROOT_ENV};
use serial_test::serial;
use tempfile::TempDir;

struct RexRootGuard {
    _dir: TempDir,
    prev_rex_root: Option<String>,
}

impl RexRootGuard {
    fn new() -> Self {
        let dir = TempDir::new().expect("temp rex root");
        let prev_rex_root = std::env::var(REX_ROOT_ENV).ok();
        std::env::set_var(REX_ROOT_ENV, dir.path());
        Self {
            _dir: dir,
            prev_rex_root,
        }
    }
}

impl Drop for RexRootGuard {
    fn drop(&mut self) {
        match &self.prev_rex_root {
            Some(v) => std::env::set_var(REX_ROOT_ENV, v),
            None => std::env::remove_var(REX_ROOT_ENV),
        }
    }
}

fn per_workspace_global_config() -> RexConfig {
    let mut cfg = RexConfig::defaults();
    cfg.daemon.socket_scope = Some(DaemonSocketScope::PerWorkspace);
    cfg.inference.runtime = "mock".to_string();
    cfg.sidecars.harness = Some("direct".to_string());
    cfg.sidecars.required = Some(false);
    if let Some(entry) = cfg.sidecars.list.first_mut() {
        entry.enabled = false;
    }
    cfg
}

fn write_project(root: &TempDir, name: &str, workspace_root: &PathBuf) -> PathBuf {
    let proj = root.path().join(name);
    fs::create_dir_all(proj.join(".rex")).expect("mkdir");
    let mut overlay = RexConfig {
        version: 1,
        workspace: rex_config::WorkspaceConfig {
            root: workspace_root.display().to_string(),
            ..Default::default()
        },
        ..Default::default()
    };
    overlay.daemon.socket_scope = Some(DaemonSocketScope::PerWorkspace);
    fs::write(
        proj.join(".rex/config.json"),
        serde_json::to_string_pretty(&overlay).expect("serialize"),
    )
    .expect("write project config");
    proj
}

#[test]
#[serial]
fn two_projects_get_distinct_daemon_sockets_and_workspace_status() {
    let guard = RexRootGuard::new();
    let global = per_workspace_global_config();
    fs::write(
        guard._dir.path().join("config.json"),
        serde_json::to_string_pretty(&global).expect("serialize"),
    )
    .expect("write global config");

    let proj_a = write_project(&guard._dir, "proj-a", &guard._dir.path().join("proj-a"));
    let proj_b = write_project(&guard._dir, "proj-b", &guard._dir.path().join("proj-b"));

    let status_a = Command::new(cargo_bin("rex"))
        .current_dir(&proj_a)
        .env(REX_ROOT_ENV, guard._dir.path())
        .args(["status"])
        .output()
        .expect("status a");
    assert!(
        status_a.status.success(),
        "proj-a status failed: {}",
        String::from_utf8_lossy(&status_a.stderr)
    );

    let status_b = Command::new(cargo_bin("rex"))
        .current_dir(&proj_b)
        .env(REX_ROOT_ENV, guard._dir.path())
        .args(["status"])
        .output()
        .expect("status b");
    assert!(
        status_b.status.success(),
        "proj-b status failed: {}",
        String::from_utf8_lossy(&status_b.stderr)
    );

    let stdout_a = String::from_utf8_lossy(&status_a.stdout);
    let stdout_b = String::from_utf8_lossy(&status_b.stdout);
    let root_a = proj_a.canonicalize().unwrap_or_else(|_| proj_a.clone());
    let root_b = proj_b.canonicalize().unwrap_or_else(|_| proj_b.clone());
    assert!(stdout_a.contains(&format!("workspace_root: {}", root_a.display())));
    assert!(stdout_b.contains(&format!("workspace_root: {}", root_b.display())));
    assert_ne!(stdout_a, stdout_b);

    let sockets_dir = guard._dir.path().join("sockets");
    let entries: Vec<_> = fs::read_dir(&sockets_dir)
        .expect("sockets dir")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "sock"))
        .collect();
    assert_eq!(entries.len(), 2, "expected two workspace daemon sockets");
}
