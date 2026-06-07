use thiserror::Error;

const MACHINE_CODE_ENGINE_UNSUPPORTED: &str = "store.engine_unsupported";
const MACHINE_CODE_CHCE_NOT_READY: &str = "store.chce_not_ready";

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
    #[error("store.engine_unsupported: engine={engine}")]
    EngineUnsupported { engine: String },
    #[error("store.chce_not_ready: CHCE write/read not implemented (R047–R048)")]
    ChceNotReady,
}

impl ObsStoreError {
    /// Stable machine code for operator/daemon paths ([ERROR_HANDLING.md] store catalog).
    pub fn machine_code(&self) -> Option<&'static str> {
        match self {
            Self::EngineUnsupported { .. } => Some(MACHINE_CODE_ENGINE_UNSUPPORTED),
            Self::ChceNotReady => Some(MACHINE_CODE_CHCE_NOT_READY),
            _ => None,
        }
    }

    pub fn user_message(&self) -> String {
        if let Some(code) = self.machine_code() {
            format!("{code}: {self}")
        } else {
            self.to_string()
        }
    }
}
