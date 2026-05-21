//! Host capability broker (MVP: workspace `fs.read`).

use std::env;
use std::path::{Component, Path, PathBuf};

use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum BrokerError {
    #[error("workspace root not configured")]
    NoWorkspaceRoot,
    #[error("path escapes workspace: {0}")]
    EscapesWorkspace(String),
    #[error("path not found: {0}")]
    NotFound(String),
    #[error("read failed: {0}")]
    Io(String),
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
    let capability = "fs.read";
    let _ = capability;
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
    use std::fs;

    #[test]
    fn reads_file_under_workspace() {
        let dir = std::env::temp_dir().join(format!("rex-broker-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("tmpdir");
        let file = dir.join("hello.txt");
        fs::write(&file, "broker-ok").expect("write");
        env::set_var("REX_WORKSPACE_ROOT", dir.display().to_string());
        let content = broker_read_file("hello.txt").expect("read");
        assert_eq!(content, "broker-ok");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_parent_traversal() {
        let dir = std::env::temp_dir().join(format!("rex-broker-deny-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("tmpdir");
        env::set_var("REX_WORKSPACE_ROOT", dir.display().to_string());
        let err = broker_read_file("../etc/passwd").unwrap_err();
        assert_eq!(
            err,
            BrokerError::EscapesWorkspace("../etc/passwd".to_string())
        );
        let _ = fs::remove_dir_all(&dir);
    }
}
