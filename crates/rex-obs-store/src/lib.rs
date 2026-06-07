mod chce;
mod dispatch;
mod error;
mod otel;
mod port;
mod query;
mod record;
mod rollup;
mod schema;
mod sqlite;
mod store;

pub use dispatch::{normalize_store_engine, open_store, StoreEngine, ENGINE_MMAP, ENGINE_SQLITE};
pub use error::ObsStoreError;
pub use otel::{
    instrument_catalog, project_metrics, InstrumentCatalogEntry, MetricsQueryRequest,
    MetricsQueryResponse,
};
pub use port::StorePort;
pub use query::{ObsQuery, QueriedStream, StreamQueryFilter};
pub use record::StreamEconomicsRecord;
pub use rollup::{
    rollup_metrics_by_label, MetricsRollupRequest, MetricsRollupResponse, RollupBucket,
};
pub use schema::SCHEMA_VERSION;
pub use sqlite::{ObsStore, SqliteEngine};
pub use store::SharedObsStore;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::port::StorePort;
    use tempfile::tempdir;

    fn sample_record(snapshot_id: &str, request_id: u64) -> StreamEconomicsRecord {
        StreamEconomicsRecord {
            snapshot_id: snapshot_id.to_string(),
            request_id,
            trace_id: format!("trace-{request_id}"),
            turn_id: "turn-1".to_string(),
            terminal: "done".to_string(),
            route: "sidecar+mock".to_string(),
            cache_decision: "miss_stored".to_string(),
            decision_id: format!("dec-{request_id}"),
            inference_runtime: "mock".to_string(),
            mode: "ask".to_string(),
            model: "gpt-4o-mini".to_string(),
            elapsed_ms: 42,
            chunks_sent: 3,
            prompt_tokens: 100,
            context_tokens: 50,
            context_candidates: 10,
            context_selected: 5,
            context_truncated: false,
            retrieval: "skipped".to_string(),
            compression_strategy: "extractive_query".to_string(),
            cached_tokens: None,
            prefix_hash: None,
            parse_retries: None,
        }
    }

    #[test]
    fn append_stream_requires_snapshot_fk() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("store.sqlite");
        let store = ObsStore::open(&path).unwrap();
        let err = store
            .append_stream(&sample_record("missing", 1))
            .expect_err("fk");
        assert!(matches!(err, ObsStoreError::UnknownSnapshot(_)));
    }

    #[test]
    fn upsert_and_append_two_streams() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("store.sqlite");
        let store = ObsStore::open(&path).unwrap();
        let snapshot_id = "abc123";
        store
            .upsert_config_snapshot(snapshot_id, r#"{"inference":{"runtime":"mock"}}"#)
            .unwrap();
        store.append_stream(&sample_record(snapshot_id, 1)).unwrap();
        store.append_stream(&sample_record(snapshot_id, 2)).unwrap();
        assert_eq!(store.stream_count().unwrap(), 2);
    }

    #[test]
    fn schema_has_no_prompt_columns() {
        let ddl = crate::schema::CREATE_TABLES_V1.to_ascii_lowercase();
        assert!(!ddl.contains("prompt_body"));
        assert!(!ddl.contains("file_path"));
        assert!(!ddl.contains("prompt_text"));
    }

    #[test]
    fn shared_obs_store_sqlite_round_trip() {
        let dir = tempdir().unwrap();
        let shared = SharedObsStore::open(ENGINE_SQLITE, dir.path().join("store.sqlite")).unwrap();
        shared
            .upsert_config_snapshot("snap", r#"{"inference":{"runtime":"mock"}}"#)
            .unwrap();
        shared.append_stream(&sample_record("snap", 1)).unwrap();
        assert_eq!(shared.stream_count().unwrap(), 1);
    }
}
