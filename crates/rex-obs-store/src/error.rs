use thiserror::Error;

#[derive(Debug, Error)]
pub enum ObsStoreError {
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("unknown config snapshot id: {0}")]
    UnknownSnapshot(String),
}
