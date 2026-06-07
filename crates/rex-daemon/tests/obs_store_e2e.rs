use std::sync::Arc;
use std::time::Duration;

use hyper_util::rt::TokioIo;
use rex_config::RexConfig;
use rex_obs_store::{ObsStore, StorePort};
use rex_proto::rex::v1::rex_service_client::RexServiceClient;
use rex_proto::rex::v1::StreamInferenceRequest;
use serial_test::serial;
use tokio::net::UnixStream;
use tokio::time::sleep;
use tonic::transport::Endpoint;
use tower::service_fn;

#[path = "../src/settings.rs"]
mod settings;
mod support;

use support::config::{install_rex_config, mock_e2e_config, rex_root_path};

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
#[path = "../src/broker_inference.rs"]
mod broker_inference;
#[allow(dead_code)]
#[path = "../src/domain.rs"]
mod domain;
#[allow(dead_code)]
#[path = "../src/gateway_supervisor.rs"]
mod gateway_supervisor;
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
#[path = "../src/service.rs"]
mod service;
#[path = "../src/sidecar_client.rs"]
mod sidecar_client;
#[allow(dead_code)]
#[path = "../src/sidecar_config.rs"]
mod sidecar_config;
#[path = "../src/sidecar_observability.rs"]
mod sidecar_observability;
#[allow(dead_code)]
#[path = "../src/supervisor.rs"]
mod supervisor;
#[allow(dead_code)]
#[path = "../src/turn_correlation.rs"]
mod turn_correlation;

fn mock_e2e_with_observability() -> RexConfig {
    let mut cfg = mock_e2e_config();
    cfg.observability.enabled = Some(true);
    cfg.observability.store.path = "obs/store.sqlite".to_string();
    cfg
}

fn test_socket_path() -> String {
    let mut path = std::env::temp_dir();
    path.push(format!("rex-obs-e2e-{}.sock", std::process::id()));
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
    let endpoint =
        Endpoint::try_from("http://[::]:50051")?.connect_timeout(Duration::from_millis(250));
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
    let started = std::time::Instant::now();
    loop {
        if connect_client(socket_path).await.is_ok() {
            return;
        }
        assert!(
            started.elapsed() < Duration::from_secs(4),
            "daemon did not become ready"
        );
        sleep(Duration::from_millis(50)).await;
    }
}

#[tokio::test]
#[serial]
async fn observability_enabled_appends_stream_row_on_terminal() {
    if !uds_bind_supported() {
        eprintln!("Skipping obs store e2e: sandbox does not allow unix socket bind");
        return;
    }

    settings::reset_for_test();
    let cfg = mock_e2e_with_observability();
    let guard = install_rex_config(cfg.clone());
    let root = rex_root_path(&guard);
    settings::init_for_test(Arc::new(rex_config::LoadedConfig {
        rex_root: root.clone(),
        global_path: Some(root.join("config.json")),
        project_path: None,
        effective: cfg,
    }));

    let socket_path = test_socket_path();
    cleanup_socket(&socket_path);
    let daemon_socket = socket_path.clone();
    let daemon = tokio::spawn(async move {
        runtime::run_daemon_on_socket(&daemon_socket)
            .await
            .expect("daemon should start");
    });
    wait_for_daemon_ready(&socket_path).await;

    let mut client = connect_client(&socket_path)
        .await
        .expect("client should connect");
    let response = client
        .stream_inference(StreamInferenceRequest {
            prompt: "obs store e2e".to_string(),
            ..Default::default()
        })
        .await
        .expect("stream should succeed");
    let mut stream = response.into_inner();
    while stream.message().await.expect("read chunk").is_some() {}

    sleep(Duration::from_millis(200)).await;

    let store_path = root.join("obs/store.sqlite");
    assert!(store_path.is_file(), "store file should exist");
    let store = ObsStore::open(&store_path).expect("open store");
    assert_eq!(store.stream_count().expect("count"), 1);
    assert_eq!(store.config_snapshot_count().expect("snapshots"), 1);

    daemon.abort();
    cleanup_socket(&socket_path);
    settings::reset_for_test();
}
