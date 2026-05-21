//! Host capability broker (MVP: workspace `fs.read`, `fs.write`, `exec.shell`).

use std::env;
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
const DEFAULT_SHELL_ALLOWLIST: &str = "echo,printf,true";

impl From<PolicyDeny> for BrokerError {
    fn from(deny: PolicyDeny) -> Self {
        Self::PolicyDenied {
            code: deny.code.to_string(),
            message: deny.message,
        }
    }
}

pub fn workspace_root_from_env() -> Result<PathBuf, BrokerError> {
    let raw = env::var("REX_WORKSPACE_ROOT")
        .or_else(|_| env::current_dir().map(|p| p.display().to_string()))
        .map_err(|_| BrokerError::NoWorkspaceRoot)?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(BrokerError::NoWorkspaceRoot);
    }
    Ok(PathBuf::from(trimmed))
}

pub fn broker_read_file(relative_path: &str) -> Result<String, BrokerError> {
    match crate::access_policy::evaluate_fs_read(relative_path) {
        AccessDecision::Allow => {}
        AccessDecision::Deny(deny) => return Err(deny.into()),
    }
    let root = workspace_root_from_env()?;
    let resolved = resolve_under_workspace(&root, relative_path)?;
    std::fs::read_to_string(&resolved).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            BrokerError::NotFound(relative_path.to_string())
        } else {
            BrokerError::Io(e.to_string())
        }
    })
}

pub fn broker_write_file(relative_path: &str, content: &str) -> Result<(), BrokerError> {
    let capability = "fs.write";
    let _ = capability;
    if content.len() > MAX_WRITE_BYTES {
        return Err(BrokerError::WriteTooLarge(content.len(), MAX_WRITE_BYTES));
    }
    let root = workspace_root_from_env()?;
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
    let root = workspace_root_from_env()?;
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
    env::var("REX_BROKER_SHELL_ALLOWLIST")
        .unwrap_or_else(|_| DEFAULT_SHELL_ALLOWLIST.to_string())
        .split(',')
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

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::fs;

    struct WorkspaceRootGuard {
        previous: Option<String>,
    }

    impl WorkspaceRootGuard {
        fn set(value: String) -> Self {
            let previous = env::var("REX_WORKSPACE_ROOT").ok();
            env::set_var("REX_WORKSPACE_ROOT", value);
            Self { previous }
        }
    }

    impl Drop for WorkspaceRootGuard {
        fn drop(&mut self) {
            match &self.previous {
                Some(v) => env::set_var("REX_WORKSPACE_ROOT", v),
                None => env::remove_var("REX_WORKSPACE_ROOT"),
            }
        }
    }

    #[test]
    #[serial]
    fn reads_file_under_workspace() {
        let dir = std::env::temp_dir().join(format!("rex-broker-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("tmpdir");
        let file = dir.join("hello.txt");
        fs::write(&file, "broker-ok").expect("write");
        let _guard = WorkspaceRootGuard::set(dir.display().to_string());
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
        let _guard = WorkspaceRootGuard::set(dir.display().to_string());
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
        let _guard = WorkspaceRootGuard::set(dir.display().to_string());
        let err = broker_read_file(".env").unwrap_err();
        assert!(matches!(err, BrokerError::PolicyDenied { .. }));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    #[serial]
    fn exec_shell_runs_allowlisted_echo() {
        let root = std::env::temp_dir().join(format!("rex-broker-shell-{}", std::process::id()));
        fs::create_dir_all(&root).expect("tmpdir");
        let _guard = WorkspaceRootGuard::set(root.display().to_string());
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
        let _guard = WorkspaceRootGuard::set(root.display().to_string());
        let err = broker_exec_shell("rm -rf /").unwrap_err();
        assert!(matches!(err, BrokerError::CommandNotAllowed(_)));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    #[serial]
    fn rejects_parent_traversal() {
        let dir = std::env::temp_dir().join(format!("rex-broker-deny-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("tmpdir");
        let _guard = WorkspaceRootGuard::set(dir.display().to_string());
        let err = broker_read_file("../etc/passwd").unwrap_err();
        assert_eq!(
            err,
            BrokerError::EscapesWorkspace("../etc/passwd".to_string())
        );
        let _ = fs::remove_dir_all(&dir);
    }
}
