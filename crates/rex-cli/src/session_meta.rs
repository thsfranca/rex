//! Read/write harness session display metadata from workspace `.rex/sessions/`.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const MAX_TITLE_LEN: usize = 48;
const SESSIONS_DIR: &str = "sessions";

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SessionMeta {
    pub title: String,
    #[serde(default)]
    pub title_source: String,
    #[serde(default)]
    pub updated_at: String,
    #[serde(default)]
    pub completed_turns: u32,
}

pub fn sanitize_session_id(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.len() > 128 {
        return None;
    }
    if !trimmed
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return None;
    }
    Some(trimmed.to_string())
}

pub fn sanitize_title(raw: &str) -> String {
    let collapsed: String = raw
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let trimmed = collapsed.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if trimmed.len() <= MAX_TITLE_LEN {
        return trimmed.to_string();
    }
    let mut end = MAX_TITLE_LEN;
    while end > 0 && !trimmed.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &trimmed[..end])
}

pub fn title_from_prompt(prompt: &str) -> String {
    sanitize_title(prompt)
}

pub fn read_meta(workspace: &Path, harness_session_id: &str) -> SessionMeta {
    let Some(path) = meta_path(workspace, harness_session_id) else {
        return SessionMeta::default();
    };
    if !path.is_file() {
        return SessionMeta::default();
    }
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|contents| serde_json::from_str(&contents).ok())
        .unwrap_or_default()
}

pub fn write_prompt_fallback_meta(
    workspace: &Path,
    harness_session_id: &str,
    prompt: &str,
) -> std::io::Result<()> {
    let mut meta = read_meta(workspace, harness_session_id);
    if !meta.title.is_empty() {
        return Ok(());
    }
    let title = title_from_prompt(prompt);
    if title.is_empty() {
        return Ok(());
    }
    meta.title = title;
    meta.title_source = "prompt".to_string();
    write_meta(workspace, harness_session_id, &meta)
}

pub fn write_meta(
    workspace: &Path,
    harness_session_id: &str,
    meta: &SessionMeta,
) -> std::io::Result<()> {
    let path = meta_path(workspace, harness_session_id)
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid session id"))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let line = serde_json::to_string(meta).map_err(|err| {
        std::io::Error::new(std::io::ErrorKind::InvalidData, err.to_string())
    })?;
    std::fs::write(path, format!("{line}\n"))
}

fn meta_path(workspace: &Path, harness_session_id: &str) -> Option<PathBuf> {
    let safe = sanitize_session_id(harness_session_id)?;
    Some(
        workspace
            .join(".rex")
            .join(SESSIONS_DIR)
            .join(format!("{safe}.meta.json")),
    )
}

pub fn session_log_path(workspace: &Path, harness_session_id: &str) -> Option<PathBuf> {
    let safe = sanitize_session_id(harness_session_id)?;
    Some(
        workspace
            .join(".rex")
            .join(SESSIONS_DIR)
            .join(format!("{safe}.jsonl")),
    )
}

pub fn session_lock_path(workspace: &Path, harness_session_id: &str) -> Option<PathBuf> {
    let safe = sanitize_session_id(harness_session_id)?;
    Some(
        workspace
            .join(".rex")
            .join(SESSIONS_DIR)
            .join(".locks")
            .join(format!("{safe}.lock")),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_title_clamps() {
        let long = "a".repeat(60);
        assert!(sanitize_title(&long).len() <= MAX_TITLE_LEN + 3);
    }
}
