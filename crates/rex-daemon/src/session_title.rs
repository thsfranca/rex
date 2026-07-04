//! Session display title sanitization and ask-mode fallback refresh.

use crate::http_openai_compat::broker_inference_completion;
use crate::session_store::{HarnessSessionStore, SessionMeta, SessionStoreError};

pub const MAX_TITLE_LEN: usize = 48;

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
    let sanitized = sanitize_title(prompt);
    if sanitized.is_empty() {
        return String::new();
    }
    sanitized
}

pub fn title_fallback_suffix(session_id: &str) -> String {
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

pub fn should_run_title_fallback(completed_turns: u32, refresh_every: u32) -> bool {
    if completed_turns == 0 {
        return false;
    }
    let every = refresh_every.max(1);
    completed_turns % every == 0
}

pub async fn maybe_refresh_title_fallback(
    store: &HarnessSessionStore,
    harness_session_id: &str,
    model: &str,
    refresh_every: u32,
) {
    let session_id = harness_session_id.trim();
    if session_id.is_empty() {
        return;
    }
    let mut meta = match store.read_meta(session_id) {
        Ok(m) => m,
        Err(err) => {
            eprintln!("session.title=read_meta_failed session={session_id} err={err}");
            return;
        }
    };
    meta.completed_turns = meta.completed_turns.saturating_add(1);
    let turns = meta.completed_turns;
    if !should_run_title_fallback(turns, refresh_every) {
        let _ = store.write_meta(session_id, &meta);
        return;
    }
    let excerpt = match store.recent_transcript_excerpt(session_id, 8) {
        Ok(s) => s,
        Err(err) => {
            eprintln!("session.title=excerpt_failed session={session_id} err={err}");
            let _ = store.write_meta(session_id, &meta);
            return;
        }
    };
    if excerpt.trim().is_empty() {
        let _ = store.write_meta(session_id, &meta);
        return;
    }
    let current = meta.title.clone();
    let prompt = format!(
        "Current chat title: \"{current}\"\n\nRecent conversation:\n{excerpt}\n\n\
         Reply KEEP if the title still fits. Otherwise reply with a new title only \
         (max {MAX_TITLE_LEN} characters, no quotes)."
    );
    match broker_inference_completion(&prompt, model).await {
        Ok(response) => {
            let trimmed = response.trim();
            if trimmed.eq_ignore_ascii_case("keep") {
                let _ = store.write_meta(session_id, &meta);
                return;
            }
            let next = sanitize_title(trimmed);
            if !next.is_empty() && next != current {
                meta.title = next;
                meta.title_source = "llm".to_string();
                meta.updated_at = chrono_now();
            }
            let _ = store.write_meta(session_id, &meta);
        }
        Err(err) => {
            eprintln!("session.title=fallback_failed session={session_id} err={err}");
            let _ = store.write_meta(session_id, &meta);
        }
    }
}

pub fn apply_tool_title(meta: &mut SessionMeta, title: &str) -> Result<(), SessionStoreError> {
    let next = sanitize_title(title);
    if next.is_empty() {
        return Err(SessionStoreError::InvalidTitle);
    }
    meta.title = next;
    meta.title_source = "tool".to_string();
    meta.updated_at = chrono_now();
    Ok(())
}

fn chrono_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_clamps_long_titles() {
        let long = "a".repeat(60);
        let out = sanitize_title(&long);
        assert!(out.len() <= MAX_TITLE_LEN + 3);
    }

    #[test]
    fn fallback_cadence_every_n_completed_turns() {
        assert!(!should_run_title_fallback(0, 3));
        assert!(!should_run_title_fallback(1, 3));
        assert!(!should_run_title_fallback(2, 3));
        assert!(should_run_title_fallback(3, 3));
        assert!(should_run_title_fallback(6, 3));
    }
}
