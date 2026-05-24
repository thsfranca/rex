use std::env;
use std::path::PathBuf;

pub const REX_ROOT_ENV: &str = "REX_ROOT";

/// Resolved `$REX_ROOT` (explicit env or `~/.rex`).
pub fn rex_root() -> PathBuf {
    if let Ok(raw) = env::var(REX_ROOT_ENV) {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    dirs::home_dir()
        .map(|home| home.join(".rex"))
        .unwrap_or_else(|| PathBuf::from(".rex"))
}

pub fn global_config_path() -> PathBuf {
    rex_root().join("config.json")
}

pub fn proto_src_path() -> PathBuf {
    rex_root().join("proto").join("src")
}

pub fn proto_gen_path() -> PathBuf {
    rex_root().join("proto").join("gen")
}
