mod error;
mod gateway;
mod gateway_layout;
mod layout;
mod merge;
mod model;
mod observability;
mod omlx;
mod omlx_layout;
mod openai_compat;
mod paths;
mod project;
mod sidecar_binary;
mod sockets;
mod workspace;

pub use error::ConfigError;
pub use gateway::{
    default_gateway_config_path, effective_gateway_port, gateway_allow_url_override,
    gateway_required, is_managed_gateway, managed_gateway_base_url, normalize_gateway_mode,
    resolve_effective_openai_compat_base_url, resolve_gateway_config_path, validate_gateway,
    DEFAULT_GATEWAY_COMMAND, DEFAULT_GATEWAY_PORT, DEFAULT_GATEWAY_STARTUP_TIMEOUT_SECS,
    GATEWAY_MODE_DISABLED, GATEWAY_MODE_EXTERNAL, GATEWAY_MODE_MANAGED,
};
pub use omlx::{
    effective_omlx_health_path, effective_omlx_port, is_managed_omlx, managed_omlx_base_url,
    normalize_omlx_mode, omlx_allow_url_override, omlx_required, resolve_effective_openai_compat_model,
    validate_omlx, validate_omlx_config, DEFAULT_OMLX_COMMAND, DEFAULT_OMLX_HEALTH_PATH,
    DEFAULT_OMLX_PORT, DEFAULT_OMLX_STARTUP_TIMEOUT_SECS, OMLX_MODE_DISABLED, OMLX_MODE_EXTERNAL,
    OMLX_MODE_MANAGED,
};
pub use gateway_layout::{ensure_gateway_layout, GatewayLayoutResult};
pub use omlx_layout::{ensure_omlx_layout, OmlxLayoutResult};
pub use layout::{ensure_global_layout, EnsureResult};
pub use merge::LoadedConfig;
pub use model::{
    AgentConfig, BrokerConfig, CacheConfig, CapabilitySidecarEntry, ContextConfig, CursorCliConfig,
    DaemonConfig, DaemonSocketScope, GatewayConfig, GatewayOllamaConfig, InferenceConfig,
    NativeToolsMode, OmlxConfig, DEFAULT_DAEMON_READY_TIMEOUT_SECS, ObservabilityConfig,
    OpenAiCompatConfig, OtlpConfig, RexConfig, SidecarEntry, SidecarsConfig, WorkspaceConfig,
    DEFAULT_DAEMON_SOCKET, DEFAULT_SIDECAR_SOCKET,
};
pub use observability::{
    economics_snapshot_id, economics_snapshot_json, observability_enabled, validate_observability,
    DEFAULT_OBS_SERVICE_NAME, DEFAULT_OTLP_PROTOCOL,
};
pub use paths::{
    gateway_dir, gateway_env_path, global_config_path, omlx_dir, proto_gen_path,
    proto_src_path, rex_root, REX_ROOT_ENV,
};
pub use project::ensure_project_workspace_root;
pub use sidecar_binary::{
    rex_agent_doctor_applies, rex_agent_doctor_checks, sidecar_binary_resolvable,
    sidecar_install_hint,
};
pub use workspace::{resolve_workspace_root_for_effective, WorkspaceRootError};

use std::env;
use std::path::PathBuf;

/// Load merged configuration, bootstrapping `$REX_ROOT` layout when missing.
pub fn load() -> Result<LoadedConfig, ConfigError> {
    let _ = ensure_global_layout()?;
    load_merged()
}

/// Load merged configuration without creating directories (tests).
pub fn load_merged() -> Result<LoadedConfig, ConfigError> {
    maybe_warn_legacy_env();
    let root = rex_root();
    let global_path = global_config_path();
    let mut effective = RexConfig::defaults();

    let global_loaded = if global_path.is_file() {
        let raw = std::fs::read_to_string(&global_path)?;
        let overlay: RexConfig = serde_json::from_str(&raw)?;
        merge::merge_config(&mut effective, overlay);
        Some(global_path)
    } else {
        None
    };

    let project_path =
        find_project_config(env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    if let Some(ref path) = project_path {
        let raw = std::fs::read_to_string(path)?;
        let overlay: RexConfig = serde_json::from_str(&raw)?;
        merge::merge_config(&mut effective, overlay);
    }

    effective.validate()?;

    LoadedConfig::from_effective(root, global_loaded, project_path, effective)
}

fn find_project_config(start: PathBuf) -> Option<PathBuf> {
    let mut current = Some(start.as_path());
    while let Some(dir) = current {
        let candidate = dir.join(".rex").join("config.json");
        if candidate.is_file() {
            return Some(candidate);
        }
        current = dir.parent();
    }
    None
}

const LEGACY_ENV_KEYS: &[&str] = &[
    "REX_INFERENCE_RUNTIME",
    "REX_OPENAI_COMPAT_BASE_URL",
    "REX_SIDECAR_ENABLED",
    "REX_DAEMON_SOCKET",
    "REX_WORKSPACE_ROOT",
];

fn maybe_warn_legacy_env() {
    let mut found = Vec::new();
    for key in LEGACY_ENV_KEYS {
        if env::var(key).is_ok() {
            found.push(*key);
        }
    }
    if found.is_empty() {
        return;
    }
    eprintln!(
        "rex: legacy environment variables are ignored ({}) — use $REX_ROOT/config.json (run `rex config init`)",
        found.join(", ")
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::fs;
    use std::path::Path;

    fn with_rex_root<F: FnOnce()>(dir: &Path, f: F) {
        let prev = env::var(REX_ROOT_ENV).ok();
        env::set_var(REX_ROOT_ENV, dir);
        f();
        match prev {
            Some(v) => env::set_var(REX_ROOT_ENV, v),
            None => env::remove_var(REX_ROOT_ENV),
        }
    }

    #[test]
    #[serial]
    fn ensure_creates_layout_and_config_when_missing() {
        let tmp = tempfile::tempdir().unwrap();
        with_rex_root(tmp.path(), || {
            let result = ensure_global_layout().expect("ensure");
            assert!(result.created_config);
            assert!(global_config_path().is_file());
            assert!(proto_src_path().is_dir());
            assert!(proto_gen_path().is_dir());
            let raw = fs::read_to_string(global_config_path()).unwrap();
            assert!(raw.contains(r#""active": "agent""#));
            assert!(raw.contains(r#""binary": "rex-agent""#));
            assert!(raw.contains(r#""enabled": true"#));
            assert!(raw.contains(r#""provider": "mock""#));
        });
    }

    #[test]
    #[serial]
    fn ensure_does_not_overwrite_existing_config() {
        let tmp = tempfile::tempdir().unwrap();
        with_rex_root(tmp.path(), || {
            fs::create_dir_all(rex_root()).unwrap();
            let path = global_config_path();
            fs::write(&path, r#"{"version":1,"inference":{"runtime":"mock"}}"#).unwrap();
            let result = ensure_global_layout().expect("ensure");
            assert!(!result.created_config);
            let raw = fs::read_to_string(path).unwrap();
            assert!(!raw.contains("sidecars"));
            assert!(raw.contains("mock"));
        });
    }

    #[test]
    #[serial]
    fn project_config_overrides_global() {
        let tmp = tempfile::tempdir().unwrap();
        with_rex_root(tmp.path(), || {
            ensure_global_layout().unwrap();
            let mut global = RexConfig::defaults();
            global.inference.runtime = "mock".to_string();
            fs::write(
                global_config_path(),
                serde_json::to_string_pretty(&global).unwrap(),
            )
            .unwrap();

            let proj_dir = tmp.path().join("proj");
            fs::create_dir_all(proj_dir.join(".rex")).unwrap();
            fs::write(
                proj_dir.join(".rex/config.json"),
                r#"{"inference":{"runtime":"http-openai-compat","openai_compat":{"base_url":"http://127.0.0.1:9/v1"}}}"#,
            )
            .unwrap();

            env::set_current_dir(&proj_dir).unwrap();
            let loaded = load_merged().expect("load");
            assert_eq!(loaded.effective.inference.runtime, "http-openai-compat");
            assert_eq!(
                loaded.effective.inference.openai_compat.base_url,
                "http://127.0.0.1:9/v1"
            );
            env::set_current_dir(tmp.path()).unwrap();
        });
    }

    #[test]
    #[serial]
    fn managed_gateway_allows_empty_base_url_in_validate() {
        let tmp = tempfile::tempdir().unwrap();
        with_rex_root(tmp.path(), || {
            env::set_current_dir(tmp.path()).unwrap();
            ensure_global_layout().unwrap();
            fs::write(
                global_config_path(),
                r#"{
  "version": 1,
  "inference": {
    "runtime": "http-openai-compat",
    "gateway": { "mode": "managed", "port": 4000 },
    "openai_compat": { "model": "gpt-4o-mini" }
  }
}"#,
            )
            .unwrap();
            let loaded = load_merged().expect("load");
            assert_eq!(
                loaded.effective_openai_compat_base_url(),
                "http://127.0.0.1:4000/v1"
            );
        });
    }

    #[test]
    #[serial]
    fn observability_enabled_otlp_validate() {
        let tmp = tempfile::tempdir().unwrap();
        with_rex_root(tmp.path(), || {
            env::set_current_dir(tmp.path()).unwrap();
            ensure_global_layout().unwrap();
            fs::write(
                global_config_path(),
                r#"{
  "version": 1,
  "observability": {
    "enabled": true,
    "otlp": { "endpoint": "http://127.0.0.1:4317", "protocol": "grpc" }
  }
}"#,
            )
            .unwrap();
            load_merged().expect("valid observability config");
        });
    }

    #[test]
    #[serial]
    fn legacy_store_fields_ignored_in_json() {
        let tmp = tempfile::tempdir().unwrap();
        with_rex_root(tmp.path(), || {
            env::set_current_dir(tmp.path()).unwrap();
            ensure_global_layout().unwrap();
            fs::write(
                global_config_path(),
                r#"{
  "version": 1,
  "observability": { "enabled": true, "store": { "engine": "mmap" } }
}"#,
            )
            .unwrap();
            load_merged().expect("legacy store keys ignored");
        });
    }

    #[test]
    #[serial]
    fn disabled_http_runtime_requires_base_url() {
        let tmp = tempfile::tempdir().unwrap();
        with_rex_root(tmp.path(), || {
            env::set_current_dir(tmp.path()).unwrap();
            ensure_global_layout().unwrap();
            fs::write(
                global_config_path(),
                r#"{
  "version": 1,
  "inference": {
    "runtime": "http-openai-compat",
    "gateway": { "mode": "disabled" },
    "openai_compat": { "model": "gpt-4o-mini" }
  }
}"#,
            )
            .unwrap();
            let err = load_merged().expect_err("missing base_url");
            assert!(err.to_string().contains("base_url"));
        });
    }

    #[test]
    fn agent_config_merge_loop_optimization_fields() {
        use crate::merge::merge_config;

        let mut base = RexConfig::defaults();
        base.agent.compaction_enabled = Some(false);
        base.agent.deterministic_init_enabled = Some(true);

        let mut overlay = RexConfig::default();
        overlay.agent.compaction_enabled = Some(true);
        overlay.agent.soft_cap_enabled = Some(true);
        overlay.agent.soft_cap_fraction = Some(0.5);

        merge_config(&mut base, overlay);
        assert_eq!(base.agent.compaction_enabled, Some(true));
        assert_eq!(base.agent.soft_cap_enabled, Some(true));
        assert_eq!(base.agent.soft_cap_fraction, Some(0.5));
        assert_eq!(base.agent.deterministic_init_enabled, Some(true));
    }

    #[test]
    fn daemon_config_defaults_auto_start_on() {
        let cfg = RexConfig::defaults();
        assert!(cfg.daemon.auto_start_enabled());
        assert_eq!(
            cfg.daemon.ready_timeout_secs,
            crate::model::DEFAULT_DAEMON_READY_TIMEOUT_SECS
        );
    }

    #[test]
    #[serial]
    fn daemon_config_merge_and_log_path() {
        use crate::merge::merge_config;

        let tmp = tempfile::tempdir().unwrap();
        with_rex_root(tmp.path(), || {
            let mut base = RexConfig::defaults();
            let mut overlay = RexConfig::default();
            overlay.daemon.auto_start = Some(false);
            overlay.daemon.ready_timeout_secs = 30;
            overlay.daemon.log_path = "/tmp/custom-daemon.log".to_string();
            merge_config(&mut base, overlay);
            assert!(!base.daemon.auto_start_enabled());
            assert_eq!(base.daemon.ready_timeout_secs, 30);
            assert_eq!(base.daemon.log_path, "/tmp/custom-daemon.log");

            let loaded = load_merged().expect("load");
            assert_eq!(
                loaded.daemon_log_path(),
                tmp.path().join("daemon.log")
            );
        });
    }
}
