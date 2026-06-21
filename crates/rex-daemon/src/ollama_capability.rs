//! Ollama `/api/show` capability probe and in-process cache (R038).

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use reqwest::header::CONTENT_TYPE;
use reqwest::Client;
use serde_json::Value;

static CAPABILITY_CACHE: OnceLock<Mutex<HashMap<(String, String), bool>>> = OnceLock::new();

fn cache() -> &'static Mutex<HashMap<(String, String), bool>> {
    CAPABILITY_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Derive Ollama `POST /api/show` URL from an OpenAI-compat `base_url`.
pub fn derive_show_url(base_url: &str) -> Option<String> {
    let trimmed = base_url.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.ends_with("/v1/chat/completions") {
        let root = trimmed.strip_suffix("/v1/chat/completions")?;
        return Some(format!("{root}/api/show"));
    }
    if trimmed.ends_with("/chat/completions") {
        let root = trimmed.strip_suffix("/chat/completions")?;
        return Some(format!("{root}/api/show"));
    }
    if trimmed.ends_with("/v1") {
        let root = trimmed.strip_suffix("/v1")?;
        return Some(format!("{root}/api/show"));
    }
    Some(format!("{trimmed}/api/show"))
}

/// Heuristic: oMLX OpenAI-compat loopback (default :8000 or configured managed port).
pub fn is_omlx_like_base_url(base_url: &str, omlx_port: Option<u16>) -> bool {
    let lower = base_url.trim().to_ascii_lowercase();
    if lower.contains("127.0.0.1:8000") || lower.contains("localhost:8000") {
        return true;
    }
    if let Some(port) = omlx_port {
        if port != 8000 {
            return lower.contains(&format!("127.0.0.1:{port}"))
                || lower.contains(&format!("localhost:{port}"));
        }
    }
    false
}

/// Heuristic: direct Ollama OpenAI-compat surface (localhost:11434 or explicit /v1 on 11434).
pub fn is_ollama_like_base_url(base_url: &str) -> bool {
    let lower = base_url.trim().to_ascii_lowercase();
    lower.contains("127.0.0.1:11434") || lower.contains("localhost:11434")
}

pub fn show_response_supports_tools(body: &str) -> bool {
    let value: Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(_) => return false,
    };
    value
        .pointer("/capabilities")
        .and_then(Value::as_array)
        .is_some_and(|caps| caps.iter().filter_map(Value::as_str).any(|c| c == "tools"))
}

pub async fn model_supports_tools(
    client: &Client,
    show_url: &str,
    model: &str,
    timeout: Duration,
) -> bool {
    let body = serde_json::json!({ "model": model });
    let request = client
        .post(show_url)
        .header(CONTENT_TYPE, "application/json")
        .json(&body);
    let response = match tokio::time::timeout(timeout, request.send()).await {
        Ok(Ok(resp)) => resp,
        _ => return false,
    };
    if !response.status().is_success() {
        return false;
    }
    let text = response.text().await.unwrap_or_default();
    show_response_supports_tools(&text)
}

pub async fn cached_model_supports_tools(
    client: &Client,
    base_url: &str,
    model: &str,
    timeout: Duration,
) -> bool {
    let key = (base_url.trim().to_string(), model.trim().to_string());
    if let Ok(guard) = cache().lock() {
        if let Some(&cached) = guard.get(&key) {
            return cached;
        }
    }
    let show_url = match derive_show_url(base_url) {
        Some(url) => url,
        None => return false,
    };
    let supports = model_supports_tools(client, &show_url, model, timeout).await;
    if let Ok(mut guard) = cache().lock() {
        guard.insert(key, supports);
    }
    supports
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_show_url_from_v1_base() {
        assert_eq!(
            derive_show_url("http://127.0.0.1:11434/v1").as_deref(),
            Some("http://127.0.0.1:11434/api/show")
        );
    }

    #[test]
    fn detects_tools_capability() {
        let body = r#"{"capabilities":["completion","tools","embeddings"]}"#;
        assert!(show_response_supports_tools(body));
        assert!(!show_response_supports_tools(
            r#"{"capabilities":["completion"]}"#
        ));
    }

    #[test]
    fn ollama_like_host_heuristic() {
        assert!(is_ollama_like_base_url("http://127.0.0.1:11434/v1"));
        assert!(!is_ollama_like_base_url("http://127.0.0.1:4000/v1"));
    }

    #[test]
    fn omlx_like_host_heuristic() {
        assert!(is_omlx_like_base_url("http://127.0.0.1:8000/v1", None));
        assert!(is_omlx_like_base_url("http://127.0.0.1:9000/v1", Some(9000)));
        assert!(!is_omlx_like_base_url("http://127.0.0.1:11434/v1", None));
    }
}
