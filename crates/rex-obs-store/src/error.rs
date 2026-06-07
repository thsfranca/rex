use thiserror::Error;

const MACHINE_CODE_ENGINE_UNSUPPORTED: &str = "store.engine_unsupported";
const MACHINE_CODE_RECOVERY_FAILED: &str = "store.recovery_failed";
const MACHINE_CODE_FORMAT_VERSION_UNSUPPORTED: &str = "store.format_version_unsupported";

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
    #[error("store.recovery_failed: could not find valid CHCE page boundary")]
    RecoveryFailed,
    #[error("store.format_version_unsupported: version={version}")]
    FormatVersionUnsupported { version: u16 },
}

impl ObsStoreError {
    /// Stable machine code for operator/daemon paths ([ERROR_HANDLING.md] store catalog).
    pub fn machine_code(&self) -> Option<&'static str> {
        match self {
            Self::EngineUnsupported { .. } => Some(MACHINE_CODE_ENGINE_UNSUPPORTED),
            Self::RecoveryFailed => Some(MACHINE_CODE_RECOVERY_FAILED),
            Self::FormatVersionUnsupported { .. } => Some(MACHINE_CODE_FORMAT_VERSION_UNSUPPORTED),
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
