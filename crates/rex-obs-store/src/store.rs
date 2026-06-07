use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::dispatch::{open_store, StoreEngine};
use crate::error::ObsStoreError;
use crate::port::StorePort;
use crate::query::ObsQuery;
use crate::record::{SidecarMetricDef, SpanRecord, StreamEconomicsRecord};
use crate::tail::{queried_from_record, TelemetryTail};

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

    pub fn tail(&self) -> Result<Arc<TelemetryTail>, ObsStoreError> {
        self.with_store(|store| Ok(Arc::clone(store.tail())))
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

    pub fn append_span(&self, span: &SpanRecord) -> Result<(), ObsStoreError> {
        self.with_store(|store| store.append_span(span))
    }

    pub fn register_sidecar_metric(&self, def: &SidecarMetricDef) -> Result<(), ObsStoreError> {
        self.with_store(|store| store.register_sidecar_metric(def))
    }

    pub fn list_sidecar_metrics(&self) -> Result<Vec<SidecarMetricDef>, ObsStoreError> {
        self.with_store(|store| store.list_sidecar_metrics())
    }

    pub fn publish_stream_tail(
        &self,
        record: &StreamEconomicsRecord,
        created_at_ms: i64,
        service_name: &str,
    ) -> Result<(), ObsStoreError> {
        self.with_store(|store| {
            store.tail().publish_stream(
                &queried_from_record(record.clone(), created_at_ms),
                service_name,
            );
            Ok(())
        })
    }
}
