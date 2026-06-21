use std::path::{Path, PathBuf};

use crate::error::ConfigError;
use crate::model::RexConfig;

const PROJECT_CONFIG_REL: &str = ".rex/config.json";

/// Ensure project `.rex/config.json` contains absolute `workspace.root`.
pub fn ensure_project_workspace_root(workspace_root: &Path) -> Result<PathBuf, ConfigError> {
    let canonical = canonicalize_if_possible(workspace_root.to_path_buf());
    let config_path = canonical.join(PROJECT_CONFIG_REL);
    let mut effective = if config_path.is_file() {
        let raw = std::fs::read_to_string(&config_path).map_err(ConfigError::Io)?;
        serde_json::from_str(&raw).map_err(ConfigError::Json)?
    } else {
        RexConfig {
            version: 1,
            ..RexConfig::default()
        }
    };
    let root = canonical.display().to_string();
    if effective.workspace.root.trim() != root {
        effective.workspace.root = root.clone();
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent).map_err(ConfigError::Io)?;
        }
        std::fs::write(
            &config_path,
            format!("{}\n", serde_json::to_string_pretty(&effective)?),
        )
        .map_err(ConfigError::Io)?;
    }
    Ok(canonical)
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
    use std::fs;

    #[test]
    fn writes_workspace_root_into_project_config() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = ensure_project_workspace_root(tmp.path()).expect("ensure");
        let config_path = root.join(PROJECT_CONFIG_REL);
        assert!(config_path.is_file());
        let raw = fs::read_to_string(config_path).expect("read");
        assert!(raw.contains(&root.display().to_string()));
    }
}
