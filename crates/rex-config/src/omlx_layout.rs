use std::fs;
use std::path::Path;

use crate::error::ConfigError;
use crate::paths::omlx_dir;

const ENV_EXAMPLE: &str = include_str!("../../../templates/omlx/.env.example");
const README: &str = include_str!("../../../templates/omlx/README.md");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OmlxLayoutResult {
    pub created_dir: bool,
    pub created_env_example: bool,
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

    let env_example = dir.join(".env.example");
    let created_env_example = write_if_missing(&env_example, ENV_EXAMPLE)?;

    let readme = dir.join("README.md");
    let created_readme = write_if_missing(&readme, README)?;

    Ok(OmlxLayoutResult {
        created_dir,
        created_env_example,
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
