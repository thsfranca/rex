//! Builtin rex-sidecar-stub: gRPC health + RunTurn without daemon (no live LLM).

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

use serial_test::serial;
use tokio::time::{sleep, timeout, Instant};

#[allow(dead_code)]
#[path = "../src/settings.rs"]
mod settings;
#[allow(dead_code)]
#[path = "../src/sidecar_client.rs"]
mod sidecar_client;
#[path = "../src/turn_correlation.rs"]
mod turn_correlation;

const READINESS_TIMEOUT: Duration = Duration::from_secs(12);
const RUN_TIMEOUT: Duration = Duration::from_secs(8);

fn builtin_sidecar_smoke_enabled() -> bool {
    matches!(
        std::env::var("REX_RUN_BUILTIN_SIDECAR_SMOKE").as_deref(),
        Ok("1") | Ok("true") | Ok("yes")
    )
}

fn test_socket_path(label: &str) -> String {
    let mut path = std::env::temp_dir();
    path.push(format!("rex-stub-{}-{}.sock", label, std::process::id()));
    path.display().to_string()
}

fn cleanup_socket(socket_path: &str) {
    let _ = fs::remove_file(socket_path);
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

fn stub_binary_path() -> String {
    if let Ok(path) = std::env::var("REX_SIDECAR_BINARY") {
        let trimmed = path.trim();
        if !trimmed.is_empty() && PathBuf::from(&trimmed).exists() {
            return trimmed.to_string();
        }
    }
    if let Some(path) = option_env!("CARGO_BIN_EXE_rex-sidecar-stub") {
        return path.to_string();
    }
    let target_dir = std::env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| repo_root().join("target"));
    for profile in ["debug", "release"] {
        let candidate = target_dir.join(profile).join("rex-sidecar-stub");
        if candidate.exists() {
            return candidate.display().to_string();
        }
    }
    panic!("rex-sidecar-stub binary not found; run: cargo build -p rex-sidecar-stub");
}

struct StubServer {
    child: std::process::Child,
}

impl StubServer {
    fn spawn(socket_path: String) -> Self {
        cleanup_socket(&socket_path);
        let binary = stub_binary_path();
        let child = Command::new(&binary)
            .env("REX_SIDECAR_SOCKET", &socket_path)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap_or_else(|err| panic!("spawn rex-sidecar-stub from {binary}: {err}"));
        Self { child }
    }
}

impl Drop for StubServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
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
            panic!("rex-sidecar-stub exited before ready: {detail}");
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
            "rex-sidecar-stub did not become ready (last error: {})",
            last_connect_err.as_deref().unwrap_or("unknown")
        );
        sleep(Duration::from_millis(50)).await;
    }
}

#[tokio::test]
#[serial]
async fn stub_sidecar_health_and_capabilities() {
    if !builtin_sidecar_smoke_enabled() {
        eprintln!(
            "skipping stub smoke: set REX_RUN_BUILTIN_SIDECAR_SMOKE=1 (see scripts/ci/run_builtin_sidecar_checks.sh)"
        );
        return;
    }
    if !uds_bind_supported() {
        eprintln!("skipping stub smoke: UDS bind not permitted");
        return;
    }

    let socket_path = test_socket_path("health");
    let mut server = StubServer::spawn(socket_path.clone());
    wait_sidecar_ready(&socket_path, &mut server.child).await;

    let mut client = sidecar_client::connect_sidecar(&socket_path)
        .await
        .expect("connect stub sidecar");
    assert!(sidecar_client::health_check(&mut client)
        .await
        .expect("health rpc"));
    cleanup_socket(&socket_path);
}

#[tokio::test]
#[serial]
async fn stub_sidecar_run_turn_broker_error_without_daemon() {
    if !builtin_sidecar_smoke_enabled() {
        eprintln!(
            "skipping stub smoke: set REX_RUN_BUILTIN_SIDECAR_SMOKE=1 (see scripts/ci/run_builtin_sidecar_checks.sh)"
        );
        return;
    }
    if !uds_bind_supported() {
        eprintln!("skipping stub smoke: UDS bind not permitted");
        return;
    }

    let socket_path = test_socket_path("run-turn");
    let mut server = StubServer::spawn(socket_path.clone());
    wait_sidecar_ready(&socket_path, &mut server.child).await;

    let result = timeout(RUN_TIMEOUT, async {
        let mut client = sidecar_client::connect_sidecar(&socket_path)
            .await
            .expect("connect stub sidecar");
        sidecar_client::run_turn_collect(
            &mut client,
            "hello stub",
            "ask",
            "",
            &sidecar_client::TurnCorrelation {
                turn_id: String::new(),
                context_revision: String::new(),
            },
            Vec::new(),
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
        text.contains("[broker.inference error"),
        "expected broker error without daemon, got: {text}"
    );
    cleanup_socket(&socket_path);
}
