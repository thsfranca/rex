//! Durable harness session event log (ADR 0040).

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rex_proto::rex::v1::{SessionEvent, StreamInferenceResponse};

const SESSIONS_DIR: &str = "sessions";
const DEFAULT_FETCH_LIMIT: u32 = 50;
const MAX_FETCH_LIMIT: u32 = 200;

#[derive(Debug, thiserror::Error)]
pub enum SessionStoreError {
    #[error("invalid harness session id")]
    InvalidSessionId,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("workspace root not configured")]
    WorkspaceNotConfigured,
}

pub struct HarnessSessionStore {
    inner: Mutex<()>,
}

impl Default for HarnessSessionStore {
    fn default() -> Self {
        Self::new()
    }
}

impl HarnessSessionStore {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(()),
        }
    }

    pub fn append_stream_chunk(
        &self,
        harness_session_id: &str,
        chunk: &StreamInferenceResponse,
    ) -> Result<(), SessionStoreError> {
        let session_id = harness_session_id.trim();
        if session_id.is_empty() {
            return Ok(());
        }
        let path = self.session_log_path(session_id)?;
        let _guard = self.inner.lock().expect("session store mutex");
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let sequence = next_sequence(&path)?;
        let record = StoredEvent::from_chunk(sequence, chunk);
        let line = serde_json::to_string(&record).map_err(|err| {
            SessionStoreError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                err.to_string(),
            ))
        })?;
        let mut file = OpenOptions::new().create(true).append(true).open(&path)?;
        writeln!(file, "{line}")?;
        Ok(())
    }

    pub fn append_operator_prompt(
        &self,
        harness_session_id: &str,
        prompt: &str,
        turn_id: &str,
    ) -> Result<(), SessionStoreError> {
        let chunk = StreamInferenceResponse {
            event: "operator_prompt".to_string(),
            text: prompt.to_string(),
            turn_id: turn_id.to_string(),
            ..Default::default()
        };
        self.append_stream_chunk(harness_session_id, &chunk)
    }

    pub fn fetch_events(
        &self,
        harness_session_id: &str,
        before_sequence: u64,
        after_sequence: u64,
        limit: u32,
    ) -> Result<FetchPage, SessionStoreError> {
        let session_id = harness_session_id.trim();
        if session_id.is_empty() {
            return Ok(FetchPage::default());
        }
        let path = self.session_log_path(session_id)?;
        let _guard = self.inner.lock().expect("session store mutex");
        if !path.is_file() {
            return Ok(FetchPage::default());
        }
        let limit = normalize_limit(limit);
        let all = read_all_events(&path)?;
        let head_sequence = all.last().map(|e| e.sequence).unwrap_or(0);

        if after_sequence > 0 {
            let events: Vec<SessionEvent> = all
                .iter()
                .filter(|e| e.sequence > after_sequence)
                .take(limit as usize)
                .map(stored_to_proto)
                .collect();
            let has_more_after = all
                .iter()
                .filter(|e| e.sequence > after_sequence)
                .count()
                > events.len();
            return Ok(FetchPage {
                events,
                has_more_before: false,
                has_more_after,
                head_sequence,
            });
        }

        if before_sequence > 0 {
            let older: Vec<&StoredEvent> = all
                .iter()
                .filter(|e| e.sequence < before_sequence)
                .collect();
            let start = older.len().saturating_sub(limit as usize);
            let slice = &older[start..];
            let has_more_before = start > 0;
            let events = slice.iter().map(|e| stored_to_proto(e)).collect();
            return Ok(FetchPage {
                events,
                has_more_before,
                has_more_after: false,
                head_sequence,
            });
        }

        let start = all.len().saturating_sub(limit as usize);
        let events = all[start..].iter().map(stored_to_proto).collect();
        Ok(FetchPage {
            events,
            has_more_before: start > 0,
            has_more_after: false,
            head_sequence,
        })
    }

    pub fn history_prefix_for_prompt(&self, harness_session_id: &str) -> Result<String, SessionStoreError> {
        let page = self.fetch_events(harness_session_id, 0, 0, MAX_FETCH_LIMIT)?;
        if page.events.is_empty() {
            return Ok(String::new());
        }
        let mut lines = vec!["[session_history]".to_string()];
        for event in page.events {
            match event.event.as_str() {
                "operator_prompt" => lines.push(format!("User: {}", event.text.trim())),
                "chunk" if !event.text.is_empty() => lines.push(format!("Assistant: {}", event.text.trim())),
                "done" => {}
                _ => {}
            }
        }
        if lines.len() <= 1 {
            return Ok(String::new());
        }
        Ok(lines.join("\n"))
    }

    fn session_log_path(&self, harness_session_id: &str) -> Result<PathBuf, SessionStoreError> {
        let workspace = crate::settings::get()
            .resolve_workspace_root()
            .map_err(|_| SessionStoreError::WorkspaceNotConfigured)?;
        let safe = sanitize_session_id(harness_session_id)?;
        Ok(workspace.join(".rex").join(SESSIONS_DIR).join(format!("{safe}.jsonl")))
    }
}

#[derive(Debug, Default)]
pub struct FetchPage {
    pub events: Vec<SessionEvent>,
    pub has_more_before: bool,
    pub has_more_after: bool,
    pub head_sequence: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct StoredEvent {
    sequence: u64,
    event: String,
    text: String,
    turn_id: String,
    tool_name: String,
    phase: String,
    summary: String,
    detail: String,
    tool_call_id: String,
    elapsed_ms: u64,
    done: bool,
}

impl StoredEvent {
    fn from_chunk(sequence: u64, chunk: &StreamInferenceResponse) -> Self {
        Self {
            sequence,
            event: chunk.event.clone(),
            text: chunk.text.clone(),
            turn_id: chunk.turn_id.clone(),
            tool_name: chunk.tool_name.clone(),
            phase: chunk.phase.clone(),
            summary: chunk.summary.clone(),
            detail: chunk.detail.clone(),
            tool_call_id: chunk.tool_call_id.clone(),
            elapsed_ms: chunk.elapsed_ms,
            done: chunk.done,
        }
    }
}

fn sanitize_session_id(raw: &str) -> Result<String, SessionStoreError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.len() > 128 {
        return Err(SessionStoreError::InvalidSessionId);
    }
    if !trimmed
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(SessionStoreError::InvalidSessionId);
    }
    Ok(trimmed.to_string())
}

fn normalize_limit(limit: u32) -> u32 {
    if limit == 0 {
        DEFAULT_FETCH_LIMIT
    } else {
        limit.min(MAX_FETCH_LIMIT)
    }
}

fn next_sequence(path: &Path) -> Result<u64, SessionStoreError> {
    let all = read_all_events(path)?;
    Ok(all.last().map(|e| e.sequence + 1).unwrap_or(1))
}

fn read_all_events(path: &Path) -> Result<Vec<StoredEvent>, SessionStoreError> {
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut out = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(event) = serde_json::from_str::<StoredEvent>(&line) {
            out.push(event);
        }
    }
    Ok(out)
}

fn stored_to_proto(stored: &StoredEvent) -> SessionEvent {
    SessionEvent {
        sequence: stored.sequence,
        event: stored.event.clone(),
        text: stored.text.clone(),
        turn_id: stored.turn_id.clone(),
        tool_name: stored.tool_name.clone(),
        phase: stored.phase.clone(),
        summary: stored.summary.clone(),
        detail: stored.detail.clone(),
        tool_call_id: stored.tool_call_id.clone(),
        elapsed_ms: stored.elapsed_ms,
        done: stored.done,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rex_config::LoadedConfig;
    use serial_test::serial;
    use std::sync::Arc;

    fn init_store_test() -> (tempfile::TempDir, HarnessSessionStore, std::path::PathBuf) {
        crate::settings::reset_for_test();
        let dir = tempfile::tempdir().expect("tempdir");
        let workspace = dir.path().to_path_buf();
        let prev_cwd = std::env::current_dir().unwrap_or_else(|_| std::env::temp_dir());
        let mut cfg = rex_config::RexConfig::defaults();
        cfg.daemon.socket_scope = Some(rex_config::DaemonSocketScope::Global);
        std::env::set_current_dir(&workspace).expect("chdir workspace");
        crate::settings::init_for_test(Arc::new(LoadedConfig::for_test(
            workspace.clone(),
            cfg,
        )));
        let resolved = crate::settings::get()
            .resolve_workspace_root()
            .expect("workspace root");
        std::fs::create_dir_all(resolved.join(".rex").join(SESSIONS_DIR))
            .expect("sessions dir");
        (dir, HarnessSessionStore::new(), prev_cwd)
    }

    struct StoreTestGuard {
        _dir: tempfile::TempDir,
        _prev_cwd: std::path::PathBuf,
    }

    impl Drop for StoreTestGuard {
        fn drop(&mut self) {
            if self._prev_cwd.is_dir() {
                let _ = std::env::set_current_dir(&self._prev_cwd);
            } else {
                let _ = std::env::set_current_dir(std::env::temp_dir());
            }
        }
    }

    fn store_test_env() -> (StoreTestGuard, HarnessSessionStore) {
        let (dir, store, prev_cwd) = init_store_test();
        (
            StoreTestGuard {
                _dir: dir,
                _prev_cwd: prev_cwd,
            },
            store,
        )
    }

    #[test]
    #[serial_test::serial]
    fn append_and_fetch_incremental() {
        let (_guard, store) = store_test_env();
        store
            .append_operator_prompt("hs-test", "hello", "turn-1")
            .expect("append");
        let chunk = StreamInferenceResponse {
            event: "chunk".to_string(),
            text: "world".to_string(),
            turn_id: "turn-1".to_string(),
            ..Default::default()
        };
        store
            .append_stream_chunk("hs-test", &chunk)
            .expect("append chunk");
        let page = store
            .fetch_events("hs-test", 0, 0, 10)
            .expect("fetch");
        assert_eq!(page.events.len(), 2);
        assert_eq!(page.head_sequence, 2);
        let inc = store
            .fetch_events("hs-test", 0, 1, 10)
            .expect("incremental");
        assert_eq!(inc.events.len(), 1);
        assert!(!inc.has_more_after);
    }

    #[test]
    #[serial_test::serial]
    fn retroactive_fetch_returns_older_events() {
        let (_guard, store) = store_test_env();
        for i in 1..=5 {
            store
                .append_operator_prompt("hs-retro", &format!("msg {i}"), &format!("t{i}"))
                .expect("append");
        }
        let page = store
            .fetch_events("hs-retro", 4, 0, 2)
            .expect("retro");
        assert_eq!(page.events.len(), 2);
        assert!(page.has_more_before);
        assert_eq!(page.events[0].sequence, 2);
    }
}
