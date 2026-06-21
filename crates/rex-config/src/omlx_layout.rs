use std::fs;
use std::path::Path;

use crate::error::ConfigError;
use crate::paths::omlx_dir;

const CONFIG_SNIPPET: &str = include_str!("../../../templates/omlx/config.snippet.json");
const README: &str = include_str!("../../../templates/omlx/README.md");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OmlxLayoutResult {
    pub created_dir: bool,
    pub created_config_snippet: bool,
    pub created_readme: bool,
}

/// Create `$REX_ROOT/omlx/` and seed templates when missing (never overwrite).
pub fn ensure_omlx_layout() -> Result<OmlxLayoutResult, ConfigError> {
    let dir = omlx_dir();
    let mut created_dir = false;
    if !dir.exists() {
        fs::create_dir_all(&dir)
            .map_err(|err| ConfigError::Write(format!("create {}: {err}", dir.display())))?;
        created_dir = true;
    }

    let config_snippet = dir.join("config.snippet.json");
    let created_config_snippet = write_if_missing(&config_snippet, CONFIG_SNIPPET)?;

    let readme = dir.join("README.md");
    let created_readme = write_if_missing(&readme, README)?;

    Ok(OmlxLayoutResult {
        created_dir,
        created_config_snippet,
        created_readme,
    })
}

fn write_if_missing(path: &Path, contents: &str) -> Result<bool, ConfigError> {
    if path.is_file() {
        return Ok(false);
    }
    fs::write(path, contents)
        .map_err(|err| ConfigError::Write(format!("write {}: {err}", path.display())))?;
    Ok(true)
}
