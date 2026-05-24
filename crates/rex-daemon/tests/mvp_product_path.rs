//! MVP product-path smoke: supervised sidecar + brokered HTTP inference (loopback fixture).
//! Complements `uds_e2e` (harness/direct) and `sidecar_roundtrip` (sidecar only).

use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use hyper_util::rt::TokioIo;
use rex_proto::rex::v1::rex_service_client::RexServiceClient;
use rex_proto::rex::v1::StreamInferenceRequest;
use serial_test::serial;
use tokio::net::UnixStream;
use tokio::time::{sleep, timeout, Instant};
use tonic::transport::Endpoint;
use tower::service_fn;

#[path = "../src/settings.rs"]
mod settings;
mod support;

use support::config::{
    install_rex_config, loaded_from_config, product_path_config, rex_root_path,
    sidecar_required_missing_binary_config,
};
use support::openai_compat_sse::{
    spawn_loopback_openai_compat_sse_fixture, spawn_loopback_openai_compat_sse_fixture_echo_model,
};

#[allow(dead_code)]
#[path = "../src/access_policy.rs"]
mod access_policy;
#[allow(dead_code)]
#[path = "../src/adapters.rs"]
mod adapters;
#[allow(dead_code)]
#[path = "../src/approvals.rs"]
mod approvals;
#[allow(dead_code)]
#[path = "../src/broker.rs"]
mod broker;
#[allow(dead_code)]
#[path = "../src/domain.rs"]
mod domain;
#[allow(dead_code)]
#[path = "../src/http_openai_compat.rs"]
mod http_openai_compat;
#[path = "../src/l1_cache.rs"]
mod l1_cache;
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

const READINESS_TIMEOUT: Duration = Duration::from_secs(8);
const CONNECT_TIMEOUT: Duration = Duration::from_millis(250);
const STREAM_TIMEOUT: Duration = Duration::from_secs(5);

struct ProductPathEnv {
    _rex_root: support::config::RexRootGuard,
}

fn init_product_path_settings(cfg: rex_config::RexConfig) -> ProductPathEnv {
    settings::reset_for_test();
    let guard = install_rex_config(cfg.clone());
    let root = rex_root_path(&guard);
    settings::init_for_test(loaded_from_config(cfg, &root));
    ProductPathEnv { _rex_root: guard }
}

fn restore_product_path_env(_saved: ProductPathEnv) {
    settings::reset_for_test();
}

fn temp_socket_path(label: &str) -> String {
    let mut path = std::env::temp_dir();
    path.push(format!("rex-mvp-{label}-{}.sock", std::process::id()));
    path.display().to_string()
}

fn cleanup_socket(socket_path: &str) {
    let _ = std::fs::remove_file(socket_path);
}

fn uds_bind_supported() -> bool {
    let socket_path = temp_socket_path("probe");
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

fn stub_binary_path() -> String {
    for key in [
        "CARGO_BIN_EXE_rex-sidecar-stub",
        "CARGO_BIN_EXE_rex_sidecar_stub",
    ] {
        if let Ok(path) = std::env::var(key) {
            if !path.contains("placeholder:") && PathBuf::from(&path).exists() {
                return path;
            }
        }
    }
    if let Some(path) = option_env!("CARGO_BIN_EXE_rex-sidecar-stub") {
        if !path.contains("placeholder:") && PathBuf::from(path).exists() {
            return path.to_string();
        }
    }
    let target_dir = std::env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target"));
    for profile in ["debug", "release"] {
        let candidate = target_dir.join(profile).join("rex-sidecar-stub");
        if candidate.exists() {
            return candidate.display().to_string();
        }
    }
    panic!(
        "rex-sidecar-stub binary not found; run: cargo build -p rex-sidecar-stub (target_dir={})",
        target_dir.display()
    );
}

fn configure_product_path(
    daemon_socket: &str,
    sidecar_socket: &str,
    workspace: &str,
    http_base_url: &str,
) -> ProductPathEnv {
    init_product_path_settings(product_path_config(
        daemon_socket,
        sidecar_socket,
        workspace,
        http_base_url,
        &stub_binary_path(),
    ))
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

async fn collect_stream_text(
    client: &mut RexServiceClient<tonic::transport::Channel>,
    prompt: &str,
    mode: &str,
    model: &str,
) -> String {
    let response = client
        .stream_inference(StreamInferenceRequest {
            prompt: prompt.to_string(),
            mode: mode.to_string(),
            model: model.to_string(),
            ..Default::default()
        })
        .await
        .expect("stream request should succeed")
        .into_inner();
    let mut stream = response;
    let mut text = String::new();
    while let Some(chunk) = stream.message().await.expect("stream read should succeed") {
        if !chunk.done {
            text.push_str(&chunk.text);
        }
    }
    text
}

#[tokio::test]
#[serial]
async fn mvp_product_path_sidecar_stream_and_brokered_read() {
    if !uds_bind_supported() {
        eprintln!("skipping mvp_product_path: UDS bind not permitted");
        return;
    }

    let daemon_socket = temp_socket_path("daemon");
    let sidecar_socket = temp_socket_path("sidecar");
    cleanup_socket(&daemon_socket);
    cleanup_socket(&sidecar_socket);

    let workspace = std::env::temp_dir().join(format!("rex-mvp-ws-{}", std::process::id()));
    fs::create_dir_all(&workspace).expect("workspace dir");
    fs::write(workspace.join("hello.txt"), "broker-read-ok").expect("fixture file");

    let http_addr = spawn_loopback_openai_compat_sse_fixture().await;
    let http_base = format!("http://{http_addr}");
    let saved = configure_product_path(
        &daemon_socket,
        &sidecar_socket,
        &workspace.display().to_string(),
        &http_base,
    );

    let daemon_socket_task = daemon_socket.clone();
    let daemon = tokio::spawn(async move {
        runtime::run_daemon_on_socket(&daemon_socket_task)
            .await
            .expect("daemon should run");
    });
    wait_for_daemon_ready(&daemon_socket).await;

    let mut client = connect_client(&daemon_socket)
        .await
        .expect("connect daemon");
    let agent_text = timeout(
        STREAM_TIMEOUT,
        collect_stream_text(&mut client, "hello mvp", "agent", ""),
    )
    .await
    .expect("agent stream timed out");
    assert!(
        agent_text.contains("hello stub"),
        "expected brokered HTTP inference via sidecar, got: {agent_text}"
    );

    let read_prompt = "inspect __rex_read:hello.txt".to_string();
    let read_text = timeout(
        STREAM_TIMEOUT,
        collect_stream_text(&mut client, &read_prompt, "agent", ""),
    )
    .await
    .expect("broker read stream timed out");
    assert!(
        read_text.contains("broker-read-ok"),
        "expected brokered fs.read content, got: {read_text}"
    );

    fs::write(workspace.join(".env"), "secret").expect("write secrets");
    let deny_prompt = "inspect __rex_read:.env".to_string();
    let deny_text = timeout(
        STREAM_TIMEOUT,
        collect_stream_text(&mut client, &deny_prompt, "agent", ""),
    )
    .await
    .expect("policy deny stream timed out");
    assert!(
        deny_text.to_ascii_lowercase().contains("protected_path")
            || deny_text.contains("fs.read error"),
        "expected access policy deny for .env, got: {deny_text}"
    );

    let list_prompt = "inspect __rex_list:".to_string();
    let list_text = timeout(
        STREAM_TIMEOUT,
        collect_stream_text(&mut client, &list_prompt, "agent", ""),
    )
    .await
    .expect("broker list stream timed out");
    assert!(
        list_text.contains("hello.txt"),
        "expected brokered fs.list content, got: {list_text}"
    );

    let http_addr_model = spawn_loopback_openai_compat_sse_fixture_echo_model().await;
    let cfg = product_path_config(
        &daemon_socket,
        &sidecar_socket,
        &workspace.display().to_string(),
        &format!("http://{http_addr_model}"),
        &stub_binary_path(),
    );
    settings::reset_for_test();
    settings::init_for_test(loaded_from_config(
        cfg.clone(),
        &rex_root_path(&saved._rex_root),
    ));
    let model_text = timeout(
        STREAM_TIMEOUT,
        collect_stream_text(&mut client, "hello model", "agent", "custom-model-id"),
    )
    .await
    .expect("model override stream timed out");
    assert!(
        model_text.contains("model=custom-model-id"),
        "expected request model override in brokered inference, got: {model_text}"
    );

    daemon.abort();
    let _ = daemon.await;
    cleanup_socket(&daemon_socket);
    cleanup_socket(&sidecar_socket);
    let _ = fs::remove_dir_all(&workspace);
    restore_product_path_env(saved);
}

#[tokio::test]
#[serial]
async fn mvp_product_path_sidecar_required_clear_error_when_binary_missing() {
    if !uds_bind_supported() {
        eprintln!("skipping mvp_product_path sidecar error: UDS bind not permitted");
        return;
    }

    let daemon_socket = temp_socket_path("daemon-err");
    let sidecar_socket = temp_socket_path("sidecar-err");
    cleanup_socket(&daemon_socket);
    let saved = init_product_path_settings(sidecar_required_missing_binary_config(
        &daemon_socket,
        &sidecar_socket,
        "/nonexistent/rex-sidecar-stub-for-mvp-test",
    ));

    let daemon_socket_task = daemon_socket.clone();
    let result = timeout(
        READINESS_TIMEOUT,
        runtime::run_daemon_on_socket(&daemon_socket_task),
    )
    .await;

    match result {
        Ok(Ok(())) => panic!("daemon should not start when required sidecar binary is missing"),
        Ok(Err(err)) => {
            let msg = err.to_string().to_ascii_lowercase();
            assert!(
                msg.contains("sidecar"),
                "expected sidecar-related error, got: {err}"
            );
        }
        Err(_) => panic!("daemon start should fail quickly, not hang"),
    }

    cleanup_socket(&daemon_socket);
    restore_product_path_env(saved);
}
