//! Display truncation aligned with daemon broker output bounds.

/// Marker appended when tool output is truncated for display.
pub const TRUNCATION_MARKER: &str = " [rex: tool output truncated]";

/// Truncate `text` to fit within `max_bytes` (UTF-8), ending at a line boundary.
/// Appends [`TRUNCATION_MARKER`] when truncated.
pub fn truncate_display(text: &str, max_bytes: usize) -> String {
    if text.len() <= max_bytes {
        return text.to_string();
    }
    let marker_len = TRUNCATION_MARKER.len();
    let budget = max_bytes.saturating_sub(marker_len);
    if budget == 0 {
        return TRUNCATION_MARKER.to_string();
    }
    let bytes = text.as_bytes();
    let mut end = budget.min(bytes.len());
    while end > 0 && std::str::from_utf8(&bytes[..end]).is_err() {
        end -= 1;
    }
    if end == 0 {
        return TRUNCATION_MARKER.to_string();
    }
    let prefix = std::str::from_utf8(&bytes[..end]).expect("utf8 boundary");
    let body = match prefix.rfind('\n') {
        Some(idx) if idx > 0 => &prefix[..idx],
        Some(_) => "",
        None => "",
    };
    if body.is_empty() {
        TRUNCATION_MARKER.to_string()
    } else {
        format!("{body}{TRUNCATION_MARKER}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncates_at_line_boundary() {
        let text = "line one\nline two\nline three";
        let out = truncate_display(text, 15);
        assert!(out.ends_with(TRUNCATION_MARKER));
    }

    #[test]
    fn short_text_unchanged() {
        assert_eq!(truncate_display("ok", 100), "ok");
    }
}
