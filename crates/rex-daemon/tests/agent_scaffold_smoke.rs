//! R017/R018 smoke: Python rex-agent sidecar gRPC + broker + LangGraph path (no live LLM).

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
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
    install_rex_config, loaded_from_config, product_path_config_named, rex_root_path,
};
use support::openai_compat_sse::spawn_loopback_openai_compat_sse_fixture;

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
#[path = "../src/turn_correlation.rs"]
mod turn_correlation;

/// Set by `scripts/ci/run_rex_agent_checks.sh`. Skips Python/proto setup during workspace `cargo nextest`.
fn agent_smoke_enabled() -> bool {
    matches!(
        std::env::var("REX_RUN_AGENT_SMOKE").as_deref(),
        Ok("1") | Ok("true") | Ok("yes")
    )
}

const READINESS_TIMEOUT: Duration = Duration::from_secs(12);
const RUN_TIMEOUT: Duration = Duration::from_secs(8);
const STREAM_TIMEOUT: Duration = Duration::from_secs(8);
const CONNECT_TIMEOUT: Duration = Duration::from_millis(250);

fn test_socket_path(label: &str) -> String {
    let mut path = std::env::temp_dir();
    path.push(format!("rex-agt-{}-{}.sock", label, std::process::id()));
    path.display().to_string()
}

fn cleanup_socket(socket_path: &str) {
    let _ = std::fs::remove_file(socket_path);
}

fn uds_bind_supported() -> bool {
    let socket_path = test_socket_path("probe");
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

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn agent_launcher_path() -> PathBuf {
    repo_root().join("sidecars/rex-agent/rex-agent")
}

fn agent_binary_path() -> String {
    if let Ok(path) = std::env::var("REX_AGENT_BINARY") {
        let trimmed = path.trim();
        if !trimmed.is_empty() && PathBuf::from(trimmed).exists() {
            return trimmed.to_string();
        }
    }
    let launcher = agent_launcher_path();
    if launcher.is_file() {
        return launcher.display().to_string();
    }
    let which = Command::new("sh")
        .arg("-lc")
        .arg("command -v rex-agent")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && PathBuf::from(s).exists());
    if let Some(path) = which {
        return path;
    }
    panic!(
        "rex-agent binary not found; set REX_AGENT_BINARY, run `pip install -e sidecars/rex-agent`, \
         or use {}",
        launcher.display()
    );
}

fn pythonpath_for_rex_root(rex_root: &std::path::Path) -> String {
    let gen = rex_root.join("proto/gen");
    let agent_src = repo_root().join("sidecars/rex-agent/src");
    format!("{}:{}", gen.display(), agent_src.display())
}

struct AgentServer {
    socket_path: String,
    child: std::process::Child,
}

impl AgentServer {
    fn spawn(socket_path: String, rex_root: &std::path::Path, daemon_socket: Option<&str>) -> Self {
        cleanup_socket(&socket_path);
        let pythonpath = pythonpath_for_rex_root(rex_root);
        let mut cmd = Command::new("python3");
        cmd.arg("-m")
            .arg("rex_agent")
            .env("REX_ROOT", rex_root)
            .env("REX_SIDECAR_SOCKET", &socket_path)
            .env("PYTHONPATH", &pythonpath)
            .stdout(Stdio::null())
            .stderr(Stdio::piped());
        if let Some(daemon) = daemon_socket {
            cmd.env("REX_DAEMON_SOCKET", daemon);
        }
        let child = cmd.spawn().expect("spawn rex-agent");
        Self { socket_path, child }
    }
}

impl Drop for AgentServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        cleanup_socket(&self.socket_path);
    }
}

#[allow(unused_assignments)]
async fn wait_sidecar_ready(socket_path: &str, child: &mut std::process::Child) {
    let started = Instant::now();
    let mut last_connect_err: Option<String> = None;
    loop {
        if let Ok(Some(status)) = child.try_wait() {
            let mut detail = format!("exit {status}");
            if let Some(mut stderr) = child.stderr.take() {
                use std::io::Read;
                let mut buf = String::new();
                let _ = stderr.read_to_string(&mut buf);
                if !buf.is_empty() {
                    detail = format!("{detail}: {buf}");
                }
            }
            panic!("rex-agent exited before ready: {detail}");
        }
        match sidecar_client::connect_sidecar(socket_path).await {
            Ok(mut client) => {
                if matches!(sidecar_client::health_check(&mut client).await, Ok(true)) {
                    return;
                }
                last_connect_err = Some("health_check returned false or error".to_string());
            }
            Err(err) => last_connect_err = Some(err.to_string()),
        }
        assert!(
            started.elapsed() < READINESS_TIMEOUT,
            "rex-agent did not become ready (last error: {})",
            last_connect_err.as_deref().unwrap_or("unknown")
        );
        sleep(Duration::from_millis(50)).await;
    }
}

async fn connect_daemon(
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

async fn wait_daemon_ready(socket_path: &str) {
    let started = Instant::now();
    loop {
        if connect_daemon(socket_path).await.is_ok() {
            return;
        }
        assert!(
            started.elapsed() < READINESS_TIMEOUT,
            "daemon did not become ready"
        );
        sleep(Duration::from_millis(50)).await;
    }
}

fn install_proto_stubs(rex_root: &std::path::Path) {
    let rex_binary = std::env::var("CARGO_BIN_EXE_rex").unwrap_or_else(|_| {
        let target = std::env::var("CARGO_TARGET_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| repo_root().join("target"));
        for profile in ["debug", "release"] {
            let candidate = target.join(profile).join("rex");
            if candidate.exists() {
                return candidate.display().to_string();
            }
        }
        panic!("rex binary not found; build workspace first");
    });
    let proto_src = repo_root().join("proto");
    let status = Command::new(&rex_binary)
        .args(["proto", "install"])
        .env("REX_ROOT", rex_root)
        .env("REX_PROTO_SRC", &proto_src)
        .status()
        .expect("rex proto install");
    assert!(status.success(), "rex proto install failed");
}

#[tokio::test]
#[serial]
async fn agent_sidecar_health_and_broker_error_without_daemon() {
    if !agent_smoke_enabled() {
        eprintln!("skipping agent smoke: set REX_RUN_AGENT_SMOKE=1 (see scripts/ci/run_rex_agent_checks.sh)");
        return;
    }
    if !uds_bind_supported() {
        eprintln!("skipping agent scaffold: UDS bind not permitted");
        return;
    }
    let rex_root = tempfile::tempdir().expect("temp rex root");
    install_proto_stubs(rex_root.path());

    let socket_path = test_socket_path("sidecar-only");
    let mut agent = AgentServer::spawn(socket_path.clone(), rex_root.path(), None);
    wait_sidecar_ready(&socket_path, &mut agent.child).await;

    let result = timeout(RUN_TIMEOUT, async {
        let mut client = sidecar_client::connect_sidecar(&socket_path)
            .await
            .expect("connect sidecar");
        sidecar_client::run_turn_collect(
            &mut client,
            "hello agent",
            "agent",
            "",
            &sidecar_client::TurnCorrelation {
                turn_id: "turn-agent-smoke".to_string(),
                context_revision: String::new(),
            },
        )
        .await
        .expect("run turn")
    })
    .await
    .expect("run_turn timed out");

    assert!(!result.is_empty());
    let terminal = result.last().expect("terminal");
    assert!(terminal.done);
    let text: String = result
        .iter()
        .filter(|c| !c.done)
        .map(|c| c.text.as_str())
        .collect();
    assert!(
        text.contains("[broker.inference error") || text.contains("Inference failed"),
        "expected broker error without daemon, got: {text}"
    );
}

#[tokio::test]
#[serial]
async fn agent_product_path_stream_inference_via_supervisor() {
    if !agent_smoke_enabled() {
        eprintln!("skipping agent smoke: set REX_RUN_AGENT_SMOKE=1 (see scripts/ci/run_rex_agent_checks.sh)");
        return;
    }
    if !uds_bind_supported() {
        eprintln!("skipping agent product path: UDS bind not permitted");
        return;
    }

    let daemon_socket = test_socket_path("daemon");
    let sidecar_socket = test_socket_path("sidecar");
    cleanup_socket(&daemon_socket);
    cleanup_socket(&sidecar_socket);

    let workspace = std::env::temp_dir().join(format!("rex-agent-ws-{}", std::process::id()));
    fs::create_dir_all(&workspace).expect("workspace");

    let http_addr = spawn_loopback_openai_compat_sse_fixture().await;
    let http_base = format!("http://{http_addr}");

    let agent_launcher = agent_binary_path();
    let rex_root_guard = install_rex_config(product_path_config_named(
        &daemon_socket,
        &sidecar_socket,
        &workspace.display().to_string(),
        &http_base,
        "agent",
        &agent_launcher,
    ));
    let rex_root = rex_root_path(&rex_root_guard);
    install_proto_stubs(&rex_root);

    settings::reset_for_test();
    settings::init_for_test(loaded_from_config(
        product_path_config_named(
            &daemon_socket,
            &sidecar_socket,
            &workspace.display().to_string(),
            &http_base,
            "agent",
            &agent_launcher,
        ),
        &rex_root,
    ));

    let daemon_socket_task = daemon_socket.clone();
    let daemon = tokio::spawn(async move {
        runtime::run_daemon_on_socket(&daemon_socket_task)
            .await
            .expect("daemon run");
    });
    wait_daemon_ready(&daemon_socket).await;

    let mut client = connect_daemon(&daemon_socket)
        .await
        .expect("connect daemon");
    let response = client
        .stream_inference(StreamInferenceRequest {
            prompt: "hello mvp".to_string(),
            mode: "agent".to_string(),
            model: String::new(),
            ..Default::default()
        })
        .await
        .expect("stream inference")
        .into_inner();
    let mut stream = response;
    let mut text = String::new();
    let collected = timeout(STREAM_TIMEOUT, async {
        while let Some(chunk) = stream.message().await.expect("chunk") {
            if !chunk.done {
                text.push_str(&chunk.text);
            }
        }
    })
    .await;
    assert!(collected.is_ok(), "stream timed out");
    assert!(
        text.contains("hello stub"),
        "expected brokered HTTP inference via rex-agent, got: {text}"
    );

    daemon.abort();
    let _ = daemon.await;
    cleanup_socket(&daemon_socket);
    cleanup_socket(&sidecar_socket);
}
