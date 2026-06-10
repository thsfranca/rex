use std::collections::BTreeMap;

use crate::error::ConfigError;
use crate::model::OpenAiCompatConfig;

/// Validate `inference.openai_compat.headers` entries for HTTP wire use.
pub fn validate_openai_compat(cfg: &OpenAiCompatConfig) -> Result<(), ConfigError> {
    for (name, value) in &cfg.headers {
        validate_header_name(name)?;
        validate_header_value(value)?;
    }
    Ok(())
}

fn validate_header_name(name: &str) -> Result<(), ConfigError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(ConfigError::Validation(
            "inference.openai_compat.headers: header name must not be empty".to_string(),
        ));
    }
    if trimmed != name {
        return Err(ConfigError::Validation(format!(
            "inference.openai_compat.headers: header name must not contain leading or trailing whitespace: {name:?}"
        )));
    }
    if !name.bytes().all(is_tchar) {
        return Err(ConfigError::Validation(format!(
            "inference.openai_compat.headers: invalid header name: {name:?}"
        )));
    }
    Ok(())
}

fn validate_header_value(value: &str) -> Result<(), ConfigError> {
    if value.bytes().any(|b| b == b'\r' || b == b'\n' || b == 0) {
        return Err(ConfigError::Validation(
            "inference.openai_compat.headers: header value must not contain CR, LF, or NUL"
                .to_string(),
        ));
    }
    Ok(())
}

/// RFC 7230 `tchar` — valid HTTP header name characters.
fn is_tchar(b: u8) -> bool {
    matches!(
        b,
        b'!' | b'#'
            | b'$'
            | b'%'
            | b'&'
            | b'\''
            | b'*'
            | b'+'
            | b'-'
            | b'.'
            | b'^'
            | b'_'
            | b'`'
            | b'|'
            | b'~'
    ) || b.is_ascii_alphanumeric()
}

pub fn header_names_sorted(headers: &BTreeMap<String, String>) -> Vec<String> {
    headers.keys().cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_valid_headers() {
        let mut cfg = OpenAiCompatConfig::default();
        cfg.headers
            .insert("X-Api-Key".to_string(), "secret".to_string());
        cfg.headers
            .insert("Authorization".to_string(), "Bearer token".to_string());
        validate_openai_compat(&cfg).expect("valid");
    }

    #[test]
    fn rejects_empty_header_name() {
        let mut cfg = OpenAiCompatConfig::default();
        cfg.headers.insert(String::new(), "v".to_string());
        let err = validate_openai_compat(&cfg).expect_err("empty name");
        assert!(err.to_string().contains("empty"));
    }

    #[test]
    fn rejects_invalid_header_name() {
        let mut cfg = OpenAiCompatConfig::default();
        cfg.headers.insert("bad name".to_string(), "v".to_string());
        let err = validate_openai_compat(&cfg).expect_err("space in name");
        assert!(err.to_string().contains("invalid header name"));
    }

    #[test]
    fn rejects_newline_in_value() {
        let mut cfg = OpenAiCompatConfig::default();
        cfg.headers.insert("X-Key".to_string(), "a\nb".to_string());
        let err = validate_openai_compat(&cfg).expect_err("newline");
        assert!(err.to_string().contains("CR, LF, or NUL"));
    }
}
