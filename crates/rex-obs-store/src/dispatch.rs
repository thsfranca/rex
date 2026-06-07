use std::path::Path;

use crate::chce::ChceEngine;
use crate::error::ObsStoreError;
use crate::port::StorePort;
use crate::query::{ObsQuery, QueriedStream, StreamQueryFilter};
use crate::record::StreamEconomicsRecord;
use crate::sqlite::SqliteEngine;

pub const ENGINE_SQLITE: &str = "sqlite";
pub const ENGINE_MMAP: &str = "mmap";

/// Opened economics store backend selected by `observability.store.engine`.
#[derive(Debug)]
pub enum StoreEngine {
    Sqlite(SqliteEngine),
    Chce(ChceEngine),
}

impl StoreEngine {
    pub fn is_chce(&self) -> bool {
        matches!(self, Self::Chce(_))
    }
}

pub fn normalize_store_engine(engine: &str) -> String {
    let trimmed = engine.trim().to_ascii_lowercase();
    match trimmed.as_str() {
        "" | ENGINE_SQLITE => ENGINE_SQLITE.to_string(),
        ENGINE_MMAP => ENGINE_MMAP.to_string(),
        other => other.to_string(),
    }
}

pub fn open_store(engine: &str, path: impl AsRef<Path>) -> Result<StoreEngine, ObsStoreError> {
    match normalize_store_engine(engine).as_str() {
        ENGINE_SQLITE => Ok(StoreEngine::Sqlite(SqliteEngine::open(path)?)),
        ENGINE_MMAP => open_mmap_engine(path),
        other => Err(ObsStoreError::EngineUnsupported {
            engine: other.to_string(),
        }),
    }
}

fn open_mmap_engine(path: impl AsRef<Path>) -> Result<StoreEngine, ObsStoreError> {
    #[cfg(target_os = "macos")]
    {
        Ok(StoreEngine::Chce(ChceEngine::open(path)?))
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = path;
        Err(ObsStoreError::EngineUnsupported {
            engine: ENGINE_MMAP.to_string(),
        })
    }
}

impl StorePort for StoreEngine {
    fn path(&self) -> &Path {
        match self {
            Self::Sqlite(engine) => engine.path(),
            Self::Chce(engine) => engine.path(),
        }
    }

    fn upsert_config_snapshot(
        &self,
        snapshot_id: &str,
        payload_json: &str,
    ) -> Result<(), ObsStoreError> {
        match self {
            Self::Sqlite(engine) => engine.upsert_config_snapshot(snapshot_id, payload_json),
            Self::Chce(engine) => engine.upsert_config_snapshot(snapshot_id, payload_json),
        }
    }

    fn append_stream(&self, record: &StreamEconomicsRecord) -> Result<(), ObsStoreError> {
        match self {
            Self::Sqlite(engine) => engine.append_stream(record),
            Self::Chce(engine) => engine.append_stream(record),
        }
    }

    fn stream_count(&self) -> Result<u64, ObsStoreError> {
        match self {
            Self::Sqlite(engine) => engine.stream_count(),
            Self::Chce(engine) => engine.stream_count(),
        }
    }
}

impl ObsQuery for StoreEngine {
    fn query_streams(
        &self,
        filter: &StreamQueryFilter,
    ) -> Result<Vec<QueriedStream>, ObsStoreError> {
        match self {
            Self::Sqlite(engine) => engine.query_streams(filter),
            Self::Chce(engine) => engine.query_streams(filter),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn mmap_engine_unsupported_on_non_macos() {
        let dir = tempfile::tempdir().unwrap();
        let err = open_store(ENGINE_MMAP, dir.path().join("store.rexobs")).expect_err("mmap");
        assert!(matches!(err, ObsStoreError::EngineUnsupported { .. }));
        assert_eq!(err.machine_code(), Some("store.engine_unsupported"));
    }

    #[test]
    fn unknown_engine_unsupported() {
        let dir = tempfile::tempdir().unwrap();
        let err = open_store("rocksdb", dir.path().join("store.db")).expect_err("unknown");
        assert!(matches!(
            err,
            ObsStoreError::EngineUnsupported {
                ref engine,
            } if engine == "rocksdb"
        ));
        assert_eq!(err.machine_code(), Some("store.engine_unsupported"));
    }

    #[test]
    fn sqlite_engine_default_path_opens() {
        let dir = tempfile::tempdir().unwrap();
        let engine = open_store("sqlite", dir.path().join("store.sqlite")).unwrap();
        assert!(!engine.is_chce());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn mmap_selects_chce_on_macos() {
        let dir = tempfile::tempdir().unwrap();
        let engine = open_store(ENGINE_MMAP, dir.path().join("store.rexobs")).unwrap();
        assert!(engine.is_chce());
    }
}
