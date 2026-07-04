//! Integration: per-workspace daemon sockets (R075).

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
}

impl RexRootGuard {
    fn new() -> Self {
        let dir = TempDir::new().expect("temp rex root");
        let prev_rex_root = std::env::var(REX_ROOT_ENV).ok();
        let prev_cwd = std::env::current_dir().expect("cwd");
        std::env::set_var(REX_ROOT_ENV, dir.path());
        Self {
            _dir: dir,
            prev_rex_root,
            prev_cwd,
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

#[tokio::test]
#[serial]
async fn two_projects_get_distinct_daemon_sockets_and_workspaces() {
    set_rex_bin_env();
    let guard = RexRootGuard::new();
    let global = per_workspace_global_config();
    fs::write(
        guard._dir.path().join("config.json"),
        serde_json::to_string_pretty(&global).expect("serialize"),
    )
    .expect("write global config");

    let proj_a = write_project(&guard._dir, "proj-a", &guard._dir.path().join("proj-a"));
    let proj_b = write_project(&guard._dir, "proj-b", &guard._dir.path().join("proj-b"));
    let root_a = proj_a.canonicalize().unwrap_or_else(|_| proj_a.clone());
    let root_b = proj_b.canonicalize().unwrap_or_else(|_| proj_b.clone());

    std::env::set_current_dir(&proj_a).expect("chdir a");
    let status_a = rex_cli::system_status().await.expect("ensure proj-a");
    assert_eq!(status_a.workspace_root, root_a.display().to_string());

    std::env::set_current_dir(&proj_b).expect("chdir b");
    let status_b = rex_cli::system_status().await.expect("ensure proj-b");
    assert_eq!(status_b.workspace_root, root_b.display().to_string());
    assert_ne!(status_a.workspace_root, status_b.workspace_root);

    let sockets_dir = guard._dir.path().join("sockets");
    let entries: Vec<_> = fs::read_dir(&sockets_dir)
        .expect("sockets dir")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "sock"))
        .collect();
    assert_eq!(entries.len(), 2, "expected two workspace daemon sockets");
}
