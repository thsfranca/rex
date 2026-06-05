use rusqlite::{Connection, Row};

use crate::error::ObsStoreError;
use crate::record::StreamEconomicsRecord;

/// Filters for historical stream queries. Engine-agnostic logical contract.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StreamQueryFilter {
    pub start_ms: Option<i64>,
    pub end_ms: Option<i64>,
    pub terminal: Option<String>,
    pub route: Option<String>,
    pub mode: Option<String>,
    pub cache_decision: Option<String>,
}

/// One persisted stream row plus store timestamp.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueriedStream {
    pub record: StreamEconomicsRecord,
    pub created_at_ms: i64,
}

/// Engine-agnostic read contract for observability stores.
pub trait ObsQuery {
    fn query_streams(
        &self,
        filter: &StreamQueryFilter,
    ) -> Result<Vec<QueriedStream>, ObsStoreError>;
}

pub(crate) fn query_streams_impl(
    conn: &Connection,
    filter: &StreamQueryFilter,
) -> Result<Vec<QueriedStream>, ObsStoreError> {
    let mut sql = String::from(
        "SELECT snapshot_id, request_id, trace_id, turn_id, terminal, route, cache_decision,
                decision_id, inference_runtime, mode, model, elapsed_ms, chunks_sent,
                prompt_tokens, context_tokens, context_candidates, context_selected,
                context_truncated, retrieval, compression_strategy,
                cached_tokens, prefix_hash, parse_retries, created_at_ms
         FROM streams WHERE 1=1",
    );
    let mut values: Vec<rusqlite::types::Value> = Vec::new();

    if let Some(start) = filter.start_ms {
        sql.push_str(" AND created_at_ms >= ?");
        values.push(start.into());
    }
    if let Some(end) = filter.end_ms {
        sql.push_str(" AND created_at_ms <= ?");
        values.push(end.into());
    }
    if let Some(terminal) = filter.terminal.as_ref() {
        sql.push_str(" AND terminal = ?");
        values.push(terminal.clone().into());
    }
    if let Some(route) = filter.route.as_ref() {
        sql.push_str(" AND route = ?");
        values.push(route.clone().into());
    }
    if let Some(mode) = filter.mode.as_ref() {
        sql.push_str(" AND mode = ?");
        values.push(mode.clone().into());
    }
    if let Some(cache) = filter.cache_decision.as_ref() {
        sql.push_str(" AND cache_decision = ?");
        values.push(cache.clone().into());
    }
    sql.push_str(" ORDER BY created_at_ms ASC");

    let mut stmt = conn.prepare(&sql)?;
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = values
        .iter()
        .map(|v| v as &dyn rusqlite::types::ToSql)
        .collect();
    let rows = stmt.query_map(param_refs.as_slice(), row_to_queried_stream)?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(ObsStoreError::from)
}

fn row_to_queried_stream(row: &Row<'_>) -> Result<QueriedStream, rusqlite::Error> {
    let context_truncated: i64 = row.get(17)?;
    let cached_tokens: Option<i64> = row.get(20)?;
    let parse_retries: Option<i64> = row.get(22)?;
    Ok(QueriedStream {
        record: StreamEconomicsRecord {
            snapshot_id: row.get(0)?,
            request_id: row.get::<_, i64>(1)? as u64,
            trace_id: row.get(2)?,
            turn_id: row.get(3)?,
            terminal: row.get(4)?,
            route: row.get(5)?,
            cache_decision: row.get(6)?,
            decision_id: row.get(7)?,
            inference_runtime: row.get(8)?,
            mode: row.get(9)?,
            model: row.get(10)?,
            elapsed_ms: row.get::<_, i64>(11)? as u64,
            chunks_sent: row.get::<_, i64>(12)? as u64,
            prompt_tokens: row.get::<_, i64>(13)? as u64,
            context_tokens: row.get::<_, i64>(14)? as u64,
            context_candidates: row.get::<_, i64>(15)? as u64,
            context_selected: row.get::<_, i64>(16)? as u64,
            context_truncated: context_truncated != 0,
            retrieval: row.get(18)?,
            compression_strategy: row.get(19)?,
            cached_tokens: cached_tokens.map(|v| v as u64),
            prefix_hash: row.get(21)?,
            parse_retries: parse_retries.map(|v| v as u64),
        },
        created_at_ms: row.get(23)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ObsStore;

    fn sample(snapshot_id: &str, request_id: u64, terminal: &str) -> StreamEconomicsRecord {
        StreamEconomicsRecord {
            snapshot_id: snapshot_id.to_string(),
            request_id,
            trace_id: format!("trace-{request_id}"),
            turn_id: "turn-1".to_string(),
            terminal: terminal.to_string(),
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
    fn query_filters_terminal() {
        let dir = tempfile::tempdir().unwrap();
        let store = ObsStore::open(dir.path().join("s.sqlite")).unwrap();
        store
            .upsert_config_snapshot("snap", r#"{"inference":{"runtime":"mock"}}"#)
            .unwrap();
        store.append_stream(&sample("snap", 1, "done")).unwrap();
        store
            .append_stream(&sample("snap", 2, "grpc_error"))
            .unwrap();

        let rows = store
            .query_streams(&StreamQueryFilter {
                terminal: Some("done".to_string()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].record.terminal, "done");
    }
}
