use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::dispatch::{open_store, StoreEngine};
use crate::error::ObsStoreError;
use crate::port::StorePort;
use crate::query::ObsQuery;
use crate::record::StreamEconomicsRecord;

/// Thread-safe handle for non-blocking appends from async daemon code.
#[derive(Clone)]
pub struct SharedObsStore {
    inner: Arc<Mutex<StoreEngine>>,
}

impl SharedObsStore {
    pub fn open(engine: &str, path: impl AsRef<std::path::Path>) -> Result<Self, ObsStoreError> {
        Ok(Self {
            inner: Arc::new(Mutex::new(open_store(engine, path)?)),
        })
    }

    pub fn from_engine(engine: StoreEngine) -> Self {
        Self {
            inner: Arc::new(Mutex::new(engine)),
        }
    }

    fn with_store<T>(
        &self,
        f: impl FnOnce(&StoreEngine) -> Result<T, ObsStoreError>,
    ) -> Result<T, ObsStoreError> {
        let guard = self
            .inner
            .lock()
            .map_err(|_| ObsStoreError::Sqlite(rusqlite::Error::InvalidQuery))?;
        f(&guard)
    }

    pub fn path(&self) -> Result<PathBuf, ObsStoreError> {
        self.with_store(|store| Ok(store.path().to_path_buf()))
    }

    pub fn upsert_config_snapshot(
        &self,
        snapshot_id: &str,
        payload_json: &str,
    ) -> Result<(), ObsStoreError> {
        self.with_store(|store| store.upsert_config_snapshot(snapshot_id, payload_json))
    }

    pub fn append_stream(&self, record: &StreamEconomicsRecord) -> Result<(), ObsStoreError> {
        self.with_store(|store| store.append_stream(record))
    }

    pub fn stream_count(&self) -> Result<u64, ObsStoreError> {
        self.with_store(|store| store.stream_count())
    }

    pub fn query_streams(
        &self,
        filter: &crate::query::StreamQueryFilter,
    ) -> Result<Vec<crate::query::QueriedStream>, ObsStoreError> {
        self.with_store(|store| store.query_streams(filter))
    }
}
