use std::time::Duration;

use futures::StreamExt;
use rex_sidecar_stub::serve_on_socket;
use serial_test::serial;
use tokio::task::JoinHandle;
use tokio::time::{sleep, timeout, Instant};

#[allow(dead_code)]
#[path = "../src/sidecar_client.rs"]
mod sidecar_client;

const READINESS_TIMEOUT: Duration = Duration::from_secs(8);
const RUN_TIMEOUT: Duration = Duration::from_secs(5);

fn test_socket_path() -> String {
    let mut path = std::env::temp_dir();
    path.push(format!("rex-sidecar-roundtrip-{}.sock", std::process::id()));
    path.display().to_string()
}

fn cleanup_socket(socket_path: &str) {
    let _ = std::fs::remove_file(socket_path);
}

struct StubServer {
    socket_path: String,
    task: JoinHandle<()>,
}

impl StubServer {
    fn spawn(socket_path: String) -> Self {
        cleanup_socket(&socket_path);
        let path = socket_path.clone();
        let task = tokio::spawn(async move {
            let _ = serve_on_socket(&path).await;
        });
        Self { socket_path, task }
    }
}

impl Drop for StubServer {
    fn drop(&mut self) {
        self.task.abort();
        cleanup_socket(&self.socket_path);
    }
}

async fn wait_ready(socket_path: &str) {
    let started = Instant::now();
    loop {
        if let Ok(mut client) = sidecar_client::connect_sidecar(socket_path).await {
            if matches!(sidecar_client::health_check(&mut client).await, Ok(true)) {
                return;
            }
        }
        assert!(
            started.elapsed() < READINESS_TIMEOUT,
            "sidecar stub did not become ready"
        );
        sleep(Duration::from_millis(50)).await;
    }
}

fn uds_bind_supported() -> bool {
    let socket_path = test_socket_path();
    cleanup_socket(&socket_path);
    let probe = std::os::unix::net::UnixListener::bind(&socket_path);
    match probe {
        Ok(listener) => {
            drop(listener);
            cleanup_socket(&socket_path);
            true
        }
        Err(err) => err.kind() != std::io::ErrorKind::PermissionDenied,
    }
}

#[tokio::test]
#[serial]
async fn sidecar_health_and_run_turn_roundtrip() {
    if !uds_bind_supported() {
        eprintln!("skipping sidecar roundtrip: UDS bind not permitted in this environment");
        return;
    }
    let socket_path = test_socket_path();
    let _stub = StubServer::spawn(socket_path.clone());
    wait_ready(&socket_path).await;

    let result = timeout(RUN_TIMEOUT, async {
        let mut client = sidecar_client::connect_sidecar(&socket_path)
            .await
            .expect("connect sidecar");
        sidecar_client::run_turn_collect(&mut client, "hello sidecar", "agent", "")
            .await
            .expect("run turn")
    })
    .await
    .expect("run_turn timed out");

    assert!(!result.is_empty(), "expected at least one chunk");
    let terminal = result.last().expect("terminal chunk");
    assert!(terminal.done, "last chunk must be done");
    let text: String = result
        .iter()
        .filter(|c| !c.done)
        .map(|c| c.text.as_str())
        .collect();
    assert!(
        text.contains("[broker.inference error"),
        "run_turn without daemon should surface broker error, got: {text}"
    );
}

#[tokio::test]
#[serial]
async fn sidecar_run_turn_stream_yields_incremental_chunks() {
    if !uds_bind_supported() {
        eprintln!("skipping sidecar stream test: UDS bind not permitted in this environment");
        return;
    }
    let socket_path = test_socket_path();
    let _stub = StubServer::spawn(socket_path.clone());
    wait_ready(&socket_path).await;

    timeout(RUN_TIMEOUT, async {
        let mut client = sidecar_client::connect_sidecar(&socket_path)
            .await
            .expect("connect sidecar");
        let mut stream = sidecar_client::run_turn_stream(&mut client, "hello sidecar", "agent", "")
            .await
            .expect("run turn stream");
        let first = stream
            .next()
            .await
            .expect("first chunk")
            .expect("first chunk ok");
        let first_at = Instant::now();
        let second = timeout(Duration::from_millis(250), stream.next())
            .await
            .expect("second chunk should arrive incrementally")
            .expect("second chunk present")
            .expect("second chunk ok");
        assert!(
            first_at.elapsed() < Duration::from_millis(200),
            "expected incremental stream delivery"
        );
        assert!(
            !first.text.is_empty() || first.done,
            "expected first chunk content or terminal"
        );
        let _ = second;
    })
    .await
    .expect("run_turn stream timed out");
}
