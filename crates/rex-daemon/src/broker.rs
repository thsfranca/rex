//! Host capability broker (MVP: workspace `fs.read`, `fs.list`, `fs.write`, `exec.shell`).

use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::access_policy::{AccessDecision, PolicyDeny};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum BrokerError {
    #[error("workspace root not configured")]
    NoWorkspaceRoot,
    #[error("access policy denied ({code}): {message}")]
    PolicyDenied { code: String, message: String },
    #[error("path escapes workspace: {0}")]
    EscapesWorkspace(String),
    #[error("path not found: {0}")]
    NotFound(String),
    #[error("read failed: {0}")]
    Io(String),
    #[error("write too large: {0} bytes (max {1})")]
    WriteTooLarge(usize, usize),
    #[error("command not allowlisted: {0}")]
    CommandNotAllowed(String),
    #[error("shell execution failed: {0}")]
    ShellFailed(String),
}

const MAX_WRITE_BYTES: usize = 65_536;
const MAX_SHELL_OUTPUT_BYTES: usize = 8_192;
const MAX_LIST_DIR_ENTRIES: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrokerDirEntry {
    pub name: String,
    pub is_dir: bool,
}

impl From<PolicyDeny> for BrokerError {
    fn from(deny: PolicyDeny) -> Self {
        Self::PolicyDenied {
            code: deny.code.to_string(),
            message: deny.message,
        }
    }
}

pub fn workspace_root_from_config() -> Result<PathBuf, BrokerError> {
    let root = crate::settings::get().workspace_root();
    if root.as_os_str().is_empty() {
        return Err(BrokerError::NoWorkspaceRoot);
    }
    Ok(root)
}

pub fn broker_read_file(relative_path: &str) -> Result<String, BrokerError> {
    match crate::access_policy::evaluate_fs_read(relative_path) {
        AccessDecision::Allow => {}
        AccessDecision::Deny(deny) => return Err(deny.into()),
    }
    let root = workspace_root_from_config()?;
    let resolved = resolve_under_workspace(&root, relative_path)?;
    std::fs::read_to_string(&resolved).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            BrokerError::NotFound(relative_path.to_string())
        } else {
            BrokerError::Io(e.to_string())
        }
    })
}

pub fn broker_list_dir(relative_path: &str) -> Result<Vec<BrokerDirEntry>, BrokerError> {
    match crate::access_policy::evaluate_fs_list(relative_path) {
        AccessDecision::Allow => {}
        AccessDecision::Deny(deny) => return Err(deny.into()),
    }
    let root = workspace_root_from_config()?;
    let resolved = resolve_list_dir(&root, relative_path)?;
    if !resolved.is_dir() {
        return Err(BrokerError::Io(format!(
            "not a directory: {}",
            relative_path.trim()
        )));
    }
    let mut entries = Vec::new();
    for entry in fs::read_dir(&resolved).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            BrokerError::NotFound(relative_path.trim().to_string())
        } else {
            BrokerError::Io(e.to_string())
        }
    })? {
        let entry = entry.map_err(|e| BrokerError::Io(e.to_string()))?;
        let file_type = entry
            .file_type()
            .map_err(|e| BrokerError::Io(e.to_string()))?;
        let name = entry.file_name().to_string_lossy().to_string();
        entries.push(BrokerDirEntry {
            name,
            is_dir: file_type.is_dir(),
        });
        if entries.len() >= MAX_LIST_DIR_ENTRIES {
            break;
        }
    }
    entries.sort_by(|a, b| {
        a.name
            .to_ascii_lowercase()
            .cmp(&b.name.to_ascii_lowercase())
    });
    Ok(entries)
}

pub fn broker_write_file(relative_path: &str, content: &str) -> Result<(), BrokerError> {
    let capability = "fs.write";
    let _ = capability;
    if content.len() > MAX_WRITE_BYTES {
        return Err(BrokerError::WriteTooLarge(content.len(), MAX_WRITE_BYTES));
    }
    let root = workspace_root_from_config()?;
    let resolved = resolve_under_workspace_for_write(&root, relative_path)?;
    if let Some(parent) = resolved.parent() {
        fs::create_dir_all(parent).map_err(|e| BrokerError::Io(e.to_string()))?;
    }
    fs::write(&resolved, content).map_err(|e| BrokerError::Io(e.to_string()))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellResult {
    pub stdout: String,
    pub stderr: String,
}

pub fn broker_exec_shell(command: &str) -> Result<ShellResult, BrokerError> {
    let capability = "exec.shell";
    let _ = capability;
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return Err(BrokerError::CommandNotAllowed("(empty)".to_string()));
    }
    let mut parts = trimmed.split_whitespace();
    let program = parts
        .next()
        .ok_or_else(|| BrokerError::CommandNotAllowed("(empty)".to_string()))?;
    if !shell_allowlist().contains(&program.to_ascii_lowercase()) {
        return Err(BrokerError::CommandNotAllowed(program.to_string()));
    }
    let root = workspace_root_from_config()?;
    let output = std::process::Command::new(program)
        .args(parts)
        .current_dir(&root)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .map_err(|e| BrokerError::ShellFailed(e.to_string()))?;
    if !output.status.success() {
        return Err(BrokerError::ShellFailed(format!(
            "exit={:?}",
            output.status.code()
        )));
    }
    Ok(ShellResult {
        stdout: truncate_output(&String::from_utf8_lossy(&output.stdout)),
        stderr: truncate_output(&String::from_utf8_lossy(&output.stderr)),
    })
}

fn shell_allowlist() -> Vec<String> {
    crate::settings::get()
        .broker_shell_allowlist()
        .iter()
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty())
        .collect()
}

fn truncate_output(text: &str) -> String {
    if text.len() <= MAX_SHELL_OUTPUT_BYTES {
        return text.to_string();
    }
    let mut out = text
        .chars()
        .take(MAX_SHELL_OUTPUT_BYTES)
        .collect::<String>();
    out.push_str(" [rex: shell output truncated]");
    out
}

fn resolve_under_workspace_for_write(
    root: &Path,
    relative_path: &str,
) -> Result<PathBuf, BrokerError> {
    let rel = Path::new(relative_path.trim());
    if rel.is_absolute() {
        return Err(BrokerError::EscapesWorkspace(relative_path.to_string()));
    }
    let mut candidate = root.to_path_buf();
    for component in rel.components() {
        match component {
            Component::Normal(part) => candidate.push(part),
            Component::CurDir => {}
            Component::ParentDir => {
                return Err(BrokerError::EscapesWorkspace(relative_path.to_string()));
            }
            _ => return Err(BrokerError::EscapesWorkspace(relative_path.to_string())),
        }
    }
    let canonical_root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let parent = candidate
        .parent()
        .map(|p| p.canonicalize().unwrap_or_else(|_| p.to_path_buf()))
        .unwrap_or_else(|| canonical_root.clone());
    if !parent.starts_with(&canonical_root) {
        return Err(BrokerError::EscapesWorkspace(relative_path.to_string()));
    }
    Ok(candidate)
}

fn resolve_under_workspace(root: &Path, relative_path: &str) -> Result<PathBuf, BrokerError> {
    let rel = Path::new(relative_path.trim());
    if rel.is_absolute() {
        return Err(BrokerError::EscapesWorkspace(relative_path.to_string()));
    }
    let mut candidate = root.to_path_buf();
    for component in rel.components() {
        match component {
            Component::Normal(part) => candidate.push(part),
            Component::CurDir => {}
            Component::ParentDir => {
                return Err(BrokerError::EscapesWorkspace(relative_path.to_string()));
            }
            _ => return Err(BrokerError::EscapesWorkspace(relative_path.to_string())),
        }
    }
    let canonical_root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let canonical = candidate
        .canonicalize()
        .map_err(|_| BrokerError::NotFound(relative_path.to_string()))?;
    if !canonical.starts_with(&canonical_root) {
        return Err(BrokerError::EscapesWorkspace(relative_path.to_string()));
    }
    Ok(canonical)
}

fn resolve_list_dir(root: &Path, relative_path: &str) -> Result<PathBuf, BrokerError> {
    let trimmed = relative_path.trim();
    if trimmed.is_empty() {
        return Ok(root.canonicalize().unwrap_or_else(|_| root.to_path_buf()));
    }
    resolve_under_workspace(root, trimmed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::fs;
    use std::sync::Arc;

    struct SettingsGuard;

    impl Drop for SettingsGuard {
        fn drop(&mut self) {
            crate::settings::reset_for_test();
        }
    }

    fn init_workspace_root(root: &str) -> SettingsGuard {
        crate::settings::reset_for_test();
        let mut cfg = rex_config::RexConfig::defaults();
        cfg.workspace.root = root.to_string();
        crate::settings::init_for_test(Arc::new(rex_config::LoadedConfig {
            rex_root: std::path::PathBuf::from("/tmp/rex-broker-test"),
            global_path: None,
            project_path: None,
            effective: cfg,
        }));
        SettingsGuard
    }

    #[test]
    #[serial]
    fn reads_file_under_workspace() {
        let dir = std::env::temp_dir().join(format!("rex-broker-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("tmpdir");
        let file = dir.join("hello.txt");
        fs::write(&file, "broker-ok").expect("write");
        let _guard = init_workspace_root(&dir.display().to_string());
        let content = broker_read_file("hello.txt").expect("read");
        assert_eq!(content, "broker-ok");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    #[serial]
    fn writes_file_under_workspace() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir =
            std::env::temp_dir().join(format!("rex-broker-write-{}-{}", std::process::id(), nonce));
        fs::create_dir_all(&dir).expect("tmpdir");
        let _guard = init_workspace_root(&dir.display().to_string());
        broker_write_file("out.txt", "written-by-broker").expect("write");
        let content = broker_read_file("out.txt").expect("read back");
        assert_eq!(content, "written-by-broker");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    #[serial]
    fn denies_protected_env_file() {
        let dir = std::env::temp_dir().join(format!("rex-broker-policy-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("tmpdir");
        let _guard = init_workspace_root(&dir.display().to_string());
        let err = broker_read_file(".env").unwrap_err();
        assert!(matches!(err, BrokerError::PolicyDenied { .. }));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    #[serial]
    fn exec_shell_runs_allowlisted_echo() {
        let root = std::env::temp_dir().join(format!("rex-broker-shell-{}", std::process::id()));
        fs::create_dir_all(&root).expect("tmpdir");
        let _guard = init_workspace_root(&root.display().to_string());
        let out = broker_exec_shell("echo broker-shell-ok").expect("echo");
        assert!(out.stdout.contains("broker-shell-ok"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    #[serial]
    fn exec_shell_denies_disallowed_command() {
        let root =
            std::env::temp_dir().join(format!("rex-broker-shell-deny-{}", std::process::id()));
        fs::create_dir_all(&root).expect("tmpdir");
        let _guard = init_workspace_root(&root.display().to_string());
        let err = broker_exec_shell("rm -rf /").unwrap_err();
        assert!(matches!(err, BrokerError::CommandNotAllowed(_)));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    #[serial]
    fn rejects_parent_traversal() {
        let dir = std::env::temp_dir().join(format!("rex-broker-deny-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("tmpdir");
        let _guard = init_workspace_root(&dir.display().to_string());
        let err = broker_read_file("../etc/passwd").unwrap_err();
        assert_eq!(
            err,
            BrokerError::EscapesWorkspace("../etc/passwd".to_string())
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    #[serial]
    fn lists_workspace_root_and_subdirectory() {
        let dir = std::env::temp_dir().join(format!("rex-broker-list-{}", std::process::id()));
        fs::create_dir_all(dir.join("src")).expect("srcdir");
        fs::write(dir.join("hello.txt"), "x").expect("file");
        let _guard = init_workspace_root(&dir.display().to_string());
        let root_entries = broker_list_dir("").expect("list root");
        assert!(root_entries
            .iter()
            .any(|e| e.name == "hello.txt" && !e.is_dir));
        assert!(root_entries.iter().any(|e| e.name == "src" && e.is_dir));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    #[serial]
    fn list_dir_denies_protected_path() {
        let dir = std::env::temp_dir().join(format!("rex-broker-list-deny-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("tmpdir");
        let _guard = init_workspace_root(&dir.display().to_string());
        let err = broker_list_dir(".env").unwrap_err();
        assert!(matches!(err, BrokerError::PolicyDenied { .. }));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    #[serial]
    fn list_dir_caps_entries() {
        let dir = std::env::temp_dir().join(format!("rex-broker-list-cap-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("tmpdir");
        for i in 0..300 {
            fs::write(dir.join(format!("file-{i:03}.txt")), "x").expect("file");
        }
        let _guard = init_workspace_root(&dir.display().to_string());
        let entries = broker_list_dir("").expect("list");
        assert_eq!(entries.len(), MAX_LIST_DIR_ENTRIES);
        let _ = fs::remove_dir_all(&dir);
    }
}
