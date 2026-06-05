use std::fs;
use std::path::Path;

use crate::error::ConfigError;
use crate::paths::gateway_dir;

const CONFIG_TEMPLATE: &str = include_str!("../../../templates/gateway/config.yaml");
const ENV_EXAMPLE: &str = include_str!("../../../templates/gateway/.env.example");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GatewayLayoutResult {
    pub created_dir: bool,
    pub created_config: bool,
    pub created_env_example: bool,
}

/// Create `$REX_ROOT/gateway/` and seed templates when missing (never overwrite).
pub fn ensure_gateway_layout() -> Result<GatewayLayoutResult, ConfigError> {
    let dir = gateway_dir();
    let mut created_dir = false;
    if !dir.exists() {
        fs::create_dir_all(&dir)
            .map_err(|err| ConfigError::Write(format!("create {}: {err}", dir.display())))?;
        created_dir = true;
    }

    let config_path = dir.join("config.yaml");
    let created_config = write_if_missing(&config_path, CONFIG_TEMPLATE)?;

    let env_example = dir.join(".env.example");
    let created_env_example = write_if_missing(&env_example, ENV_EXAMPLE)?;

    Ok(GatewayLayoutResult {
        created_dir,
        created_config,
        created_env_example,
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
