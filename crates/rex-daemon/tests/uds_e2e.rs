use std::sync::Arc;
use std::time::Duration;

use hyper_util::rt::TokioIo;
use rex_proto::rex::v1::rex_service_client::RexServiceClient;
use rex_proto::rex::v1::{GetSystemStatusRequest, StreamInferenceRequest};
use serial_test::serial;
use tokio::net::UnixStream;
use tokio::time::{sleep, timeout, Instant};
use tonic::transport::Endpoint;
use tower::service_fn;

#[path = "../src/settings.rs"]
mod settings;
mod support;

use support::config::{
    cursor_cli_e2e_config, install_rex_config, mock_e2e_config, mock_e2e_with_approvals,
    rex_root_path,
};

#[allow(dead_code)]
#[path = "../src/access_policy.rs"]
mod access_policy;
#[allow(dead_code)]
#[path = "../src/activity.rs"]
mod activity;
#[allow(dead_code)]
#[path = "../src/adapters.rs"]
mod adapters;
#[allow(dead_code)]
#[path = "../src/advisory_intent.rs"]
mod advisory_intent;
#[allow(dead_code)]
#[path = "../src/approvals.rs"]
mod approvals;
#[allow(dead_code)]
#[path = "../src/broker.rs"]
mod broker;
#[allow(dead_code)]
#[path = "../src/broker_inference.rs"]
mod broker_inference;
#[allow(dead_code)]
#[path = "../src/capability_client.rs"]
mod capability_client;
#[allow(dead_code)]
#[path = "../src/domain.rs"]
mod domain;
#[path = "../src/economics_record.rs"]
mod economics_record;
#[allow(dead_code)]
#[path = "../src/gateway_supervisor.rs"]
mod gateway_supervisor;
#[allow(dead_code)]
#[path = "../src/omlx_supervisor.rs"]
mod omlx_supervisor;
#[allow(dead_code)]
#[path = "../src/http_openai_compat.rs"]
mod http_openai_compat;
#[path = "../src/l1_cache.rs"]
mod l1_cache;
#[allow(dead_code)]
#[path = "../src/observability.rs"]
mod observability;
#[path = "../src/ollama_capability.rs"]
mod ollama_capability;
#[allow(dead_code)]
#[path = "../src/otlp_metrics.rs"]
mod otlp_metrics;
#[allow(dead_code)]
#[path = "../src/plugins.rs"]
mod plugins;
#[allow(dead_code)]
#[path = "../src/policy.rs"]
mod policy;
#[allow(dead_code)]
#[path = "../src/routing.rs"]
mod routing;
#[allow(dead_code)]
#[path = "../src/runtime.rs"]
mod runtime;
#[allow(dead_code)]
#[path = "../src/tool_approval.rs"]
mod tool_approval;
#[allow(dead_code)]
#[path = "../src/service.rs"]
mod service;
#[path = "../src/sidecar_client.rs"]
mod sidecar_client;
#[allow(dead_code)]
#[path = "../src/sidecar_config.rs"]
mod sidecar_config;
#[allow(dead_code)]
#[path = "../src/supervisor.rs"]
mod supervisor;
#[allow(dead_code)]
#[path = "../src/turn_correlation.rs"]
mod turn_correlation;

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

struct E2eInferenceEnv {
    _rex_root: support::config::RexRootGuard,
}

fn init_daemon_settings(cfg: rex_config::RexConfig) -> E2eInferenceEnv {
    settings::reset_for_test();
    let guard = install_rex_config(cfg.clone());
    let root = rex_root_path(&guard);
    let loaded = Arc::new(
        rex_config::LoadedConfig::from_effective(
            root.clone(),
            Some(root.join("config.json")),
            None,
            cfg,
        )
        .expect("test loaded config"),
    );
    settings::init_for_test(loaded);
    E2eInferenceEnv { _rex_root: guard }
}

/// UDS e2e tests use the **mock** runtime so they do not require a live HTTP backend.
fn set_e2e_mock_runtime() -> E2eInferenceEnv {
    init_daemon_settings(mock_e2e_config())
}

fn restore_inference_runtime(_saved: E2eInferenceEnv) {
    settings::reset_for_test();
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

    let prev_runtime = set_e2e_mock_runtime();

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
    assert_eq!(status.lifecycle_state, "idle");
    assert_eq!(status.idle_seconds, 0);

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

    restore_inference_runtime(prev_runtime);
}

#[tokio::test]
#[serial]
async fn status_probe_prevents_idle_shutdown() {
    if !uds_bind_supported() {
        eprintln!("Skipping UDS e2e: sandbox does not allow unix socket bind");
        return;
    }

    let mut cfg = mock_e2e_config();
    cfg.daemon.idle_shutdown_secs = Some(2);
    let _runtime = init_daemon_settings(cfg);

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
    for _ in 0..4 {
        client
            .get_system_status(GetSystemStatusRequest {})
            .await
            .expect("status probe should succeed");
        sleep(Duration::from_millis(700)).await;
    }
    assert!(
        connect_client(&socket_path).await.is_ok(),
        "daemon should remain up while status probes continue"
    );

    daemon.abort();
    let _ = daemon.await;
    cleanup_socket(&socket_path);
    settings::reset_for_test();
}

#[tokio::test]
#[serial]
async fn daemon_shuts_down_after_idle_budget_without_contact() {
    if !uds_bind_supported() {
        eprintln!("Skipping UDS e2e: sandbox does not allow unix socket bind");
        return;
    }

    let mut cfg = mock_e2e_config();
    cfg.daemon.idle_shutdown_secs = Some(2);
    let _runtime = init_daemon_settings(cfg);

    let socket_path = test_socket_path();
    cleanup_socket(&socket_path);
    let daemon_socket = socket_path.clone();
    let daemon = tokio::spawn(async move {
        runtime::run_daemon_on_socket(&daemon_socket)
            .await
            .expect("daemon should exit cleanly after idle shutdown");
    });
    wait_for_daemon_ready(&socket_path).await;

    sleep(Duration::from_secs(3)).await;

    assert!(
        connect_client(&socket_path).await.is_err(),
        "daemon socket should be closed after idle shutdown budget"
    );
    assert!(
        !std::path::Path::new(&socket_path).exists(),
        "daemon should remove its socket on idle shutdown"
    );

    timeout(Duration::from_secs(2), daemon)
        .await
        .expect("daemon task should finish")
        .expect("daemon join");
    cleanup_socket(&socket_path);
    settings::reset_for_test();
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

    let _runtime = init_daemon_settings(cursor_cli_e2e_config(
        "printf '{\"text\":\"hello from cursor\"}\\n'",
    ));

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
    settings::reset_for_test();
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

    let prev_runtime = set_e2e_mock_runtime();
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
    restore_inference_runtime(prev_runtime);
}

#[tokio::test]
#[serial]
async fn stream_reports_terminal_error_when_daemon_interrupts() {
    if !uds_bind_supported() {
        eprintln!("Skipping UDS e2e: sandbox does not allow unix socket bind");
        return;
    }

    let prev_runtime = set_e2e_mock_runtime();
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
    restore_inference_runtime(prev_runtime);
}

/// With `agent.approvals_enabled` true, an `agent`-mode request must surface a
/// `FailedPrecondition` gRPC error matching `ENFORCEMENT_DENY_REASON` from
/// `approvals.rs`. `ask`-mode requests on the same daemon stay successful.
#[tokio::test]
#[serial]
async fn agent_mode_is_denied_when_approvals_enabled() {
    if !uds_bind_supported() {
        eprintln!("Skipping UDS e2e: sandbox does not allow unix socket bind");
        return;
    }

    let _runtime = init_daemon_settings(mock_e2e_with_approvals(true));

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
        .expect_err("agent mode must be denied when agent.approvals_enabled is true");
    assert_eq!(agent_err.code(), tonic::Code::FailedPrecondition);
    assert!(
        agent_err.message().contains(
            "agent.approvals_enabled is true and no approval context supplied for agent mode"
        ),
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
    settings::reset_for_test();
}

#[tokio::test]
#[serial]
async fn agent_mode_succeeds_with_approval_id_when_enforced() {
    if !uds_bind_supported() {
        eprintln!("Skipping UDS e2e: sandbox does not allow unix socket bind");
        return;
    }

    let _runtime = init_daemon_settings(mock_e2e_with_approvals(true));

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
            prompt: "agent approved".to_string(),
            mode: "agent".to_string(),
            approval_id: "apr-e2e-1".to_string(),
            ..Default::default()
        })
        .await
        .expect("agent with approval_id should succeed");
    let mut stream = response.into_inner();
    let mut saw_done = false;
    while let Some(chunk) = stream.message().await.expect("stream read") {
        saw_done = chunk.done;
    }
    assert!(saw_done);

    daemon.abort();
    let _ = daemon.await;
    cleanup_socket(&socket_path);
    settings::reset_for_test();
}

/// With `agent.approvals_enabled` false, an `agent`-mode request must succeed
/// (preserves today's default behavior).
#[tokio::test]
#[serial]
async fn agent_mode_is_allowed_when_approvals_disabled() {
    if !uds_bind_supported() {
        eprintln!("Skipping UDS e2e: sandbox does not allow unix socket bind");
        return;
    }

    let _runtime = init_daemon_settings(mock_e2e_with_approvals(false));

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
        .expect("agent mode should succeed when approvals are disabled in config");
    let mut stream = response.into_inner();
    let mut saw_done = false;
    while let Some(chunk) = stream.message().await.expect("stream read should succeed") {
        saw_done = chunk.done;
    }
    assert!(saw_done, "agent stream must reach the terminal done chunk");

    daemon.abort();
    let _ = daemon.await;
    cleanup_socket(&socket_path);
    settings::reset_for_test();
}
