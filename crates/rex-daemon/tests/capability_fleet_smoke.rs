//! Capability sidecar fleet supervisor (mock capability Health; no network).

#![allow(dead_code)]

use rex_config::{CapabilitySidecarEntry, RexConfig};
use serial_test::serial;
use std::fs;
use std::path::PathBuf;

#[allow(dead_code)]
#[path = "../src/capability_client.rs"]
mod capability_client;
#[allow(dead_code)]
#[path = "../src/settings.rs"]
mod settings;
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

mod support;

use sidecar_config::{SidecarFleetConfig, SidecarProcessConfig};
use supervisor::{SidecarFleet, SupervisorError};
use support::config::{install_rex_config, loaded_from_config, rex_root_path};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("repo root")
        .to_path_buf()
}

fn capability_mock_binary() -> PathBuf {
    for key in [
        "CARGO_BIN_EXE_rex_capability_mock",
        "CARGO_BIN_EXE_rex-capability-mock",
    ] {
        if let Ok(path) = std::env::var(key) {
            let path = PathBuf::from(path);
            if path.exists() {
                return path;
            }
        }
    }
    if let Some(path) = option_env!("CARGO_BIN_EXE_rex_capability_mock") {
        return PathBuf::from(path);
    }
    let target_dir = std::env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| repo_root().join("target"));
    for profile in ["debug", "release"] {
        let candidate = target_dir.join(profile).join("rex-capability-mock");
        if candidate.exists() {
            return candidate;
        }
    }
    panic!("rex-capability-mock binary not found; run: cargo build -p rex-capability-mock");
}

fn capability_only_config(cap_socket: &str, binary: &str) -> RexConfig {
    let mut cfg = RexConfig::defaults();
    cfg.sidecars.harness = None;
    cfg.sidecars.required = Some(false);
    if let Some(host) = cfg.sidecars.list.first_mut() {
        host.enabled = false;
    }
    cfg.sidecars.capabilities = vec![CapabilitySidecarEntry {
        name: "mock".to_string(),
        binary: binary.to_string(),
        enabled: true,
        socket: cap_socket.to_string(),
        provides: vec!["web.search".to_string()],
        required: Some(true),
    }];
    cfg
}

fn test_socket_path(label: &str) -> String {
    let dir = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_string());
    format!("{dir}/rex-cap-fleet-{label}-{}.sock", std::process::id())
}

#[tokio::test]
#[serial]
async fn capability_fleet_spawns_mock_and_passes_health() {
    let cap_socket = test_socket_path("mock");
    if std::path::Path::new(&cap_socket).exists() {
        let _ = fs::remove_file(&cap_socket);
    }
    let binary = capability_mock_binary();
    let binary_str = binary.display().to_string();
    let _guard = install_rex_config(capability_only_config(&cap_socket, &binary_str));
    let root = rex_root_path(&_guard);

    let loaded = loaded_from_config(capability_only_config(&cap_socket, &binary_str), &root);
    settings::init_for_test(loaded.clone());

    let fleet = SidecarFleet::new(SidecarFleetConfig {
        host: SidecarProcessConfig {
            name: "stub".to_string(),
            enabled: false,
            required: false,
            binary: PathBuf::from("rex-sidecar-stub"),
            socket_path: "/tmp/rex-sidecar.sock".to_string(),
            is_capability: false,
        },
        capabilities: vec![SidecarProcessConfig {
            name: "mock".to_string(),
            enabled: true,
            required: true,
            binary,
            socket_path: cap_socket.clone(),
            is_capability: true,
        }],
    });

    fleet
        .ensure_running()
        .await
        .expect("capability mock should become healthy");
    assert!(
        fleet.capabilities()[0].is_healthy().await,
        "capability health probe failed"
    );

    fleet.stop().await;
    settings::reset_for_test();
}

#[tokio::test]
#[serial]
async fn capability_fleet_config_from_loaded_parses_capabilities() {
    let cap_socket = test_socket_path("cfg");
    let binary = capability_mock_binary();
    let cfg = capability_only_config(&cap_socket, &binary.display().to_string());
    let _guard = install_rex_config(cfg.clone());
    let root = rex_root_path(&_guard);
    let loaded = loaded_from_config(cfg, &root);
    settings::init_for_test(loaded.clone());

    let fleet_cfg = SidecarFleetConfig::from_config(&loaded);
    assert!(!fleet_cfg.host.enabled);
    assert_eq!(fleet_cfg.capabilities.len(), 1);
    assert!(fleet_cfg.capabilities[0].is_capability);
    assert_eq!(fleet_cfg.capabilities[0].name, "mock");

    settings::reset_for_test();
}

#[tokio::test]
#[serial]
async fn capability_fleet_missing_binary_returns_error() {
    let fleet = SidecarFleet::new(SidecarFleetConfig {
        host: SidecarProcessConfig {
            name: "stub".to_string(),
            enabled: false,
            required: false,
            binary: PathBuf::from("rex-sidecar-stub"),
            socket_path: "/tmp/rex-sidecar.sock".to_string(),
            is_capability: false,
        },
        capabilities: vec![SidecarProcessConfig {
            name: "missing".to_string(),
            enabled: true,
            required: true,
            binary: PathBuf::from("/nonexistent/rex-capability-mock-test"),
            socket_path: test_socket_path("missing"),
            is_capability: true,
        }],
    });

    let err = fleet
        .ensure_running()
        .await
        .expect_err("missing binary should fail");
    assert!(matches!(err, SupervisorError::BinaryMissing { .. }));
}
