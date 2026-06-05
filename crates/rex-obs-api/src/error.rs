use thiserror::Error;

#[derive(Debug, Error)]
pub enum ReadApiError {
    #[error("obs.read_api.bind_failed: {0}")]
    BindFailed(String),
    #[error("obs.read_api.query_invalid: {0}")]
    QueryInvalid(String),
    #[error("store error: {0}")]
    Store(#[from] rex_obs_store::ObsStoreError),
    #[error("config error: {0}")]
    Config(#[from] rex_config::ConfigError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("http error: {0}")]
    Http(String),
}
