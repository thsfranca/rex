//! Host capability broker (MVP: workspace `fs.read`, `fs.write`).

use std::env;
use std::fs;
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
    #[error("write too large: {0} bytes (max {1})")]
    WriteTooLarge(usize, usize),
}

const MAX_WRITE_BYTES: usize = 65_536;

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
