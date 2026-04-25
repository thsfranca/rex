/// Unix Domain Socket path for local daemon-client transport.
pub const SOCKET_PATH: &str = "/tmp/rex.sock";

/// Daemon semantic version exposed over status endpoint.
pub const DAEMON_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Model identifier used by the MVP mock inference implementation.
pub const ACTIVE_MODEL_ID: &str = "mock-model-v0";

/// Returns the deterministic mock output for a prompt.
pub fn build_mock_output(prompt: &str) -> String {
    if prompt.trim().is_empty() {
        "[empty prompt]".to_string()
    } else {
        format!("mock: {prompt}")
    }
}

/// Splits text into deterministic, contiguous chunks for streaming.
pub fn chunk_output(text: &str, max_chunk_chars: usize) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }

    let chunk_size = max_chunk_chars.max(1);
    let chars: Vec<char> = text.chars().collect();
    chars
        .chunks(chunk_size)
        .map(|chunk| chunk.iter().collect::<String>())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{build_mock_output, chunk_output};

    #[test]
    fn uses_empty_prompt_marker_for_blank_input() {
        assert_eq!(build_mock_output("   "), "[empty prompt]");
    }

    #[test]
    fn prefixes_non_empty_prompt() {
        assert_eq!(build_mock_output("hello"), "mock: hello");
    }

    #[test]
    fn chunks_output_into_contiguous_segments() {
        let chunks = chunk_output("mock: hello world", 5);
        assert_eq!(chunks, vec!["mock:", " hell", "o wor", "ld"]);
    }

    #[test]
    fn uses_minimum_chunk_size_of_one() {
        let chunks = chunk_output("abc", 0);
        assert_eq!(chunks, vec!["a", "b", "c"]);
    }
}
