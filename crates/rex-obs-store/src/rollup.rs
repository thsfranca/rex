use std::collections::BTreeMap;

use serde::Serialize;

use crate::query::QueriedStream;

/// Request body for aggregated metric rollups.
#[derive(Debug, Clone, serde::Deserialize, PartialEq, Eq)]
pub struct MetricsRollupRequest {
    pub start_ms: Option<i64>,
    pub end_ms: Option<i64>,
    /// Label keys to group by (e.g. `route`, `terminal`, `mode`).
    pub group_by: Vec<String>,
    /// Instrument names to aggregate; empty means all rollup-capable instruments.
    #[serde(default)]
    pub instruments: Vec<String>,
}

/// One aggregated bucket for a label combination.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct RollupBucket {
    pub labels: BTreeMap<String, String>,
    pub count: u64,
    pub sum_elapsed_ms: u64,
    pub sum_prompt_tokens: u64,
    pub sum_context_tokens: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct MetricsRollupResponse {
    pub buckets: Vec<RollupBucket>,
    pub cursor_commit_ms: i64,
}

/// Aggregate stream rows by the requested label keys within the time window.
pub fn rollup_metrics_by_label(
    streams: &[QueriedStream],
    request: &MetricsRollupRequest,
) -> MetricsRollupResponse {
    let cursor_commit_ms = streams.iter().map(|s| s.created_at_ms).max().unwrap_or(0);

    let filtered: Vec<&QueriedStream> = streams
        .iter()
        .filter(|s| in_time_window(s.created_at_ms, request.start_ms, request.end_ms))
        .collect();

    let mut buckets: BTreeMap<String, RollupBucket> = BTreeMap::new();

    for stream in filtered {
        let mut labels = BTreeMap::new();
        for key in &request.group_by {
            if let Some(value) = label_value(&stream.record, key) {
                labels.insert(key.clone(), value);
            }
        }
        let key = labels
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join("|");
        let entry = buckets.entry(key).or_insert_with(|| RollupBucket {
            labels: labels.clone(),
            count: 0,
            sum_elapsed_ms: 0,
            sum_prompt_tokens: 0,
            sum_context_tokens: 0,
        });
        entry.count += 1;
        entry.sum_elapsed_ms = entry
            .sum_elapsed_ms
            .saturating_add(stream.record.elapsed_ms);
        entry.sum_prompt_tokens = entry
            .sum_prompt_tokens
            .saturating_add(stream.record.prompt_tokens);
        entry.sum_context_tokens = entry
            .sum_context_tokens
            .saturating_add(stream.record.context_tokens);
    }

    MetricsRollupResponse {
        buckets: buckets.into_values().collect(),
        cursor_commit_ms,
    }
}

fn in_time_window(ts: i64, start_ms: Option<i64>, end_ms: Option<i64>) -> bool {
    if let Some(start) = start_ms {
        if ts < start {
            return false;
        }
    }
    if let Some(end) = end_ms {
        if ts > end {
            return false;
        }
    }
    true
}

fn label_value(record: &crate::record::StreamEconomicsRecord, key: &str) -> Option<String> {
    Some(match key {
        "terminal" => record.terminal.clone(),
        "route" => record.route.clone(),
        "mode" => record.mode.clone(),
        "decision" => record.cache_decision.clone(),
        "inference_runtime" => record.inference_runtime.clone(),
        "model_id" => record.model.clone(),
        "retrieval_status" => record.retrieval.clone(),
        "compression_strategy" => record.compression_strategy.clone(),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::QueriedStream;
    use crate::StreamEconomicsRecord;

    fn stream(route: &str, terminal: &str, elapsed: u64, ts: i64) -> QueriedStream {
        QueriedStream {
            record: StreamEconomicsRecord {
                snapshot_id: "snap".into(),
                request_id: 1,
                trace_id: "t".into(),
                turn_id: "".into(),
                terminal: terminal.into(),
                route: route.into(),
                cache_decision: "miss".into(),
                decision_id: "d".into(),
                inference_runtime: "mock".into(),
                mode: "ask".into(),
                model: "m".into(),
                elapsed_ms: elapsed,
                chunks_sent: 1,
                prompt_tokens: 10,
                context_tokens: 5,
                context_candidates: 0,
                context_selected: 0,
                context_truncated: false,
                retrieval: "skipped".into(),
                compression_strategy: "none".into(),
                cached_tokens: None,
                prefix_hash: None,
                parse_retries: None,
            },
            created_at_ms: ts,
        }
    }

    #[test]
    fn rollup_groups_by_route() {
        let streams = vec![
            stream("a", "done", 100, 1000),
            stream("a", "done", 200, 1001),
            stream("b", "done", 50, 1002),
        ];
        let resp = rollup_metrics_by_label(
            &streams,
            &MetricsRollupRequest {
                start_ms: None,
                end_ms: None,
                group_by: vec!["route".into()],
                instruments: vec![],
            },
        );
        assert_eq!(resp.buckets.len(), 2);
        assert_eq!(resp.cursor_commit_ms, 1002);
        let a = resp
            .buckets
            .iter()
            .find(|b| b.labels.get("route") == Some(&"a".into()))
            .unwrap();
        assert_eq!(a.count, 2);
        assert_eq!(a.sum_elapsed_ms, 300);
    }
}
