//! Advisory prompt intent detection (R067 / R070).

const ADVISORY_PATTERNS: &[&str] = &[
    "what should we do next",
    "what's next",
    "what to work on",
    "priorities",
    "roadmap",
    "next step",
];

/// True when the prompt asks for priorities, roadmap, or next-work guidance.
pub fn matches_advisory_intent(prompt: &str) -> bool {
    let lower = prompt.to_ascii_lowercase();
    ADVISORY_PATTERNS
        .iter()
        .any(|pattern| lower.contains(pattern))
}

/// True when injected daemon context already carries roadmap/priority signals.
/// Retained for R070 sidecar parity; daemon init gating lives in the sidecar.
#[allow(dead_code)]
pub fn context_has_priority_markers(context: &str) -> bool {
    let lower = context.to_ascii_lowercase();
    lower.contains("roadmap") || lower.contains("prioritization") || lower.contains("current focus")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_short_advisory_prompt() {
        assert!(matches_advisory_intent("What should we do next?"));
    }

    #[test]
    fn skips_unrelated_short_prompt() {
        assert!(!matches_advisory_intent("hello"));
    }
}
