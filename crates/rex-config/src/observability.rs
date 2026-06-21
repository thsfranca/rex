use crate::error::ConfigError;
use crate::model::{ObservabilityConfig, RexConfig};

pub const DEFAULT_OBS_SERVICE_NAME: &str = "rex-daemon";
pub const DEFAULT_OTLP_PROTOCOL: &str = "grpc";

pub fn observability_enabled(obs: &ObservabilityConfig) -> bool {
    obs.enabled.unwrap_or(false)
}

pub fn validate_observability(obs: &ObservabilityConfig) -> Result<(), ConfigError> {
    if !observability_enabled(obs) {
        return Ok(());
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

    Ok(())
}

pub fn economics_snapshot_json(config: &RexConfig) -> serde_json::Value {
    let mut value = serde_json::json!({
        "inference": {
            "runtime": config.inference.runtime,
            "openai_compat": {
                "base_url": config.inference.openai_compat.base_url,
                "model": config.inference.openai_compat.model,
                "timeout_secs": config.inference.openai_compat.timeout_secs,
                "native_tools": config.inference.openai_compat.effective_native_tools(),
                "header_names": crate::openai_compat::header_names_sorted(
                    &config.inference.openai_compat.headers,
                ),
            },
            "gateway": {
                "mode": config.inference.gateway.mode,
                "port": config.inference.gateway.port,
            },
            "omlx": {
                "mode": config.inference.omlx.mode,
                "port": config.inference.omlx.port,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::RexConfig;

    #[test]
    fn otlp_protocol_validation_accepts_grpc_and_http() {
        let mut cfg = RexConfig::defaults();
        cfg.observability.enabled = Some(true);
        cfg.observability.otlp.protocol = "http/protobuf".to_string();
        validate_observability(&cfg.observability).expect("http/protobuf accepted");
    }

    #[test]
    fn unknown_otlp_protocol_rejected() {
        let mut cfg = RexConfig::defaults();
        cfg.observability.enabled = Some(true);
        cfg.observability.otlp.protocol = "udp".to_string();
        let err = validate_observability(&cfg.observability).expect_err("bind");
        assert!(err.to_string().contains("protocol"));
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

    #[test]
    fn economics_snapshot_excludes_header_values() {
        let mut cfg = RexConfig::defaults();
        cfg.inference
            .openai_compat
            .headers
            .insert("X-Api-Key".to_string(), "top-secret".to_string());
        let json = economics_snapshot_json(&cfg);
        let raw = json.to_string();
        assert!(!raw.contains("top-secret"));
        assert!(raw.contains("X-Api-Key"));
        assert!(raw.contains("header_names"));
    }
}
