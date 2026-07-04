use std::path::PathBuf;

use thiserror::Error;

use crate::merge::LoadedConfig;

#[derive(Debug, Error, PartialEq, Eq)]
#[error("could not resolve workspace from current working directory")]
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
    _config: &crate::model::RexConfig,
) -> Result<PathBuf, WorkspaceRootError> {
    std::env::current_dir()
        .map(canonicalize_if_possible)
        .map_err(|_| WorkspaceRootError)
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
    use serial_test::serial;

    fn loaded_for_test() -> LoadedConfig {
        LoadedConfig::for_test(PathBuf::from("/tmp/rex-test"), RexConfig::defaults())
    }

    #[test]
    #[serial]
    fn resolves_to_current_dir() {
        if std::env::current_dir().is_err() {
            std::env::set_current_dir(std::env::temp_dir()).expect("recover cwd");
        }
        let loaded = loaded_for_test();
        let root = loaded.resolve_workspace_root().expect("cwd");
        let cwd = std::env::current_dir().expect("cwd");
        assert_eq!(root, canonicalize_if_possible(cwd));
    }
}
