use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to read config: {0}")]
    Read(String),
    #[error("failed to write config: {0}")]
    Write(String),
    #[error("invalid config JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("config validation: {0}")]
    Validation(String),
    #[error("home directory unavailable for default REX_ROOT")]
    NoHomeDir,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
