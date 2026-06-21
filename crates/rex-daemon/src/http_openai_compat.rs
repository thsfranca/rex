//! OpenAI-compatible HTTP chat/completions adapter (SSE streaming).

use std::collections::{BTreeMap, HashMap};
use std::time::Duration;

use futures::StreamExt;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use reqwest::Client;
use rex_config::NativeToolsMode;
use rex_proto::rex::v1::{InferenceProtocol, StreamInferenceResponse};
use serde_json::{json, Value};
use tonic::Status;

use crate::adapters::{stream_chunks_with_done, RuntimeKind};
use crate::domain::chunk_output;
use crate::ollama_capability::{cached_model_supports_tools, is_ollama_like_base_url};

const TIMEOUT_SECS_DEFAULT: u64 = 120;
const STREAM_CHUNK_MAX_CHARS: usize = 8;
pub const MODEL_DEFAULT: &str = "gpt-4o-mini";
const NATIVE_TOOLS_UNSUPPORTED: &str = "native_tools_unsupported";

#[derive(Debug, Clone)]
pub struct HttpChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct HttpToolSpec {
    pub name: String,
    pub description: String,
    pub parameters_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpToolCall {
    pub id: String,
    pub name: String,
    pub arguments_json: String,
}

#[derive(Debug, Clone)]
pub struct BrokerCompletionResult {
    pub content: String,
    pub tool_calls: Vec<HttpToolCall>,
    pub protocol: InferenceProtocol,
    pub error: Option<String>,
}

pub struct HttpOpenAiCompatRuntime {
    client: Client,
    chat_completions_url: String,
    api_key: Option<String>,
    headers: BTreeMap<String, String>,
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
        let headers = cfg.headers.clone();
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
            headers,
            model,
            timeout,
        })
    }

    /// Single completion for sidecar `BrokerInference` (assembled from SSE).
    pub async fn fetch_completion_text(&self, prompt: &str, model: &str) -> Result<String, Status> {
        let messages = vec![HttpChatMessage {
            role: "user".to_string(),
            content: prompt.to_string(),
        }];
        let result = self
            .fetch_broker_completion(&messages, &[], model, false)
            .await?;
        if let Some(err) = result.error {
            return Err(Status::unavailable(err));
        }
        if result.content.trim().is_empty() && result.tool_calls.is_empty() {
            return Err(Status::unavailable(
                "http inference returned empty completion",
            ));
        }
        Ok(result.content)
    }

    pub async fn fetch_broker_completion(
        &self,
        messages: &[HttpChatMessage],
        tools: &[HttpToolSpec],
        model: &str,
        attach_tools: bool,
    ) -> Result<BrokerCompletionResult, Status> {
        let effective_model = resolve_inference_model(model, &self.model);
        let mut body = json!({
            "model": effective_model,
            "messages": messages.iter().map(|m| json!({"role": m.role, "content": m.content})).collect::<Vec<_>>(),
            "stream": true
        });
        let tool_name_map = build_tool_name_map(tools);
        if attach_tools && !tools.is_empty() {
            let openai_tools: Vec<Value> = tools
                .iter()
                .map(|t| {
                    let parameters: Value = serde_json::from_str(&t.parameters_json)
                        .unwrap_or_else(|_| json!({"type": "object", "properties": {}}));
                    json!({
                        "type": "function",
                        "function": {
                            "name": encode_tool_name_for_provider(&t.name),
                            "description": t.description,
                            "parameters": parameters,
                        }
                    })
                })
                .collect();
            body["tools"] = json!(openai_tools);
            body["tool_choice"] = json!("auto");
        }

        let body_str = serde_json::to_string(&body).map_err(|err| {
            Status::internal(format!("http inference request encode failed: {err}"))
        })?;
        let mut request = self.client.post(&self.chat_completions_url);
        request = apply_inference_headers(request, &self.headers, self.api_key.as_deref());
        let request = request.body(body_str);
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
            if attach_tools && status.is_client_error() {
                return Ok(BrokerCompletionResult {
                    content: String::new(),
                    tool_calls: Vec::new(),
                    protocol: InferenceProtocol::InterimFallback,
                    error: Some(format!(
                        "{NATIVE_TOOLS_UNSUPPORTED}: http status={status} body={}",
                        truncate_body(&detail, 512)
                    )),
                });
            }
            return Err(Status::unavailable(format!(
                "http inference failed: status={status} body={}",
                truncate_body(&detail, 512)
            )));
        }

        let mut stream = response.bytes_stream();
        let mut assembler = SseAssemblyState::default();
        while let Some(item) = stream.next().await {
            let chunk = item.map_err(|err| {
                Status::unavailable(format!("http inference stream read failed: {err}"))
            })?;
            let text = String::from_utf8_lossy(&chunk);
            for line in text.lines() {
                assembler.ingest_line(line);
            }
        }

        let protocol = if attach_tools && !tools.is_empty() {
            InferenceProtocol::Native
        } else {
            InferenceProtocol::Interim
        };

        let tool_calls =
            decode_tool_calls_from_provider(assembler.finish_tool_calls(), &tool_name_map);
        Ok(BrokerCompletionResult {
            content: assembler.content,
            tool_calls,
            protocol,
            error: None,
        })
    }

    pub fn client(&self) -> &Client {
        &self.client
    }

    pub fn timeout(&self) -> Duration {
        self.timeout
    }
}

#[derive(Default)]
struct SseAssemblyState {
    content: String,
    tool_parts: BTreeMap<u32, ToolCallPart>,
}

#[derive(Default, Clone)]
struct ToolCallPart {
    id: String,
    name: String,
    arguments: String,
}

impl SseAssemblyState {
    fn ingest_line(&mut self, line: &str) {
        let trimmed = line.trim();
        if !trimmed.starts_with("data:") {
            return;
        }
        let payload = match trimmed.strip_prefix("data:") {
            Some(p) => p.trim(),
            None => return,
        };
        if payload == "[DONE]" {
            return;
        }
        let value: Value = match serde_json::from_str(payload) {
            Ok(v) => v,
            Err(_) => return,
        };
        if let Some(content) = value
            .pointer("/choices/0/delta/content")
            .and_then(Value::as_str)
        {
            self.content.push_str(content);
        } else if let Some(content) = value
            .pointer("/choices/0/message/content")
            .and_then(Value::as_str)
        {
            self.content.push_str(content);
        }

        if let Some(calls) = value
            .pointer("/choices/0/delta/tool_calls")
            .and_then(Value::as_array)
        {
            for call in calls {
                let index = call.get("index").and_then(Value::as_u64).unwrap_or(0) as u32;
                let part = self.tool_parts.entry(index).or_default();
                if let Some(id) = call.get("id").and_then(Value::as_str) {
                    if !id.is_empty() {
                        part.id = id.to_string();
                    }
                }
                if let Some(function) = call.get("function") {
                    if let Some(name) = function.get("name").and_then(Value::as_str) {
                        if !name.is_empty() {
                            part.name = name.to_string();
                        }
                    }
                    if let Some(args) = function.get("arguments").and_then(Value::as_str) {
                        part.arguments.push_str(args);
                    }
                }
            }
        }
    }

    fn finish_tool_calls(&self) -> Vec<HttpToolCall> {
        self.tool_parts
            .values()
            .filter(|p| !p.name.is_empty())
            .map(|p| HttpToolCall {
                id: p.id.clone(),
                name: p.name.clone(),
                arguments_json: p.arguments.clone(),
            })
            .collect()
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

pub fn normalize_chat_completions_url(base: &str) -> String {
    let trimmed = base.trim_end_matches('/');
    if trimmed.ends_with("/chat/completions") {
        trimmed.to_string()
    } else if trimmed.ends_with("/v1") {
        format!("{trimmed}/chat/completions")
    } else {
        format!("{trimmed}/v1/chat/completions")
    }
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

pub async fn broker_inference_http(
    messages: &[HttpChatMessage],
    tools: &[HttpToolSpec],
    model: &str,
    forward_tools: bool,
    native_tools: NativeToolsMode,
    runtime: RuntimeKind,
    base_url: &str,
) -> Result<BrokerCompletionResult, Status> {
    let runtime_http = HttpOpenAiCompatRuntime::from_config().map_err(Status::unavailable)?;

    let mut attach_tools = forward_tools && !tools.is_empty();
    if attach_tools && native_tools == NativeToolsMode::Auto && is_ollama_like_base_url(base_url) {
        let effective_model = resolve_inference_model(model, &runtime_http.model);
        let supports = cached_model_supports_tools(
            runtime_http.client(),
            base_url,
            &effective_model,
            runtime_http.timeout(),
        )
        .await;
        if !supports {
            attach_tools = false;
        }
    }

    if matches!(runtime, RuntimeKind::Mock | RuntimeKind::CursorCli) {
        attach_tools = false;
    }

    if native_tools == NativeToolsMode::False {
        attach_tools = false;
    }

    let attempted_native = forward_tools && !tools.is_empty();
    let result = runtime_http
        .fetch_broker_completion(messages, tools, model, attach_tools)
        .await?;

    if result.error.is_some() {
        return Ok(result);
    }

    if attempted_native && attach_tools {
        if result.tool_calls.is_empty() && result.content.trim().is_empty() {
            return Ok(BrokerCompletionResult {
                content: String::new(),
                tool_calls: Vec::new(),
                protocol: InferenceProtocol::InterimFallback,
                error: Some(format!(
                    "{NATIVE_TOOLS_UNSUPPORTED}: provider returned no tool_calls or content"
                )),
            });
        }
        if result.tool_calls.is_empty() && !result.content.trim().is_empty() {
            return Ok(BrokerCompletionResult {
                content: result.content,
                tool_calls: Vec::new(),
                protocol: InferenceProtocol::InterimFallback,
                error: Some(format!(
                    "{NATIVE_TOOLS_UNSUPPORTED}: provider returned prose-only after tools sent"
                )),
            });
        }
    }

    let protocol = if attach_tools && !result.tool_calls.is_empty() {
        InferenceProtocol::Native
    } else if attempted_native && !attach_tools {
        InferenceProtocol::Interim
    } else {
        result.protocol
    };

    Ok(BrokerCompletionResult {
        content: result.content,
        tool_calls: result.tool_calls,
        protocol,
        error: None,
    })
}

/// Broker RPC entry: HTTP OpenAI-compat when env is configured (prompt-only legacy).
#[allow(dead_code)]
pub async fn broker_inference_completion(prompt: &str, model: &str) -> Result<String, String> {
    let runtime = HttpOpenAiCompatRuntime::from_config()?;
    runtime
        .fetch_completion_text(prompt, model)
        .await
        .map_err(|status| status.message().to_string())
}

fn apply_inference_headers(
    mut request: reqwest::RequestBuilder,
    headers: &BTreeMap<String, String>,
    api_key: Option<&str>,
) -> reqwest::RequestBuilder {
    for (name, value) in headers {
        request = request.header(name.as_str(), value.as_str());
    }
    let has_authorization = headers
        .keys()
        .any(|name| name.eq_ignore_ascii_case("authorization"));
    if let Some(key) = api_key {
        if !has_authorization {
            request = request.header(AUTHORIZATION, format!("Bearer {key}"));
        }
    }
    request.header(CONTENT_TYPE, "application/json")
}

/// Encode Rex canonical tool names for strict OpenAI-compat providers (e.g. DeepSeek).
fn encode_tool_name_for_provider(name: &str) -> String {
    name.replace('.', "_")
}

fn build_tool_name_map(tools: &[HttpToolSpec]) -> HashMap<String, String> {
    tools
        .iter()
        .map(|t| {
            let wire = encode_tool_name_for_provider(&t.name);
            (wire, t.name.clone())
        })
        .collect()
}

fn decode_tool_name_from_provider(wire: &str, map: &HashMap<String, String>) -> String {
    map.get(wire).cloned().unwrap_or_else(|| wire.to_string())
}

fn decode_tool_calls_from_provider(
    calls: Vec<HttpToolCall>,
    map: &HashMap<String, String>,
) -> Vec<HttpToolCall> {
    calls
        .into_iter()
        .map(|mut call| {
            call.name = decode_tool_name_from_provider(&call.name, map);
            call
        })
        .collect()
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
    fn encodes_and_decodes_tool_names_for_provider_wire() {
        let tools = vec![
            HttpToolSpec {
                name: "fs.read".to_string(),
                description: String::new(),
                parameters_json: "{}".to_string(),
            },
            HttpToolSpec {
                name: "web.search".to_string(),
                description: String::new(),
                parameters_json: "{}".to_string(),
            },
        ];
        let map = build_tool_name_map(&tools);
        assert_eq!(encode_tool_name_for_provider("fs.read"), "fs_read");
        assert_eq!(encode_tool_name_for_provider("exec.shell"), "exec_shell");
        assert_eq!(decode_tool_name_from_provider("fs_read", &map), "fs.read");
        assert_eq!(
            decode_tool_name_from_provider("web_search", &map),
            "web.search"
        );
        assert_eq!(
            decode_tool_name_from_provider("unknown_tool", &map),
            "unknown_tool"
        );
    }

    #[test]
    fn decodes_provider_tool_calls_to_canonical_names() {
        let tools = vec![HttpToolSpec {
            name: "fs.read".to_string(),
            description: String::new(),
            parameters_json: "{}".to_string(),
        }];
        let map = build_tool_name_map(&tools);
        let calls = decode_tool_calls_from_provider(
            vec![HttpToolCall {
                id: "call_1".to_string(),
                name: "fs_read".to_string(),
                arguments_json: r#"{"path":"x"}"#.to_string(),
            }],
            &map,
        );
        assert_eq!(calls[0].name, "fs.read");
    }

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
        let mut state = SseAssemblyState::default();
        state.ingest_line(r#"data: {"choices":[{"delta":{"content":"hi"}}]}"#);
        assert_eq!(state.content, "hi");
    }

    #[test]
    fn assembles_tool_calls_from_sse_fragments() {
        let mut state = SseAssemblyState::default();
        state.ingest_line(
            r#"data: {"choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_1","function":{"name":"fs.read","arguments":"{\"path\""}}]}}]}"#,
        );
        state.ingest_line(
            r#"data: {"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":"\":\"x\"}"}}]}}]}"#,
        );
        let calls = state.finish_tool_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "fs.read");
        assert_eq!(calls[0].id, "call_1");
        assert!(calls[0].arguments_json.contains("path"));
    }

    #[test]
    fn api_key_skipped_when_authorization_header_configured() {
        let mut headers = BTreeMap::new();
        headers.insert("Authorization".to_string(), "Custom scheme".to_string());
        let request = apply_inference_headers(
            Client::new().post("http://127.0.0.1:9/v1/chat/completions"),
            &headers,
            Some("should-not-apply"),
        );
        let built = request.build().expect("build request");
        assert_eq!(
            built
                .headers()
                .get(AUTHORIZATION)
                .and_then(|v| v.to_str().ok()),
            Some("Custom scheme")
        );
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn forwards_custom_headers_to_backend() {
        use std::sync::Arc;

        use crate::adapters::InferenceRuntime;
        use rex_config::RexConfig;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;
        use tokio::sync::oneshot;

        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");
        let (tx, rx) = oneshot::channel::<String>();
        let body = "data: {\"choices\":[{\"delta\":{\"content\":\"hello stub\"}}]}\n\n\
                    data: [DONE]\n\n";
        tokio::spawn(async move {
            if let Ok((mut stream, _)) = listener.accept().await {
                let mut buf = vec![0u8; 8192];
                let n = stream.read(&mut buf).await.unwrap_or(0);
                let request = String::from_utf8_lossy(&buf[..n]).to_string();
                let _ = tx.send(request);
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
        cfg.inference
            .openai_compat
            .headers
            .insert("X-Api-Key".to_string(), "test-token".to_string());
        crate::settings::init_for_test(Arc::new(rex_config::LoadedConfig::for_test(
            std::path::PathBuf::from("/tmp/rex-http-test"),
            cfg,
        )));

        let runtime = HttpOpenAiCompatRuntime::from_config().expect("runtime");
        let _chunks = runtime.build_chunks("ping").await;

        let request = rx.await.expect("request captured");
        let lower = request.to_ascii_lowercase();
        assert!(
            lower.contains("x-api-key: test-token"),
            "expected custom header in request, got: {request}"
        );

        crate::settings::reset_for_test();
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
        crate::settings::init_for_test(Arc::new(rex_config::LoadedConfig::for_test(
            std::path::PathBuf::from("/tmp/rex-http-test"),
            cfg,
        )));

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
