use std::time::Duration;

use hyper_util::rt::TokioIo;
use rex_proto::rex::v1::rex_service_client::RexServiceClient;
use rex_proto::rex::v1::{GetSystemStatusRequest, StreamInferenceRequest};
use serial_test::serial;
use tokio::net::UnixStream;
use tokio::time::{sleep, Instant};
use tonic::transport::Endpoint;
use tower::service_fn;

#[allow(dead_code)]
#[path = "../src/domain.rs"]
mod domain;
#[allow(dead_code)]
#[path = "../src/runtime.rs"]
mod runtime;
#[allow(dead_code)]
#[path = "../src/service.rs"]
mod service;

const READINESS_TIMEOUT: Duration = Duration::from_secs(4);
const CONNECT_TIMEOUT: Duration = Duration::from_millis(250);

fn test_socket_path() -> String {
    let mut path = std::env::temp_dir();
    path.push(format!("rex-daemon-e2e-{}.sock", std::process::id()));
    path.display().to_string()
}

fn cleanup_socket(socket_path: &str) {
    let _ = std::fs::remove_file(socket_path);
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

async fn connect_client(
    socket_path: &str,
) -> Result<RexServiceClient<tonic::transport::Channel>, tonic::transport::Error> {
    let endpoint = Endpoint::try_from("http://[::]:50051")?.connect_timeout(CONNECT_TIMEOUT);
    let socket_path = socket_path.to_string();
    let channel = endpoint
        .connect_with_connector(service_fn(move |_: tonic::transport::Uri| {
            let socket_path = socket_path.clone();
            async move { UnixStream::connect(socket_path).await.map(TokioIo::new) }
        }))
        .await?;
    Ok(RexServiceClient::new(channel))
}

async fn wait_for_daemon_ready(socket_path: &str) {
    let started = Instant::now();
    loop {
        if connect_client(socket_path).await.is_ok() {
            return;
        }
        assert!(
            started.elapsed() < READINESS_TIMEOUT,
            "daemon did not become ready in {:?}",
            READINESS_TIMEOUT
        );
        sleep(Duration::from_millis(50)).await;
    }
}

#[tokio::test]
#[serial]
async fn status_and_stream_inference_work_over_uds() {
    if !uds_bind_supported() {
        eprintln!("Skipping UDS e2e: sandbox does not allow unix socket bind");
        return;
    }

    let socket_path = test_socket_path();
    cleanup_socket(&socket_path);
    let daemon_socket = socket_path.clone();
    let daemon = tokio::spawn(async move {
        runtime::run_daemon_on_socket(&daemon_socket)
            .await
            .expect("daemon runtime should run without transport error");
    });
    wait_for_daemon_ready(&socket_path).await;

    let mut client = connect_client(&socket_path)
        .await
        .expect("daemon should accept connections");
    let status = client
        .get_system_status(GetSystemStatusRequest {})
        .await
        .expect("status request should succeed")
        .into_inner();
    assert!(!status.daemon_version.trim().is_empty());
    assert_eq!(status.active_model_id, "mock-model-v0");

    let response = client
        .stream_inference(StreamInferenceRequest {
            prompt: "hello from test".to_string(),
        })
        .await
        .expect("stream request should succeed");
    let mut stream = response.into_inner();
    let mut chunks = Vec::new();
    while let Some(chunk) = stream.message().await.expect("stream read should succeed") {
        chunks.push(chunk);
    }

    assert!(
        chunks.len() >= 3,
        "expected at least 2 data chunks and done chunk"
    );
    for (idx, chunk) in chunks.iter().enumerate() {
        assert_eq!(chunk.index, idx as u64);
    }
    let last = chunks
        .last()
        .expect("stream should return at least one chunk");
    assert!(last.done);
    assert!(last.text.is_empty());
    assert!(chunks[..chunks.len() - 1].iter().all(|chunk| !chunk.done));
    assert!(chunks[..chunks.len() - 1]
        .iter()
        .all(|chunk| !chunk.text.is_empty()));

    daemon.abort();
    let _ = daemon.await;
    cleanup_socket(&socket_path);
}

#[tokio::test]
#[serial]
async fn connect_fails_when_daemon_is_unavailable() {
    let socket_path = test_socket_path();
    cleanup_socket(&socket_path);
    let err = connect_client(&socket_path)
        .await
        .expect_err("connecting without daemon should fail");
    assert!(
        err.to_string().contains("transport error")
            || err.to_string().contains("connection refused")
            || err.to_string().contains("No such file"),
        "unexpected error message: {err}"
    );
}
