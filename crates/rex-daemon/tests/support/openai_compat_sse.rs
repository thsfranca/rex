//! Shared loopback OpenAI-compat SSE fixture for integration tests (plan Step 3.3).
#![allow(dead_code)]

use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

const DEFAULT_SSE_BODY: &str = "data: {\"choices\":[{\"delta\":{\"content\":\"hello stub\"}}]}\n\n\
                                data: [DONE]\n\n";

const TOOL_CALLS_SSE_BODY: &str = "data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"id\":\"call_test\",\"function\":{\"name\":\"fs_read\",\"arguments\":\"{\\\"path\\\":\\\"README.md\\\"}\"}}]}}]}\n\n\
                                  data: [DONE]\n\n";

const TOOL_NAME_REJECT_BODY: &str = "{\"error\":{\"message\":\"Invalid 'tools[0].function.name'\",\"type\":\"invalid_request_error\"}}";

fn extract_model_from_request(request: &str) -> Option<String> {
    let model_key = "\"model\"";
    let start = request.find(model_key)? + model_key.len();
    let rest = request.get(start..)?.trim_start();
    if !rest.starts_with(':') {
        return None;
    }
    let rest = rest[1..].trim_start();
    if !rest.starts_with('"') {
        return None;
    }
    let rest = &rest[1..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn sse_body_for_model(model: &str) -> String {
    let content = format!("model={model}");
    format!(
        "data: {{\"choices\":[{{\"delta\":{{\"content\":\"{content}\"}}}}]}}\n\n\
         data: [DONE]\n\n"
    )
}

/// Binds `127.0.0.1:0` and serves a minimal `text/event-stream` response per connection.
pub async fn spawn_loopback_openai_compat_sse_fixture() -> SocketAddr {
    spawn_loopback_openai_compat_sse_fixture_with_body(DEFAULT_SSE_BODY).await
}

pub async fn spawn_loopback_openai_compat_sse_fixture_with_body(body: &str) -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind fixture");
    let addr = listener.local_addr().expect("fixture addr");
    let body = body.to_string();
    tokio::spawn(async move {
        loop {
            let Ok((mut stream, _)) = listener.accept().await else {
                break;
            };
            let body = body.clone();
            tokio::spawn(async move {
                let mut buf = [0u8; 4096];
                let _ = stream.read(&mut buf).await;
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(response.as_bytes()).await;
            });
        }
    });
    addr
}

/// SSE fixture that returns a single assembled tool_call (native path smoke).
pub async fn spawn_loopback_openai_compat_tool_calls_fixture() -> SocketAddr {
    spawn_loopback_openai_compat_sse_fixture_with_body(TOOL_CALLS_SSE_BODY).await
}

/// Captures the first POST request body and returns 200 SSE with tool_calls using wire names.
pub async fn spawn_loopback_openai_compat_tool_calls_capture_fixture(
) -> (SocketAddr, tokio::sync::oneshot::Receiver<String>) {
    use tokio::sync::oneshot;

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind fixture");
    let addr = listener.local_addr().expect("fixture addr");
    let (tx, rx) = oneshot::channel();
    let body = TOOL_CALLS_SSE_BODY.to_string();
    tokio::spawn(async move {
        if let Ok((mut stream, _)) = listener.accept().await {
            let mut buf = vec![0u8; 16384];
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
    (addr, rx)
}

/// Returns HTTP 400 for tool validation failures (strict provider smoke).
pub async fn spawn_loopback_openai_compat_tool_reject_fixture() -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind fixture");
    let addr = listener.local_addr().expect("fixture addr");
    let body = TOOL_NAME_REJECT_BODY.to_string();
    tokio::spawn(async move {
        if let Ok((mut stream, _)) = listener.accept().await {
            let mut buf = [0u8; 4096];
            let _ = stream.read(&mut buf).await;
            let response = format!(
                "HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes()).await;
        }
    });
    addr
}

const MODELS_JSON: &str = "{\"object\":\"list\",\"data\":[{\"id\":\"ollama/llama3\"}]}";

/// Minimal HTTP responder for gateway `GET /v1/models` health checks.
pub async fn spawn_loopback_gateway_models_fixture() -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind gateway fixture");
    let addr = listener.local_addr().expect("fixture addr");
    let body = MODELS_JSON.to_string();
    tokio::spawn(async move {
        loop {
            let Ok((mut stream, _)) = listener.accept().await else {
                break;
            };
            let body = body.clone();
            tokio::spawn(async move {
                let mut buf = [0u8; 512];
                let _ = stream.read(&mut buf).await;
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(response.as_bytes()).await;
            });
        }
    });
    addr
}

/// Serves SSE where response content echoes the request `"model"` field.
pub async fn spawn_loopback_openai_compat_sse_fixture_echo_model() -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind fixture");
    let addr = listener.local_addr().expect("fixture addr");
    tokio::spawn(async move {
        loop {
            let Ok((mut stream, _)) = listener.accept().await else {
                break;
            };
            tokio::spawn(async move {
                let mut buf = [0u8; 4096];
                let read = stream.read(&mut buf).await.unwrap_or(0);
                let request = String::from_utf8_lossy(&buf[..read]);
                let model =
                    extract_model_from_request(&request).unwrap_or_else(|| "unknown".to_string());
                let body = sse_body_for_model(&model);
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(response.as_bytes()).await;
            });
        }
    });
    addr
}
