use std::fs;

use crate::error::ConfigError;
use crate::model::RexConfig;
use crate::paths::{global_config_path, proto_gen_path, proto_src_path, rex_root};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnsureResult {
    pub created_config: bool,
    pub created_dirs: bool,
}

/// Create missing `$REX_ROOT` layout; never overwrite existing files.
pub fn ensure_global_layout() -> Result<EnsureResult, ConfigError> {
    let root = rex_root();
    let mut created_dirs = false;

    for dir in [root.clone(), proto_src_path(), proto_gen_path()] {
        if !dir.exists() {
            fs::create_dir_all(&dir)
                .map_err(|err| ConfigError::Write(format!("create {}: {err}", dir.display())))?;
            created_dirs = true;
        }
    }

    let config_path = global_config_path();
    let created_config = if config_path.is_file() {
        false
    } else {
        let template = RexConfig::defaults();
        let json = serde_json::to_string_pretty(&template).map_err(ConfigError::Json)?;
        fs::write(&config_path, json)
            .map_err(|err| ConfigError::Write(format!("write {}: {err}", config_path.display())))?;
        true
    };

    Ok(EnsureResult {
        created_config,
        created_dirs,
    })
}
