use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures::StreamExt;
use rex_obs_store::{open_store, project_metrics, MetricsQueryRequest, StorePort};
use rex_obs_store::{tail_telemetry, ObsQuery};

#[tokio::test]
async fn cursor_merge_skips_historical_points() {
    let dir = tempfile::tempdir().unwrap();
    let store = open_store("sqlite", dir.path().join("store.sqlite")).unwrap();
    store
        .upsert_config_snapshot("snap", r#"{"inference":{"runtime":"mock"}}"#)
        .unwrap();

    let t1 = 1_700_000_000_000_i64;
    let t2 = t1 + 500;
    let t3 = t1 + 1500;

    store.append_stream_at(&sample("snap", 1, t1), t1).unwrap();
    store.append_stream_at(&sample("snap", 2, t2), t2).unwrap();

    let historical = store.query_streams(&Default::default()).unwrap();
    let query_resp = project_metrics(
        "rex-daemon",
        &historical,
        &MetricsQueryRequest {
            start_ms: None,
            end_ms: None,
            instruments: vec!["rex.stream.requests".into()],
            labels: Default::default(),
        },
    );
    let cursor = query_resp.cursor_commit_ms.expect("cursor");

    let tail = store.tail().clone();
    let store = Arc::new(Mutex::new(store));
    let mut stream = std::pin::pin!(tail_telemetry(
        &tail,
        store.clone(),
        "rex-daemon",
        cursor,
        Some(vec!["rex.stream.requests".into()]),
    ));

    store
        .lock()
        .unwrap()
        .append_stream_at(&sample("snap", 3, t3), t3)
        .unwrap();

    let event = tokio::time::timeout(Duration::from_secs(3), stream.next())
        .await
        .expect("timed out waiting for tail event")
        .expect("tail event");
    assert!(event.timestamp_ms > cursor);
    assert_eq!(event.instrument, "rex.stream.requests");
}

fn sample(snapshot_id: &str, request_id: u64, _ts: i64) -> rex_obs_store::StreamEconomicsRecord {
    rex_obs_store::StreamEconomicsRecord {
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
        chunks_sent: 1,
        prompt_tokens: 10,
        context_tokens: 5,
        context_candidates: 1,
        context_selected: 1,
        context_truncated: false,
        retrieval: "skipped".to_string(),
        compression_strategy: "none".to_string(),
        cached_tokens: None,
        prefix_hash: None,
        parse_retries: None,
    }
}
