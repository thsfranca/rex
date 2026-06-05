use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection};

use crate::error::ObsStoreError;
use crate::record::StreamEconomicsRecord;
use crate::schema::{CREATE_TABLES_V1, SCHEMA_VERSION};

pub struct ObsStore {
    conn: Connection,
    path: PathBuf,
}

impl ObsStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, ObsStoreError> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(&path)?;
        let store = Self { conn, path };
        store.ensure_schema()?;
        Ok(store)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn schema_version(&self) -> u32 {
        SCHEMA_VERSION
    }

    pub fn ensure_schema(&self) -> Result<(), ObsStoreError> {
        self.conn.execute_batch(CREATE_TABLES_V1)?;
        Ok(())
    }

    pub fn upsert_config_snapshot(
        &self,
        snapshot_id: &str,
        payload_json: &str,
    ) -> Result<(), ObsStoreError> {
        let now = now_ms();
        self.conn.execute(
            "INSERT INTO config_snapshots (id, payload_json, created_at_ms)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(id) DO UPDATE SET payload_json = excluded.payload_json",
            (snapshot_id, payload_json, now),
        )?;
        Ok(())
    }

    pub fn append_stream(&self, record: &StreamEconomicsRecord) -> Result<(), ObsStoreError> {
        let exists: i64 = self.conn.query_row(
            "SELECT COUNT(1) FROM config_snapshots WHERE id = ?1",
            [&record.snapshot_id],
            |row| row.get(0),
        )?;
        if exists == 0 {
            return Err(ObsStoreError::UnknownSnapshot(record.snapshot_id.clone()));
        }

        let now = now_ms();
        self.conn.execute(
            "INSERT INTO streams (
                snapshot_id, request_id, trace_id, turn_id, terminal, route, cache_decision,
                decision_id, inference_runtime, mode, model, elapsed_ms, chunks_sent,
                prompt_tokens, context_tokens, context_candidates, context_selected,
                context_truncated, retrieval, compression_strategy,
                cached_tokens, prefix_hash, parse_retries, created_at_ms
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16,
                ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24
            )",
            params![
                record.snapshot_id,
                record.request_id as i64,
                record.trace_id,
                record.turn_id,
                record.terminal,
                record.route,
                record.cache_decision,
                record.decision_id,
                record.inference_runtime,
                record.mode,
                record.model,
                record.elapsed_ms as i64,
                record.chunks_sent as i64,
                record.prompt_tokens as i64,
                record.context_tokens as i64,
                record.context_candidates as i64,
                record.context_selected as i64,
                i64::from(record.context_truncated),
                record.retrieval,
                record.compression_strategy,
                record.cached_tokens.map(|v| v as i64),
                record.prefix_hash,
                record.parse_retries.map(|v| v as i64),
                now,
            ],
        )?;
        Ok(())
    }

    pub fn stream_count(&self) -> Result<u64, ObsStoreError> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(1) FROM streams", [], |row| row.get(0))?;
        Ok(count as u64)
    }

    pub fn config_snapshot_count(&self) -> Result<u64, ObsStoreError> {
        let count: i64 =
            self.conn
                .query_row("SELECT COUNT(1) FROM config_snapshots", [], |row| {
                    row.get(0)
                })?;
        Ok(count as u64)
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
