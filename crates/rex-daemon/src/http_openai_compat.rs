//! OpenAI-compatible HTTP chat/completions adapter (SSE streaming).

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

pub struct HttpOpenAiCompatRuntime {
    client: Client,
    chat_completions_url: String,
    api_key: Option<String>,
    model: String,
    timeout: Duration,
}

impl HttpOpenAiCompatRuntime {
    pub fn from_config() -> Result<Self, String> {
        let cfg = &crate::settings::get().effective.inference.openai_compat;
        let base = cfg.base_url.trim();
        if base.is_empty() {
            return Err(
                "HTTP inference requires inference.openai_compat.base_url (see docs/CONFIGURATION.md)"
                    .to_string(),
            );
        }
        let chat_completions_url = normalize_chat_completions_url(base);
        let api_key = cfg
            .api_key
            .as_ref()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());
        let model = {
            let m = cfg.model.trim();
            if m.is_empty() {
                MODEL_DEFAULT.to_string()
            } else {
                m.to_string()
            }
        };
        let timeout_secs = if cfg.timeout_secs == 0 {
            TIMEOUT_SECS_DEFAULT
        } else {
            cfg.timeout_secs
        };
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
    pub async fn fetch_completion_text(&self, prompt: &str, model: &str) -> Result<String, Status> {
        let effective_model = resolve_inference_model(model, &self.model);
        let body = serde_json::json!({
            "model": effective_model,
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
                    "http inference timed out after {}s (adjust inference.openai_compat.timeout_secs)",
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
        match self.fetch_completion_text(prompt, &self.model).await {
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

/// Resolve request model override against the configured default.
pub fn resolve_inference_model(request_model: &str, default_model: &str) -> String {
    let trimmed = request_model.trim();
    if trimmed.is_empty() {
        default_model.to_string()
    } else {
        trimmed.to_string()
    }
}

/// Broker RPC entry: HTTP OpenAI-compat when env is configured.
pub async fn broker_inference_completion(prompt: &str, model: &str) -> Result<String, String> {
    let runtime = HttpOpenAiCompatRuntime::from_config()?;
    runtime
        .fetch_completion_text(prompt, model)
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
    fn resolves_model_override() {
        assert_eq!(
            resolve_inference_model("", "default-model"),
            "default-model"
        );
        assert_eq!(
            resolve_inference_model("  custom-model  ", "default-model"),
            "custom-model"
        );
    }

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
        use std::sync::Arc;

        use crate::adapters::InferenceRuntime;
        use rex_config::RexConfig;
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

        crate::settings::reset_for_test();
        let mut cfg = RexConfig::defaults();
        cfg.inference.runtime = "http-openai-compat".to_string();
        cfg.inference.openai_compat.base_url = format!("http://{addr}");
        crate::settings::init_for_test(Arc::new(rex_config::LoadedConfig {
            rex_root: std::path::PathBuf::from("/tmp/rex-http-test"),
            global_path: None,
            project_path: None,
            effective: cfg,
        }));

        let runtime = HttpOpenAiCompatRuntime::from_config().expect("runtime");
        let chunks = runtime.build_chunks("ping").await;
        assert!(chunks.len() >= 2);
        assert!(chunks[..chunks.len() - 1]
            .iter()
            .all(|c| c.as_ref().is_ok_and(|v| !v.done)));
        assert!(chunks.last().unwrap().as_ref().unwrap().done);

        crate::settings::reset_for_test();
    }
}
