use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tokio::time::sleep;

use crate::otel::{project_stream_tail_point, TailPointFilter};
use crate::query::QueriedStream;
use crate::query::{ObsQuery, StreamQueryFilter};
use crate::record::StreamEconomicsRecord;
use crate::StoreEngine;

const BROADCAST_CAPACITY: usize = 1024;
const SQLITE_POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(100);

/// One live telemetry point for SSE tail consumers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TailTelemetryEvent {
    pub timestamp_ms: i64,
    pub instrument: String,
    #[serde(rename = "dataPoint")]
    pub data_point: serde_json::Value,
}

/// Fan-out hub for live tail subscribers (CHCE ring + SQLite append notify).
#[derive(Clone, Debug)]
pub struct TelemetryTail {
    tx: broadcast::Sender<TailTelemetryEvent>,
}

impl TelemetryTail {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(BROADCAST_CAPACITY);
        Self { tx }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<TailTelemetryEvent> {
        self.tx.subscribe()
    }

    pub fn publish(&self, event: TailTelemetryEvent) {
        let _ = self.tx.send(event);
    }

    pub fn publish_stream(&self, stream: &QueriedStream, service_name: &str) {
        for event in stream_to_tail_events(stream, service_name, None) {
            self.publish(event);
        }
    }
}

impl Default for TelemetryTail {
    fn default() -> Self {
        Self::new()
    }
}

/// Subscribe to live tail events newer than `cursor_commit_ms`.
pub fn tail_telemetry(
    tail: &TelemetryTail,
    store: Arc<Mutex<StoreEngine>>,
    service_name: &str,
    cursor_commit_ms: i64,
    instruments: Option<Vec<String>>,
) -> impl futures::Stream<Item = TailTelemetryEvent> + use<> {
    let filter = TailPointFilter {
        instruments: instruments.clone(),
    };
    let mut rx = tail.subscribe();
    let service_name = service_name.to_string();

    async_stream::stream! {
        let mut last_sqlite_poll_ms = cursor_commit_ms;
        loop {
            tokio::select! {
                recv = rx.recv() => {
                    match recv {
                        Ok(event) if event.timestamp_ms > cursor_commit_ms
                            && instrument_allowed(&event.instrument, instruments.as_ref()) => {
                            yield event;
                        }
                        Ok(_) => {}
                        Err(broadcast::error::RecvError::Lagged(_)) => {}
                        Err(broadcast::error::RecvError::Closed) => break,
                    }
                }
                _ = sleep(SQLITE_POLL_INTERVAL) => {
                    let poll_from = last_sqlite_poll_ms;
                    match poll_sqlite_tail(&store, poll_from, &service_name, &filter) {
                        Ok(events) => {
                            for event in events {
                                if event.timestamp_ms > last_sqlite_poll_ms {
                                    last_sqlite_poll_ms = event.timestamp_ms;
                                }
                                if event.timestamp_ms > cursor_commit_ms
                                    && instrument_allowed(&event.instrument, instruments.as_ref()) {
                                    yield event;
                                }
                            }
                        }
                        Err(err) => {
                            eprintln!("obs.tail=degraded reason=sqlite_poll error={err}");
                        }
                    }
                }
            }
        }
    }
}

fn instrument_allowed(name: &str, instruments: Option<&Vec<String>>) -> bool {
    match instruments {
        None => true,
        Some(list) if list.is_empty() => true,
        Some(list) => list.iter().any(|want| want == name),
    }
}

fn stream_to_tail_events(
    stream: &QueriedStream,
    service_name: &str,
    instruments: Option<&Vec<String>>,
) -> Vec<TailTelemetryEvent> {
    let filter = TailPointFilter {
        instruments: instruments.cloned(),
    };
    project_stream_tail_point(stream, service_name, &filter)
}

fn poll_sqlite_tail(
    store: &Arc<Mutex<StoreEngine>>,
    after_ms: i64,
    service_name: &str,
    filter: &TailPointFilter,
) -> Result<Vec<TailTelemetryEvent>, crate::ObsStoreError> {
    let guard = store
        .lock()
        .map_err(|_| crate::ObsStoreError::Sqlite(rusqlite::Error::InvalidQuery))?;
    let rows = guard.query_streams(&StreamQueryFilter {
        start_ms: Some(after_ms.saturating_add(1)),
        ..Default::default()
    })?;
    Ok(rows
        .iter()
        .flat_map(|row| project_stream_tail_point(row, service_name, filter))
        .collect())
}

pub fn queried_from_record(
    record: StreamEconomicsRecord,
    created_at_ms: i64,
) -> crate::query::QueriedStream {
    crate::query::QueriedStream {
        record,
        created_at_ms,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::open_store;
    use crate::port::StorePort;
    use futures::StreamExt;
    use std::sync::{Arc, Mutex};

    fn sample(snapshot_id: &str, request_id: u64, _created_at_ms: i64) -> StreamEconomicsRecord {
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

    #[tokio::test]
    async fn tail_skips_events_at_or_before_cursor() {
        let dir = tempfile::tempdir().unwrap();
        let store = open_store("sqlite", dir.path().join("store.sqlite")).unwrap();
        let tail = store.tail().clone();
        let store = Arc::new(Mutex::new(store));
        store
            .lock()
            .unwrap()
            .upsert_config_snapshot("snap", r#"{"inference":{"runtime":"mock"}}"#)
            .unwrap();

        let t1 = 1_700_000_000_000_i64;
        let t2 = t1 + 1000;
        store
            .lock()
            .unwrap()
            .append_stream_at(&sample("snap", 1, t1), t1)
            .unwrap();
        store
            .lock()
            .unwrap()
            .append_stream_at(&sample("snap", 2, t2), t2)
            .unwrap();

        let cursor = t1;
        let mut stream = std::pin::pin!(tail_telemetry(
            &tail,
            Arc::clone(&store),
            "rex-daemon",
            cursor,
            Some(vec!["rex.stream.requests".into()]),
        ));

        let first = tokio::time::timeout(std::time::Duration::from_secs(2), stream.next())
            .await
            .expect("timeout")
            .expect("event");
        assert!(first.timestamp_ms > cursor);
        assert_eq!(first.instrument, "rex.stream.requests");
    }
}
