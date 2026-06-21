//! Managed oMLX supervisor (mock HTTP health; no oMLX binary in CI).

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;

use serial_test::serial;

#[allow(dead_code)]
#[path = "../src/omlx_supervisor.rs"]
mod omlx_supervisor;
#[allow(dead_code)]
#[path = "../src/settings.rs"]
mod settings;

mod support;

use omlx_supervisor::{OmlxSupervisor, OmlxSupervisorConfig, OmlxSupervisorError};
use support::config::{install_rex_config, managed_omlx_config, rex_root_path};
use support::openai_compat_sse::spawn_loopback_gateway_models_fixture;

fn write_sleep_stub(root: &std::path::Path) -> std::path::PathBuf {
    let stub = root.join("omlx-stub.sh");
    fs::write(&stub, "#!/bin/sh\nexec sleep 300\n").expect("write stub");
    let mut perms = fs::metadata(&stub).expect("meta").permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&stub, perms).expect("chmod stub");
    stub
}

#[tokio::test]
#[serial]
async fn managed_omlx_passes_health_with_mock_server() {
    let addr = spawn_loopback_gateway_models_fixture().await;
    let port = addr.port();
    let _guard = install_rex_config(managed_omlx_config(port, "/bin/sleep"));
    let root = rex_root_path(&_guard);
    std::env::set_current_dir(&root).expect("chdir rex root");
    let stub = write_sleep_stub(&root);
    let stub_cmd = stub.display().to_string();
    let mut cfg = managed_omlx_config(port, &stub_cmd);
    cfg.inference.omlx.command = stub.display().to_string();
    fs::write(
        root.join("config.json"),
        serde_json::to_string_pretty(&cfg).expect("json"),
    )
    .expect("rewrite config");
    let mut loaded = rex_config::load_merged().expect("load");
    loaded.apply_effective_openai_compat_base_url();
    settings::init_for_test(Arc::new(loaded.clone()));

    let supervisor = OmlxSupervisor::new(OmlxSupervisorConfig::from_loaded(&loaded));
    supervisor
        .ensure_running()
        .await
        .expect("omlx should become healthy");
    assert!(supervisor.is_healthy().await);
    supervisor.stop().await;
    settings::reset_for_test();
}

#[tokio::test]
#[serial]
async fn managed_omlx_fails_when_command_missing() {
    let _guard = install_rex_config(managed_omlx_config(
        8000,
        "rex-omlx-missing-binary-xyz",
    ));
    let root = rex_root_path(&_guard);
    std::env::set_current_dir(&root).expect("chdir rex root");
    let mut loaded = rex_config::load_merged().expect("load");
    loaded.apply_effective_openai_compat_base_url();
    settings::init_for_test(Arc::new(loaded.clone()));

    let supervisor = OmlxSupervisor::new(OmlxSupervisorConfig::from_loaded(&loaded));
    assert!(
        supervisor.config().enabled,
        "expected managed omlx, got mode={}",
        loaded.effective.inference.omlx.mode
    );
    let err = supervisor
        .restart()
        .await
        .expect_err("missing omlx command");
    assert!(matches!(err, OmlxSupervisorError::CommandMissing { .. }));
    settings::reset_for_test();
    let _ = root;
}
