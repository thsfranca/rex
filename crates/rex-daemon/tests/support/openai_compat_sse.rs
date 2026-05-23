//! Shared loopback OpenAI-compat SSE fixture for integration tests (plan Step 3.3).

use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

const DEFAULT_SSE_BODY: &str = "data: {\"choices\":[{\"delta\":{\"content\":\"hello stub\"}}]}\n\n\
                                data: [DONE]\n\n";

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
