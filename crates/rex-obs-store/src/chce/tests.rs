use crate::chce::page::{MmapPaginator, SealedPage, MAGIC, PAGE_SIZE};
use crate::chce::ChceEngine;
use crate::dispatch::{open_store, ENGINE_MMAP, ENGINE_SQLITE};
use crate::port::StorePort;
use crate::query::{ObsQuery, StreamQueryFilter};
use crate::record::StreamEconomicsRecord;
use crate::sqlite::SqliteEngine;
use tempfile::tempdir;

fn fixture_records(snapshot_id: &str) -> Vec<StreamEconomicsRecord> {
    vec![
        StreamEconomicsRecord {
            snapshot_id: snapshot_id.to_string(),
            request_id: 1,
            trace_id: "trace-1".to_string(),
            turn_id: "turn-1".to_string(),
            terminal: "done".to_string(),
            route: "sidecar+mock".to_string(),
            cache_decision: "miss_stored".to_string(),
            decision_id: "dec-1".to_string(),
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
        },
        StreamEconomicsRecord {
            snapshot_id: snapshot_id.to_string(),
            request_id: 2,
            trace_id: "trace-2".to_string(),
            turn_id: "turn-2".to_string(),
            terminal: "grpc_error".to_string(),
            route: "sidecar+mock".to_string(),
            cache_decision: "hit".to_string(),
            decision_id: "dec-2".to_string(),
            inference_runtime: "mock".to_string(),
            mode: "agent".to_string(),
            model: "gpt-4o".to_string(),
            elapsed_ms: 128,
            chunks_sent: 7,
            prompt_tokens: 220,
            context_tokens: 80,
            context_candidates: 20,
            context_selected: 12,
            context_truncated: true,
            retrieval: "vector".to_string(),
            compression_strategy: "none".to_string(),
            cached_tokens: Some(64),
            prefix_hash: Some("hash-2".to_string()),
            parse_retries: Some(2),
        },
    ]
}

fn load_fixture_into(store: &dyn StorePort, snapshot_id: &str) {
    store
        .upsert_config_snapshot(snapshot_id, r#"{"inference":{"runtime":"mock"}}"#)
        .unwrap();
    for record in fixture_records(snapshot_id) {
        store.append_stream(&record).unwrap();
    }
}

#[cfg(target_os = "macos")]
#[test]
fn sqlite_vs_mmap_fixture_parity() {
    let dir = tempdir().unwrap();
    let snapshot_id = "fixture-snap";

    let sqlite = SqliteEngine::open(dir.path().join("store.sqlite")).unwrap();
    load_fixture_into(&sqlite, snapshot_id);

    let mmap = ChceEngine::open(dir.path().join("obs/store.rexobs")).unwrap();
    load_fixture_into(&mmap, snapshot_id);
    mmap.drain_for_test().unwrap();

    let sqlite_rows = sqlite.query_streams(&StreamQueryFilter::default()).unwrap();
    let mmap_rows = mmap.query_streams(&StreamQueryFilter::default()).unwrap();

    assert_eq!(sqlite.stream_count().unwrap(), mmap.stream_count().unwrap());
    assert_eq!(sqlite_rows.len(), mmap_rows.len());

    for (sqlite_row, mmap_row) in sqlite_rows.iter().zip(mmap_rows.iter()) {
        assert_eq!(sqlite_row.record, mmap_row.record);
    }

    let terminal_filter = StreamQueryFilter {
        terminal: Some("done".to_string()),
        ..Default::default()
    };
    assert_eq!(
        sqlite.query_streams(&terminal_filter).unwrap().len(),
        mmap.query_streams(&terminal_filter).unwrap().len()
    );
}

#[cfg(target_os = "macos")]
#[test]
fn mmap_store_dict_artifact_created() {
    let dir = tempdir().unwrap();
    let engine = ChceEngine::open(dir.path().join("obs/store.rexobs")).unwrap();
    engine
        .upsert_config_snapshot("snap", r#"{"inference":{"runtime":"mock"}}"#)
        .unwrap();
    engine.append_stream(&fixture_records("snap")[0]).unwrap();
    engine.drain_for_test().unwrap();

    assert!(dir.path().join("obs/store.dict").is_file());
    assert!(dir.path().join("obs/store.rexobs").is_file());
}

#[cfg(target_os = "macos")]
#[test]
fn recovery_truncates_torn_tail_page() {
    use crate::chce::dict::DictionaryManager;
    use crate::chce::page::ColumnarCodec;
    use crate::chce::ring::StreamAppendEvent;

    let dir = tempdir().unwrap();
    let rexobs = dir.path().join("store.rexobs");
    let mut paginator = MmapPaginator::open(rexobs.clone()).unwrap();
    let mut dict = DictionaryManager::open(dir.path().join("store.dict")).unwrap();

    let event = StreamAppendEvent {
        record: fixture_records("snap")[0].clone(),
        created_at_ms: 1_700_000_000_000,
    };
    let encoded = ColumnarCodec::encode_batch(&[event], &mut dict).unwrap();
    let page = SealedPage::seal(encoded).unwrap();
    paginator.append_page(&page).unwrap();

    let committed = paginator.committed_byte_len();
    let mut bytes = std::fs::read(&rexobs).unwrap();
    bytes.truncate(committed as usize);
    let mut torn_page = vec![0_u8; PAGE_SIZE];
    torn_page[0..4].copy_from_slice(&MAGIC);
    torn_page[4..6].copy_from_slice(&1u16.to_le_bytes());
    torn_page[6..8].copy_from_slice(&(PAGE_SIZE as u16).to_le_bytes());
    bytes.extend_from_slice(&torn_page);
    bytes.extend_from_slice(&(committed + PAGE_SIZE as u64).to_le_bytes());
    std::fs::write(&rexobs, bytes).unwrap();

    let recovered = MmapPaginator::open(rexobs).unwrap();
    assert_eq!(recovered.committed_byte_len(), committed);
    assert_eq!(recovered.total_record_count().unwrap(), 1);
}

#[cfg(target_os = "macos")]
#[test]
fn mmap_open_via_dispatch_writes_and_reads() {
    let dir = tempdir().unwrap();
    let engine = open_store(ENGINE_MMAP, dir.path().join("obs/store.rexobs")).unwrap();
    load_fixture_into(&engine, "snap");
    if let crate::dispatch::StoreEngine::Chce(chce) = engine {
        chce.drain_for_test().unwrap();
        assert_eq!(chce.stream_count().unwrap(), 2);
    } else {
        panic!("expected CHCE engine");
    }
}

#[test]
fn sqlite_dispatch_still_default() {
    let dir = tempdir().unwrap();
    let engine = open_store(ENGINE_SQLITE, dir.path().join("store.sqlite")).unwrap();
    assert!(!engine.is_chce());
}

#[cfg(not(target_os = "macos"))]
#[test]
fn mmap_engine_unsupported_off_macos_in_chce_tests() {
    use crate::error::ObsStoreError;

    let dir = tempdir().unwrap();
    let err = open_store(ENGINE_MMAP, dir.path().join("store.rexobs")).expect_err("mmap");
    assert!(matches!(err, ObsStoreError::EngineUnsupported { .. }));
}
