//! Managed inference gateway supervisor (mock HTTP health; no LiteLLM in CI).

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;

use serial_test::serial;

#[allow(dead_code)]
#[path = "../src/gateway_supervisor.rs"]
mod gateway_supervisor;
#[allow(dead_code)]
#[path = "../src/settings.rs"]
mod settings;

mod support;

use gateway_supervisor::{GatewaySupervisor, GatewaySupervisorConfig, GatewaySupervisorError};
use support::config::{install_rex_config, managed_gateway_config, rex_root_path};
use support::openai_compat_sse::spawn_loopback_gateway_models_fixture;

fn write_sleep_stub(root: &std::path::Path) -> std::path::PathBuf {
    let stub = root.join("gateway-stub.sh");
    fs::write(&stub, "#!/bin/sh\nexec sleep 300\n").expect("write stub");
    let mut perms = fs::metadata(&stub).expect("meta").permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&stub, perms).expect("chmod stub");
    stub
}

#[tokio::test]
#[serial]
async fn managed_gateway_passes_health_with_mock_server() {
    let addr = spawn_loopback_gateway_models_fixture().await;
    let port = addr.port();
    let _guard = install_rex_config(managed_gateway_config(port, "/bin/sleep"));
    let root = rex_root_path(&_guard);
    std::env::set_current_dir(&root).expect("chdir rex root");
    let stub = write_sleep_stub(&root);
    let stub_cmd = stub.display().to_string();
    let mut cfg = managed_gateway_config(port, &stub_cmd);
    cfg.inference.gateway.command = stub.display().to_string();
    fs::write(
        root.join("config.json"),
        serde_json::to_string_pretty(&cfg).expect("json"),
    )
    .expect("rewrite config");
    let mut loaded = rex_config::load_merged().expect("load");
    loaded.apply_effective_openai_compat_base_url();
    settings::init_for_test(Arc::new(loaded.clone()));

    let supervisor = GatewaySupervisor::new(GatewaySupervisorConfig::from_loaded(&loaded));
    supervisor
        .ensure_running()
        .await
        .expect("gateway should become healthy");
    assert!(supervisor.is_healthy().await);
    supervisor.stop().await;
    settings::reset_for_test();
}

#[tokio::test]
#[serial]
async fn managed_gateway_fails_when_command_missing() {
    let _guard = install_rex_config(managed_gateway_config(
        4000,
        "rex-gateway-missing-binary-xyz",
    ));
    let root = rex_root_path(&_guard);
    std::env::set_current_dir(&root).expect("chdir rex root");
    let mut loaded = rex_config::load_merged().expect("load");
    loaded.apply_effective_openai_compat_base_url();
    settings::init_for_test(Arc::new(loaded.clone()));

    let supervisor = GatewaySupervisor::new(GatewaySupervisorConfig::from_loaded(&loaded));
    assert!(
        supervisor.config().enabled,
        "expected managed gateway, got mode={}",
        loaded.effective.inference.gateway.mode
    );
    let err = supervisor
        .restart()
        .await
        .expect_err("missing gateway command");
    assert!(matches!(err, GatewaySupervisorError::CommandMissing { .. }));
    settings::reset_for_test();
    let _ = root;
}
