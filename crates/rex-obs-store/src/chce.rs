use std::path::{Path, PathBuf};

use crate::error::ObsStoreError;
use crate::port::StorePort;
use crate::query::{ObsQuery, QueriedStream, StreamQueryFilter};
use crate::record::StreamEconomicsRecord;

/// CHCE mmap engine stub — selected on macOS when `engine=mmap`; write/read land in R047–R048.
#[derive(Debug)]
pub struct ChceEngine {
    path: PathBuf,
}

impl ChceEngine {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, ObsStoreError> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        Ok(Self { path })
    }

    fn not_ready() -> ObsStoreError {
        ObsStoreError::ChceNotReady
    }
}

impl StorePort for ChceEngine {
    fn path(&self) -> &Path {
        &self.path
    }

    fn upsert_config_snapshot(
        &self,
        _snapshot_id: &str,
        _payload_json: &str,
    ) -> Result<(), ObsStoreError> {
        Err(Self::not_ready())
    }

    fn append_stream(&self, _record: &StreamEconomicsRecord) -> Result<(), ObsStoreError> {
        Err(Self::not_ready())
    }

    fn stream_count(&self) -> Result<u64, ObsStoreError> {
        Err(Self::not_ready())
    }
}

impl ObsQuery for ChceEngine {
    fn query_streams(
        &self,
        _filter: &StreamQueryFilter,
    ) -> Result<Vec<QueriedStream>, ObsStoreError> {
        Err(Self::not_ready())
    }
}
