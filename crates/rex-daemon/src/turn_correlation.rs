//! Turn correlation ids for Phase 1b (`turn_id`, `context_revision`).

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnCorrelation {
    pub turn_id: String,
    pub context_revision: String,
}

pub fn issue_turn_id(request_id: u64) -> String {
    format!("turn-{request_id}")
}

pub fn build_turn_correlation(
    request_id: u64,
    injected_context: &str,
    retrieval: &str,
    compression_strategy: &str,
    context_selected: usize,
    context_truncated: bool,
) -> TurnCorrelation {
    TurnCorrelation {
        turn_id: issue_turn_id(request_id),
        context_revision: compute_context_revision(
            injected_context,
            retrieval,
            compression_strategy,
            context_selected,
            context_truncated,
        ),
    }
}

pub fn compute_context_revision(
    injected_context: &str,
    retrieval: &str,
    compression_strategy: &str,
    context_selected: usize,
    context_truncated: bool,
) -> String {
    if retrieval == "skipped" && injected_context.is_empty() {
        return String::new();
    }
    let workspace = workspace_fingerprint();
    let mut hasher = DefaultHasher::new();
    injected_context.hash(&mut hasher);
    retrieval.hash(&mut hasher);
    compression_strategy.hash(&mut hasher);
    context_selected.hash(&mut hasher);
    context_truncated.hash(&mut hasher);
    workspace.hash(&mut hasher);
    format!("ctx-{:016x}", hasher.finish())
}

fn workspace_fingerprint() -> String {
    crate::settings::get()
        .workspace_root()
        .display()
        .to_string()
}

/// Strip extension-embedded `---` / `File:` / `Selection:` trailer (conflict C1).
pub fn strip_extension_context_blocks(prompt: &str) -> (String, bool) {
    let trimmed = prompt.trim_end();
    let Some(sep_idx) = trimmed.rfind("\n---\n") else {
        return (prompt.to_string(), false);
    };
    let after_sep = &trimmed[sep_idx + 5..];
    if !after_sep.starts_with("File:") {
        return (prompt.to_string(), false);
    }
    (trimmed[..sep_idx].trim_end().to_string(), true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issue_turn_id_matches_request_id() {
        assert_eq!(issue_turn_id(7), "turn-7");
    }

    #[test]
    fn context_revision_empty_when_no_context_stages() {
        assert!(compute_context_revision("", "skipped", "none", 0, false).is_empty());
    }

    #[test]
    fn strip_removes_extension_file_selection_trailer() {
        let prompt = "fix the bug\n\n---\nFile: src/main.rs\nLanguage: rust";
        let (stripped, applied) = strip_extension_context_blocks(prompt);
        assert!(applied);
        assert_eq!(stripped, "fix the bug");
    }
}
