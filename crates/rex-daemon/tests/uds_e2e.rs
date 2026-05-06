use std::time::Duration;

use hyper_util::rt::TokioIo;
use rex_proto::rex::v1::rex_service_client::RexServiceClient;
use rex_proto::rex::v1::{GetSystemStatusRequest, StreamInferenceRequest};
use serial_test::serial;
use std::env;
use tokio::net::UnixStream;
use tokio::time::{sleep, timeout, Instant};
use tonic::transport::Endpoint;
use tower::service_fn;

#[allow(dead_code)]
#[path = "../src/adapters.rs"]
mod adapters;
#[allow(dead_code)]
#[path = "../src/approvals.rs"]
mod approvals;
#[allow(dead_code)]
#[path = "../src/domain.rs"]
mod domain;
#[path = "../src/l1_cache.rs"]
mod l1_cache;
#[allow(dead_code)]
#[path = "../src/plugins.rs"]
mod plugins;
#[allow(dead_code)]
#[path = "../src/policy.rs"]
mod policy;
#[allow(dead_code)]
#[path = "../src/runtime.rs"]
mod runtime;
#[allow(dead_code)]
#[path = "../src/service.rs"]
mod service;

const READINESS_TIMEOUT: Duration = Duration::from_secs(4);
const CONNECT_TIMEOUT: Duration = Duration::from_millis(250);
const STREAM_READ_TIMEOUT: Duration = Duration::from_secs(2);

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
            ..Default::default()
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

/// Covers the Cursor-CLI runtime with a **shell stub** (`REX_CURSOR_CLI_COMMAND`), not the real
/// `cursor-agent` binary. For optional local testing with a real CLI, see `docs/ADAPTERS.md`
/// (Local verification).
#[tokio::test]
#[serial]
async fn cursor_runtime_streams_chunks_over_uds() {
    if !uds_bind_supported() {
        eprintln!("Skipping UDS e2e: sandbox does not allow unix socket bind");
        return;
    }

    let prev_runtime = env::var("REX_INFERENCE_RUNTIME").ok();
    let prev_cmd = env::var("REX_CURSOR_CLI_COMMAND").ok();
    env::set_var("REX_INFERENCE_RUNTIME", "cursor-cli");
    env::set_var(
        "REX_CURSOR_CLI_COMMAND",
        "printf '{\"text\":\"hello from cursor\"}\\n'",
    );

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
    let response = client
        .stream_inference(StreamInferenceRequest {
            prompt: "use cursor path".to_string(),
            ..Default::default()
        })
        .await
        .expect("stream request should succeed");
    let mut stream = response.into_inner();
    let mut chunks = Vec::new();
    while let Some(chunk) = stream.message().await.expect("stream read should succeed") {
        chunks.push(chunk);
    }
    assert!(
        chunks.len() >= 2,
        "expected at least one data chunk and done chunk"
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

    if let Some(value) = prev_runtime {
        env::set_var("REX_INFERENCE_RUNTIME", value);
    } else {
        env::remove_var("REX_INFERENCE_RUNTIME");
    }
    if let Some(value) = prev_cmd {
        env::set_var("REX_CURSOR_CLI_COMMAND", value);
    } else {
        env::remove_var("REX_CURSOR_CLI_COMMAND");
    }
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

#[tokio::test]
#[serial]
async fn startup_race_recovers_and_serves_status() {
    if !uds_bind_supported() {
        eprintln!("Skipping UDS e2e: sandbox does not allow unix socket bind");
        return;
    }

    let socket_path = test_socket_path();
    cleanup_socket(&socket_path);
    let daemon_socket = socket_path.clone();
    let daemon = tokio::spawn(async move {
        // Delay startup so the test always exercises the unavailable-then-ready race window.
        sleep(Duration::from_millis(250)).await;
        runtime::run_daemon_on_socket(&daemon_socket)
            .await
            .expect("daemon runtime should run without transport error");
    });

    let started = Instant::now();
    let mut saw_unavailable = false;
    loop {
        match connect_client(&socket_path).await {
            Ok(_) => break,
            Err(_) => {
                saw_unavailable = true;
                assert!(
                    started.elapsed() < READINESS_TIMEOUT,
                    "daemon did not become ready in {:?}",
                    READINESS_TIMEOUT
                );
                sleep(Duration::from_millis(50)).await;
            }
        }
    }
    assert!(
        saw_unavailable,
        "expected at least one unavailable connection attempt before daemon readiness"
    );

    let mut client = connect_client(&socket_path)
        .await
        .expect("daemon should accept connections after ready");
    let status = client
        .get_system_status(GetSystemStatusRequest {})
        .await
        .expect("status request should succeed")
        .into_inner();
    assert!(!status.daemon_version.is_empty());

    daemon.abort();
    let _ = daemon.await;
    cleanup_socket(&socket_path);
}

#[tokio::test]
#[serial]
async fn stream_reports_terminal_error_when_daemon_interrupts() {
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
    let response = client
        .stream_inference(StreamInferenceRequest {
            prompt: "interrupt me".to_string(),
            ..Default::default()
        })
        .await
        .expect("stream request should succeed");
    let mut stream = response.into_inner();

    let first = timeout(STREAM_READ_TIMEOUT, stream.message())
        .await
        .expect("first chunk should arrive within timeout")
        .expect("stream read should not fail before interruption")
        .expect("stream should emit first chunk");
    assert!(!first.done);

    daemon.abort();
    let _ = daemon.await;
    cleanup_socket(&socket_path);

    let deadline = Instant::now() + STREAM_READ_TIMEOUT;
    loop {
        let now = Instant::now();
        assert!(
            now < deadline,
            "expected stream read error or termination after daemon abort"
        );
        let remaining = deadline - now;
        match timeout(remaining, stream.message()).await {
            Ok(Err(_)) | Ok(Ok(None)) => break,
            Ok(Ok(Some(_))) => continue,
            Err(_) => panic!("expected stream read error or termination after daemon abort"),
        }
    }
}

const APPROVALS_ENV: &str = "REX_AGENT_APPROVALS";

/// With `REX_AGENT_APPROVALS=1`, an `agent`-mode request must surface a
/// `FailedPrecondition` gRPC error matching `ENFORCEMENT_DENY_REASON` from
/// `approvals.rs`. `ask`-mode requests on the same daemon stay successful.
#[tokio::test]
#[serial]
async fn agent_mode_is_denied_when_approvals_env_is_set() {
    if !uds_bind_supported() {
        eprintln!("Skipping UDS e2e: sandbox does not allow unix socket bind");
        return;
    }

    let prev = env::var(APPROVALS_ENV).ok();
    env::set_var(APPROVALS_ENV, "1");

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

    let agent_err = client
        .stream_inference(StreamInferenceRequest {
            prompt: "agent run".to_string(),
            mode: "agent".to_string(),
            ..Default::default()
        })
        .await
        .expect_err("agent mode must be denied when REX_AGENT_APPROVALS=1");
    assert_eq!(agent_err.code(), tonic::Code::FailedPrecondition);
    assert!(
        agent_err
            .message()
            .contains("REX_AGENT_APPROVALS=1 and no approval context supplied for agent mode"),
        "deny reason should match ADR 0009 wording: {}",
        agent_err.message()
    );

    let response = client
        .stream_inference(StreamInferenceRequest {
            prompt: "ask run".to_string(),
            mode: "ask".to_string(),
            ..Default::default()
        })
        .await
        .expect("ask mode must still succeed when approvals are enforced");
    let mut stream = response.into_inner();
    let mut saw_done = false;
    while let Some(chunk) = stream.message().await.expect("stream read should succeed") {
        saw_done = chunk.done;
    }
    assert!(saw_done, "ask stream must reach the terminal done chunk");

    daemon.abort();
    let _ = daemon.await;
    cleanup_socket(&socket_path);

    if let Some(value) = prev {
        env::set_var(APPROVALS_ENV, value);
    } else {
        env::remove_var(APPROVALS_ENV);
    }
}

/// With `REX_AGENT_APPROVALS` unset, an `agent`-mode request must succeed
/// (preserves today's default behavior).
#[tokio::test]
#[serial]
async fn agent_mode_is_allowed_when_approvals_env_is_unset() {
    if !uds_bind_supported() {
        eprintln!("Skipping UDS e2e: sandbox does not allow unix socket bind");
        return;
    }

    let prev = env::var(APPROVALS_ENV).ok();
    env::remove_var(APPROVALS_ENV);

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

    let response = client
        .stream_inference(StreamInferenceRequest {
            prompt: "agent run".to_string(),
            mode: "agent".to_string(),
            ..Default::default()
        })
        .await
        .expect("agent mode should succeed when approvals env is unset");
    let mut stream = response.into_inner();
    let mut saw_done = false;
    while let Some(chunk) = stream.message().await.expect("stream read should succeed") {
        saw_done = chunk.done;
    }
    assert!(saw_done, "agent stream must reach the terminal done chunk");

    daemon.abort();
    let _ = daemon.await;
    cleanup_socket(&socket_path);

    if let Some(value) = prev {
        env::set_var(APPROVALS_ENV, value);
    }
}
