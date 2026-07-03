use std::path::PathBuf;

use thiserror::Error;

use crate::merge::LoadedConfig;

#[derive(Debug, Error, PartialEq, Eq)]
#[error("workspace root not configured (set workspace.root or enable harness cwd fallback)")]
pub struct WorkspaceRootError;

impl LoadedConfig {
    pub fn resolve_workspace_root(&self) -> Result<PathBuf, WorkspaceRootError> {
        resolve_workspace_root_for_effective(&self.effective)
    }

    pub fn workspace_root(&self) -> PathBuf {
        self.resolve_workspace_root()
            .unwrap_or_else(|_| PathBuf::new())
    }
}

pub fn resolve_workspace_root_for_effective(
    config: &crate::model::RexConfig,
) -> Result<PathBuf, WorkspaceRootError> {
    let raw = config.workspace.root.trim();
    if !raw.is_empty() && raw != "." {
        return Ok(canonicalize_if_possible(PathBuf::from(raw)));
    }
    if cwd_fallback_allowed_for_effective(config) {
        return std::env::current_dir()
            .map(canonicalize_if_possible)
            .map_err(|_| WorkspaceRootError);
    }
    Err(WorkspaceRootError)
}

fn cwd_fallback_allowed_for_effective(config: &crate::model::RexConfig) -> bool {
    config.workspace.allow_cwd_fallback.unwrap_or(false)
}

fn canonicalize_if_possible(path: PathBuf) -> PathBuf {
    if path.as_os_str().is_empty() {
        return path;
    }
    std::fs::canonicalize(&path).unwrap_or(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::RexConfig;

    fn loaded_with_root(root: &str, allow_cwd: Option<bool>) -> LoadedConfig {
        let mut cfg = RexConfig::defaults();
        cfg.workspace.root = root.to_string();
        cfg.workspace.allow_cwd_fallback = allow_cwd;
        LoadedConfig::for_test(
            PathBuf::from("/tmp/rex-test"),
            cfg,
        )
    }

    #[test]
    fn unset_root_without_flag_errors() {
        let loaded = loaded_with_root("", None);
        assert_eq!(loaded.resolve_workspace_root(), Err(WorkspaceRootError));
    }

    #[test]
    fn allow_cwd_fallback_uses_current_dir() {
        let loaded = loaded_with_root(".", Some(true));
        let root = loaded.resolve_workspace_root().expect("cwd fallback");
        let cwd = std::env::current_dir().expect("cwd");
        assert_eq!(root, canonicalize_if_possible(cwd));
    }

    #[test]
    fn explicit_path_wins() {
        let loaded = loaded_with_root("/tmp", None);
        let root = loaded.resolve_workspace_root().expect("explicit");
        assert!(root.ends_with("tmp"));
    }
}
