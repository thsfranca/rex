//! OpenAI-compatible HTTP chat/completions adapter (SSE streaming).

use std::env;
use std::time::Duration;

use futures::StreamExt;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use reqwest::Client;
use rex_proto::rex::v1::StreamInferenceResponse;
use serde_json::Value;
use tonic::Status;

use crate::adapters::stream_chunks_with_done;
use crate::domain::chunk_output;

const TIMEOUT_SECS_DEFAULT: u64 = 120;
const STREAM_CHUNK_MAX_CHARS: usize = 8;
pub const MODEL_DEFAULT: &str = "gpt-4o-mini";

/// Environment keys — catalog in `docs/CONFIGURATION.md`.
pub const BASE_URL_ENV: &str = "REX_OPENAI_COMPAT_BASE_URL";
pub const API_KEY_ENV: &str = "REX_OPENAI_COMPAT_API_KEY";
pub const MODEL_ENV: &str = "REX_OPENAI_COMPAT_MODEL";
pub const TIMEOUT_ENV: &str = "REX_OPENAI_COMPAT_TIMEOUT_SECS";

pub struct HttpOpenAiCompatRuntime {
    client: Client,
    chat_completions_url: String,
    api_key: Option<String>,
    model: String,
    timeout: Duration,
}

impl HttpOpenAiCompatRuntime {
    pub fn from_env() -> Result<Self, String> {
        let base = env::var(BASE_URL_ENV)
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .ok_or_else(|| {
                format!("HTTP inference requires {BASE_URL_ENV} (see docs/CONFIGURATION.md)")
            })?;
        let chat_completions_url = normalize_chat_completions_url(&base);
        let api_key = env::var(API_KEY_ENV)
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());
        let model = env::var(MODEL_ENV)
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| MODEL_DEFAULT.to_string());
        let timeout_secs = env::var(TIMEOUT_ENV)
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(TIMEOUT_SECS_DEFAULT);
        let timeout = Duration::from_secs(timeout_secs);
        let client = Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|err| format!("http client build failed: {err}"))?;
        Ok(Self {
            client,
            chat_completions_url,
            api_key,
            model,
            timeout,
        })
    }

    /// Single completion for sidecar `BrokerInference` (assembled from SSE).
    pub async fn fetch_completion_text(&self, prompt: &str) -> Result<String, Status> {
        self.fetch_completion_text_with_model(prompt, None).await
    }

    pub async fn fetch_completion_text_with_model(
        &self,
        prompt: &str,
        model_override: Option<&str>,
    ) -> Result<String, Status> {
        let model = model_override
            .map(str::trim)
            .filter(|m| !m.is_empty())
            .unwrap_or(self.model.as_str());
        let body = serde_json::json!({
            "model": model,
            "messages": [{"role": "user", "content": prompt}],
            "stream": true
        });
        let mut request = self
            .client
            .post(&self.chat_completions_url)
            .header(CONTENT_TYPE, "application/json")
            .json(&body);
        if let Some(key) = &self.api_key {
            request = request.header(AUTHORIZATION, format!("Bearer {key}"));
        }
        let response = tokio::time::timeout(self.timeout, request.send())
            .await
            .map_err(|_| {
                Status::deadline_exceeded(format!(
                    "http inference timed out after {}s (adjust {TIMEOUT_ENV})",
                    self.timeout.as_secs()
                ))
            })?
            .map_err(|err| Status::unavailable(format!("http inference request failed: {err}")))?;
        if !response.status().is_success() {
            let status = response.status();
            let detail = response.text().await.unwrap_or_default();
            return Err(Status::unavailable(format!(
                "http inference failed: status={status} body={}",
                truncate_body(&detail, 512)
            )));
        }
        let mut stream = response.bytes_stream();
        let mut assembled = String::new();
        while let Some(item) = stream.next().await {
            let chunk = item.map_err(|err| {
                Status::unavailable(format!("http inference stream read failed: {err}"))
            })?;
            let text = String::from_utf8_lossy(&chunk);
            for line in text.lines() {
                if let Some(delta) = parse_sse_data_line(line) {
                    assembled.push_str(&delta);
                }
            }
        }
        if assembled.trim().is_empty() {
            return Err(Status::unavailable(
                "http inference returned empty completion",
            ));
        }
        Ok(assembled)
    }
}

#[tonic::async_trait]
impl crate::adapters::InferenceRuntime for HttpOpenAiCompatRuntime {
    async fn build_chunks(&self, prompt: &str) -> Vec<Result<StreamInferenceResponse, Status>> {
        match self.fetch_completion_text(prompt).await {
            Ok(text) => {
                let content_chunks = chunk_output(&text, STREAM_CHUNK_MAX_CHARS);
                stream_chunks_with_done(content_chunks)
            }
            Err(err) => vec![Err(err)],
        }
    }
}

fn normalize_chat_completions_url(base: &str) -> String {
    let trimmed = base.trim_end_matches('/');
    if trimmed.ends_with("/chat/completions") {
        trimmed.to_string()
    } else if trimmed.ends_with("/v1") {
        format!("{trimmed}/chat/completions")
    } else {
        format!("{trimmed}/v1/chat/completions")
    }
}

fn parse_sse_data_line(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if !trimmed.starts_with("data:") {
        return None;
    }
    let payload = trimmed.strip_prefix("data:")?.trim();
    if payload == "[DONE]" {
        return None;
    }
    let value: Value = serde_json::from_str(payload).ok()?;
    value
        .pointer("/choices/0/delta/content")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| {
            value
                .pointer("/choices/0/message/content")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
}

/// Broker RPC entry: HTTP OpenAI-compat when env is configured.
pub async fn broker_inference_completion(prompt: &str, model: &str) -> Result<String, String> {
    let runtime = HttpOpenAiCompatRuntime::from_env()?;
    let override_model = if model.trim().is_empty() {
        None
    } else {
        Some(model.trim())
    };
    runtime
        .fetch_completion_text_with_model(prompt, override_model)
        .await
        .map_err(|status| status.message().to_string())
}

fn truncate_body(body: &str, max: usize) -> String {
    if body.chars().count() <= max {
        return body.to_string();
    }
    body.chars().take(max).collect::<String>() + "…"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_base_urls() {
        assert_eq!(
            normalize_chat_completions_url("http://127.0.0.1:11434/v1"),
            "http://127.0.0.1:11434/v1/chat/completions"
        );
        assert_eq!(
            normalize_chat_completions_url("http://host/v1/chat/completions"),
            "http://host/v1/chat/completions"
        );
    }

    #[test]
    fn parses_sse_delta() {
        let line = r#"data: {"choices":[{"delta":{"content":"hi"}}]}"#;
        assert_eq!(parse_sse_data_line(line), Some("hi".to_string()));
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn streams_from_local_sse_stub() {
        use crate::adapters::InferenceRuntime;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");
        let body = "data: {\"choices\":[{\"delta\":{\"content\":\"hello stub\"}}]}\n\n\
                    data: [DONE]\n\n";
        tokio::spawn(async move {
            if let Ok((mut stream, _)) = listener.accept().await {
                let mut buf = [0u8; 4096];
                let _ = stream.read(&mut buf).await;
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(response.as_bytes()).await;
            }
        });

        let prev_base = env::var(BASE_URL_ENV).ok();
        let prev_runtime = env::var("REX_INFERENCE_RUNTIME").ok();
        env::set_var(BASE_URL_ENV, format!("http://{addr}"));
        env::set_var("REX_INFERENCE_RUNTIME", "http-openai-compat");

        let runtime = HttpOpenAiCompatRuntime::from_env().expect("runtime");
        let chunks = runtime.build_chunks("ping").await;
        assert!(chunks.len() >= 2);
        assert!(chunks[..chunks.len() - 1]
            .iter()
            .all(|c| c.as_ref().is_ok_and(|v| !v.done)));
        assert!(chunks.last().unwrap().as_ref().unwrap().done);

        if let Some(v) = prev_base {
            env::set_var(BASE_URL_ENV, v);
        } else {
            env::remove_var(BASE_URL_ENV);
        }
        if let Some(v) = prev_runtime {
            env::set_var("REX_INFERENCE_RUNTIME", v);
        } else {
            env::remove_var("REX_INFERENCE_RUNTIME");
        }
    }
}
