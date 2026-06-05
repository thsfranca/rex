use std::path::{Path, PathBuf};

use crate::error::ConfigError;
use crate::model::{ObservabilityConfig, RexConfig, StoreConfig};

pub const DEFAULT_OBS_SERVICE_NAME: &str = "rex-daemon";
pub const DEFAULT_STORE_ENGINE_SQLITE: &str = "sqlite";
pub const DEFAULT_STORE_ENGINE_MMAP: &str = "mmap";
pub const DEFAULT_STORE_PATH_SQLITE: &str = "obs/store.sqlite";
pub const DEFAULT_STORE_PATH_MMAP: &str = "obs/store.rexobs";
pub const DEFAULT_OTLP_PROTOCOL: &str = "grpc";

pub fn observability_enabled(obs: &ObservabilityConfig) -> bool {
    obs.enabled.unwrap_or(false)
}

pub fn validate_observability(obs: &ObservabilityConfig) -> Result<(), ConfigError> {
    if !observability_enabled(obs) {
        return Ok(());
    }

    let engine = normalized_store_engine(&obs.store.engine);
    if engine == DEFAULT_STORE_ENGINE_MMAP {
        return Err(ConfigError::Validation(
            "mmap store not implemented; use sqlite".to_string(),
        ));
    }
    if engine != DEFAULT_STORE_ENGINE_SQLITE {
        return Err(ConfigError::Validation(format!(
            "unknown observability.store.engine: {}",
            obs.store.engine
        )));
    }

    let protocol = obs.otlp.protocol.trim().to_ascii_lowercase();
    match protocol.as_str() {
        "" | DEFAULT_OTLP_PROTOCOL | "http/protobuf" | "http-protobuf" => {}
        other => {
            return Err(ConfigError::Validation(format!(
                "unknown observability.otlp.protocol: {other}"
            )));
        }
    }

    if obs.store.format_version != 0 && obs.store.format_version != 1 {
        return Err(ConfigError::Validation(format!(
            "unsupported observability.store.format_version: {}",
            obs.store.format_version
        )));
    }

    Ok(())
}

pub fn resolve_store_path(rex_root: &Path, store: &StoreConfig) -> PathBuf {
    let engine = normalized_store_engine(&store.engine);
    let default_path = if engine == DEFAULT_STORE_ENGINE_MMAP {
        DEFAULT_STORE_PATH_MMAP
    } else {
        DEFAULT_STORE_PATH_SQLITE
    };
    let raw = store.path.trim();
    let relative = if raw.is_empty() { default_path } else { raw };
    if Path::new(relative).is_absolute() {
        PathBuf::from(relative)
    } else {
        rex_root.join(relative)
    }
}

pub fn economics_snapshot_json(config: &RexConfig) -> serde_json::Value {
    let mut value = serde_json::json!({
        "inference": {
            "runtime": config.inference.runtime,
            "openai_compat": {
                "base_url": config.inference.openai_compat.base_url,
                "model": config.inference.openai_compat.model,
                "timeout_secs": config.inference.openai_compat.timeout_secs,
            },
            "gateway": {
                "mode": config.inference.gateway.mode,
                "port": config.inference.gateway.port,
            },
        },
        "context": config.context,
        "cache": config.cache,
        "broker": {
            "max_tool_result_bytes": config.broker.max_tool_result_bytes,
            "shell_allowlist_len": config.broker.shell_allowlist.len(),
        },
        "agent": config.agent,
        "observability": {
            "enabled": observability_enabled(&config.observability),
            "service_name": config.observability.service_name,
            "store": {
                "engine": config.observability.store.engine,
                "path": config.observability.store.path,
            },
        },
    });
    if let Some(obj) = value.as_object_mut() {
        obj.remove("api_key");
    }
    value
}

pub fn economics_snapshot_id(config: &RexConfig) -> String {
    let json = economics_snapshot_json(config);
    let canonical = serde_json::to_string(&json).unwrap_or_default();
    snapshot_hash(&canonical)
}

fn snapshot_hash(canonical_json: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    canonical_json.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn normalized_store_engine(engine: &str) -> String {
    let trimmed = engine.trim().to_ascii_lowercase();
    match trimmed.as_str() {
        "" | DEFAULT_STORE_ENGINE_SQLITE => DEFAULT_STORE_ENGINE_SQLITE.to_string(),
        DEFAULT_STORE_ENGINE_MMAP => DEFAULT_STORE_ENGINE_MMAP.to_string(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::RexConfig;

    #[test]
    fn mmap_rejected_when_enabled() {
        let mut cfg = RexConfig::defaults();
        cfg.observability.enabled = Some(true);
        cfg.observability.store.engine = "mmap".to_string();
        let err = validate_observability(&cfg.observability).expect_err("mmap");
        assert!(err.to_string().contains("mmap store not implemented"));
    }

    #[test]
    fn economics_snapshot_excludes_api_key() {
        let mut cfg = RexConfig::defaults();
        cfg.inference.openai_compat.api_key = Some("secret".to_string());
        let json = economics_snapshot_json(&cfg);
        let raw = json.to_string();
        assert!(!raw.contains("secret"));
        assert!(!raw.contains("api_key"));
    }
}
