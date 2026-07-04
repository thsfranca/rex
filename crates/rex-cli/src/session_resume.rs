//! Closed-session history, locks, and resume resolution.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::error::CliError;
use crate::lock_util::{lock_holder_alive, try_acquire_lock, PidLock};
use crate::session_meta::{read_meta, sanitize_session_id, session_lock_path, session_log_path};

const CLOSED_SESSIONS_FILE: &str = "closed_sessions.json";
const MAX_CLOSED_HISTORY: usize = 32;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ClosedSessionsFile {
    #[serde(default)]
    sessions: Vec<ClosedSessionEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ClosedSessionEntry {
    harness_session_id: String,
    closed_at: String,
}

#[derive(Debug, Clone)]
pub struct ClosedSessionItem {
    pub harness_session_id: String,
    pub title: String,
    pub closed_at: String,
}

pub fn workspace_root() -> Result<PathBuf, CliError> {
    rex_config::load_merged()
        .map_err(|_| CliError::workspace_not_configured())?
        .resolve_workspace_root()
        .map_err(|_| CliError::workspace_not_configured())
}

pub fn record_closed_session(workspace: &Path, harness_session_id: &str) -> Result<(), CliError> {
    let id = harness_session_id.trim();
    if sanitize_session_id(id).is_none() {
        return Ok(());
    }
    let path = closed_sessions_path(workspace);
    let mut file = read_closed_file(&path);
    file.sessions.retain(|e| e.harness_session_id != id);
    file.sessions.insert(
        0,
        ClosedSessionEntry {
            harness_session_id: id.to_string(),
            closed_at: iso_now(),
        },
    );
    file.sessions.truncate(MAX_CLOSED_HISTORY);
    write_closed_file(&path, &file).map_err(CliError::Stdout)?;
    Ok(())
}

pub fn list_closed_sessions(workspace: &Path) -> Result<Vec<ClosedSessionItem>, CliError> {
    let path = closed_sessions_path(workspace);
    let file = read_closed_file(&path);
    let mut out = Vec::new();
    for entry in file.sessions {
        if !is_session_available(workspace, &entry.harness_session_id) {
            continue;
        }
        let meta = read_meta(workspace, &entry.harness_session_id);
        let title = if meta.title.is_empty() {
            fallback_title(&entry.harness_session_id)
        } else {
            meta.title
        };
        out.push(ClosedSessionItem {
            harness_session_id: entry.harness_session_id,
            title,
            closed_at: entry.closed_at,
        });
    }
    Ok(out)
}

pub fn resolve_last_available_session_id(workspace: &Path) -> Result<String, CliError> {
    let items = list_closed_sessions(workspace)?;
    if items.is_empty() {
        let path = closed_sessions_path(workspace);
        if read_closed_file(&path).sessions.is_empty() {
            return Err(CliError::NoSessionToContinue);
        }
        return Err(CliError::AllSessionsOpen);
    }
    Ok(items[0].harness_session_id.clone())
}

pub fn acquire_session_lock(
    workspace: &Path,
    harness_session_id: &str,
) -> Result<PidLock, CliError> {
    let path = session_lock_path(workspace, harness_session_id)
        .ok_or(CliError::SessionNotFound)?;
    try_acquire_lock(&path).ok_or(CliError::SessionLockFailed)
}

pub fn is_session_available(workspace: &Path, harness_session_id: &str) -> bool {
    let Some(log) = session_log_path(workspace, harness_session_id) else {
        return false;
    };
    if !log.is_file() {
        return false;
    }
    let Some(lock_path) = session_lock_path(workspace, harness_session_id) else {
        return false;
    };
    if lock_path.is_file() && lock_holder_alive(&lock_path) {
        return false;
    }
    true
}

fn fallback_title(session_id: &str) -> String {
    let suffix: String = session_id
        .chars()
        .rev()
        .take(6)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("Chat {suffix}")
}

fn closed_sessions_path(workspace: &Path) -> PathBuf {
    workspace.join(".rex").join(CLOSED_SESSIONS_FILE)
}

fn read_closed_file(path: &Path) -> ClosedSessionsFile {
    if !path.is_file() {
        return ClosedSessionsFile {
            sessions: Vec::new(),
        };
    }
    std::fs::read_to_string(path)
        .ok()
        .and_then(|contents| serde_json::from_str(&contents).ok())
        .unwrap_or(ClosedSessionsFile {
            sessions: Vec::new(),
        })
}

fn write_closed_file(path: &Path, file: &ClosedSessionsFile) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let line = serde_json::to_string(file).map_err(|err| {
        std::io::Error::new(std::io::ErrorKind::InvalidData, err.to_string())
    })?;
    std::fs::write(path, format!("{line}\n"))
}

fn iso_now() -> String {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

pub fn format_relative_closed_at(closed_at: &str) -> String {
    let Ok(secs) = closed_at.parse::<u64>() else {
        return String::new();
    };
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(secs);
    let delta = now.saturating_sub(secs);
    if delta < 3600 {
        let mins = (delta / 60).max(1);
        format!("{mins}m ago")
    } else if delta < 86_400 {
        format!("{}h ago", delta / 3600)
    } else {
        format!("{}d ago", delta / 86_400)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_prepends_and_caps_history() {
        let dir = tempfile::tempdir().expect("tempdir");
        let ws = dir.path();
        for i in 0..40 {
            record_closed_session(ws, &format!("hs-test-{i}")).expect("record");
        }
        let file = read_closed_file(&closed_sessions_path(ws));
        assert_eq!(file.sessions.len(), MAX_CLOSED_HISTORY);
        assert_eq!(file.sessions[0].harness_session_id, "hs-test-39");
    }
}
