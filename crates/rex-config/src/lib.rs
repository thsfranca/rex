mod error;
mod layout;
mod merge;
mod model;
mod paths;
mod sidecar_binary;
mod workspace;

pub use error::ConfigError;
pub use layout::{ensure_global_layout, EnsureResult};
pub use merge::LoadedConfig;
pub use model::{
    AgentConfig, BrokerConfig, CacheConfig, ContextConfig, CursorCliConfig, DaemonConfig,
    InferenceConfig, OpenAiCompatConfig, RexConfig, SidecarEntry, SidecarsConfig, WorkspaceConfig,
    DEFAULT_DAEMON_SOCKET, DEFAULT_SIDECAR_SOCKET,
};
pub use paths::{global_config_path, proto_gen_path, proto_src_path, rex_root, REX_ROOT_ENV};
pub use sidecar_binary::sidecar_binary_resolvable;
pub use workspace::WorkspaceRootError;

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

    Ok(LoadedConfig {
        rex_root: root,
        global_path: global_loaded,
        project_path,
        effective,
    })
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
}
