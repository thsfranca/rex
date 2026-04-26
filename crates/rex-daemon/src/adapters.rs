use std::env;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use rex_proto::rex::v1::StreamInferenceResponse;
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tonic::Status;

use crate::domain::{build_mock_output, chunk_output};

const STREAM_CHUNK_MAX_CHARS: usize = 8;
const CURSOR_TIMEOUT_SECS_DEFAULT: u64 = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeKind {
    Mock,
    CursorCli,
}

impl RuntimeKind {
    pub fn from_env() -> Self {
        let raw = env::var("REX_INFERENCE_RUNTIME").unwrap_or_else(|_| "mock".to_string());
        Self::from_setting(&raw)
    }

    fn from_setting(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "cursor" | "cursor-cli" => Self::CursorCli,
            _ => Self::Mock,
        }
    }
}

#[tonic::async_trait]
pub trait InferenceRuntime: Send + Sync {
    async fn build_chunks(&self, prompt: &str) -> Vec<Result<StreamInferenceResponse, Status>>;
}

pub fn runtime_from_env() -> Arc<dyn InferenceRuntime> {
    match RuntimeKind::from_env() {
        RuntimeKind::Mock => Arc::new(MockInferenceRuntime),
        RuntimeKind::CursorCli => Arc::new(CursorCliRuntime::from_env()),
    }
}

pub struct MockInferenceRuntime;

#[tonic::async_trait]
impl InferenceRuntime for MockInferenceRuntime {
    async fn build_chunks(&self, prompt: &str) -> Vec<Result<StreamInferenceResponse, Status>> {
        let text = build_mock_output(prompt);
        let content_chunks = chunk_output(&text, STREAM_CHUNK_MAX_CHARS);
        stream_chunks_with_done(content_chunks)
    }
}

pub struct CursorCliRuntime {
    command_template: Option<String>,
    command_path: String,
    timeout: Duration,
}

impl CursorCliRuntime {
    pub fn from_env() -> Self {
        let command_template = env::var("REX_CURSOR_CLI_COMMAND")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        let command_path = env::var("REX_CURSOR_CLI_PATH")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "cursor-agent".to_string());
        let timeout = env::var("REX_CURSOR_CLI_TIMEOUT_SECS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .map(Duration::from_secs)
            .unwrap_or(Duration::from_secs(CURSOR_TIMEOUT_SECS_DEFAULT));
        Self {
            command_template,
            command_path,
            timeout,
        }
    }

    #[cfg(test)]
    fn with_command_template(command_template: &str, timeout_secs: u64) -> Self {
        Self {
            command_template: Some(command_template.to_string()),
            command_path: "cursor-agent".to_string(),
            timeout: Duration::from_secs(timeout_secs),
        }
    }

    async fn run_command(&self, prompt: &str) -> Result<String, Status> {
        let mut command = if let Some(template) = &self.command_template {
            let rendered = template.replace("{prompt}", &shell_single_quote(prompt));
            let mut cmd = Command::new("sh");
            cmd.arg("-lc").arg(rendered);
            cmd
        } else {
            let mut cmd = Command::new(&self.command_path);
            cmd.arg("-p").arg(prompt).arg("--output-format").arg("json");
            cmd
        };
        command.stdout(Stdio::piped()).stderr(Stdio::piped());
        let mut child = command
            .spawn()
            .map_err(|err| Status::unavailable(format!("cursor runtime spawn failed: {err}")))?;

        let mut stdout = child
            .stdout
            .take()
            .ok_or_else(|| Status::internal("cursor runtime stdout missing"))?;
        let mut stderr = child
            .stderr
            .take()
            .ok_or_else(|| Status::internal("cursor runtime stderr missing"))?;

        let stdout_task = tokio::spawn(async move {
            let mut buf = Vec::new();
            let _ = stdout.read_to_end(&mut buf).await?;
            Ok::<Vec<u8>, std::io::Error>(buf)
        });
        let stderr_task = tokio::spawn(async move {
            let mut buf = Vec::new();
            let _ = stderr.read_to_end(&mut buf).await?;
            Ok::<Vec<u8>, std::io::Error>(buf)
        });

        let wait_result = tokio::time::timeout(self.timeout, child.wait()).await;
        let exit_status = match wait_result {
            Ok(Ok(status)) => status,
            Ok(Err(err)) => {
                let _ = child.kill().await;
                return Err(Status::internal(format!(
                    "cursor runtime wait failed: {err}"
                )));
            }
            Err(_) => {
                let _ = child.kill().await;
                let _ = child.wait().await;
                return Err(Status::deadline_exceeded(format!(
                    "cursor runtime timed out after {}s",
                    self.timeout.as_secs()
                )));
            }
        };

        let stdout_bytes = stdout_task
            .await
            .map_err(|err| Status::internal(format!("cursor runtime stdout task failed: {err}")))?
            .map_err(|err| Status::internal(format!("cursor runtime stdout read failed: {err}")))?;
        let stderr_bytes = stderr_task
            .await
            .map_err(|err| Status::internal(format!("cursor runtime stderr task failed: {err}")))?
            .map_err(|err| Status::internal(format!("cursor runtime stderr read failed: {err}")))?;

        if !exit_status.success() {
            let stderr = String::from_utf8_lossy(&stderr_bytes);
            return Err(Status::unavailable(format!(
                "cursor runtime failed: status={exit_status}; stderr={}",
                stderr.trim()
            )));
        }

        let stdout_text = String::from_utf8_lossy(&stdout_bytes).to_string();
        Ok(stdout_text)
    }
}

#[tonic::async_trait]
impl InferenceRuntime for CursorCliRuntime {
    async fn build_chunks(&self, prompt: &str) -> Vec<Result<StreamInferenceResponse, Status>> {
        match self.run_command(prompt).await {
            Ok(raw) => {
                let text = extract_cursor_text(&raw);
                let content_chunks = chunk_output(&text, STREAM_CHUNK_MAX_CHARS);
                stream_chunks_with_done(content_chunks)
            }
            Err(err) => vec![Err(err)],
        }
    }
}

fn stream_chunks_with_done(
    content_chunks: Vec<String>,
) -> Vec<Result<StreamInferenceResponse, Status>> {
    let mut chunks = Vec::new();
    for (index, chunk) in content_chunks.iter().enumerate() {
        chunks.push(Ok(StreamInferenceResponse {
            text: chunk.clone(),
            index: index as u64,
            done: false,
        }));
    }
    chunks.push(Ok(StreamInferenceResponse {
        text: String::new(),
        index: content_chunks.len() as u64,
        done: true,
    }));
    chunks
}

fn extract_cursor_text(raw_stdout: &str) -> String {
    let mut extracted = String::new();
    for line in raw_stdout.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(trimmed) {
            if let Some(text) = extract_text_from_json_line(&json) {
                if !extracted.is_empty() {
                    extracted.push('\n');
                }
                extracted.push_str(text);
                continue;
            }
        }
        if !extracted.is_empty() {
            extracted.push('\n');
        }
        extracted.push_str(trimmed);
    }
    if extracted.trim().is_empty() {
        "[empty response]".to_string()
    } else {
        extracted
    }
}

fn extract_text_from_json_line(value: &serde_json::Value) -> Option<&str> {
    value
        .get("text")
        .and_then(serde_json::Value::as_str)
        .or_else(|| value.get("content").and_then(serde_json::Value::as_str))
        .or_else(|| {
            value
                .get("delta")
                .and_then(|delta| delta.get("text"))
                .and_then(serde_json::Value::as_str)
        })
}

fn shell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

#[cfg(test)]
mod tests {
    use super::{
        extract_cursor_text, shell_single_quote, CursorCliRuntime, InferenceRuntime, RuntimeKind,
    };

    #[test]
    fn runtime_kind_defaults_to_mock() {
        assert_eq!(RuntimeKind::from_setting(""), RuntimeKind::Mock);
    }

    #[test]
    fn runtime_kind_uses_cursor_when_requested() {
        assert_eq!(
            RuntimeKind::from_setting("cursor-cli"),
            RuntimeKind::CursorCli
        );
    }

    #[test]
    fn extracts_text_from_json_and_plain_lines() {
        let raw = r#"{"text":"hello"}
{"delta":{"text":"world"}}
plain line"#;
        assert_eq!(extract_cursor_text(raw), "hello\nworld\nplain line");
    }

    #[test]
    fn shell_quote_escapes_single_quotes() {
        assert_eq!(shell_single_quote("a'b"), "'a'\"'\"'b'");
    }

    #[tokio::test]
    async fn cursor_runtime_maps_timeout_to_terminal_error() {
        let runtime = CursorCliRuntime::with_command_template("sleep 2", 1);
        let chunks = runtime.build_chunks("ignored").await;
        assert_eq!(chunks.len(), 1);
        let err = chunks[0]
            .as_ref()
            .expect_err("timeout should return an error chunk");
        assert_eq!(err.code(), tonic::Code::DeadlineExceeded);
    }

    #[tokio::test]
    async fn cursor_runtime_returns_chunk_and_terminal_done() {
        let runtime = CursorCliRuntime::with_command_template(
            "printf '{\"text\":\"hello from cursor\"}\\n'",
            20,
        );
        let chunks = runtime.build_chunks("ignored").await;
        assert!(chunks.len() >= 2);
        assert!(chunks[..chunks.len() - 1]
            .iter()
            .all(|chunk| chunk.as_ref().is_ok_and(|value| !value.done)));
        assert!(chunks[chunks.len() - 1]
            .as_ref()
            .is_ok_and(|value| value.done));
    }
}
