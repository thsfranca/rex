use std::path::{Path, PathBuf};

use crate::model::{GatewayConfig, InferenceConfig};

pub const GATEWAY_MODE_DISABLED: &str = "disabled";
pub const GATEWAY_MODE_EXTERNAL: &str = "external";
pub const GATEWAY_MODE_MANAGED: &str = "managed";

pub const DEFAULT_GATEWAY_PORT: u16 = 4000;
pub const DEFAULT_GATEWAY_COMMAND: &str = "litellm";
pub const DEFAULT_GATEWAY_STARTUP_TIMEOUT_SECS: u64 = 30;

/// Normalize `inference.gateway.mode` to a known token.
pub fn normalize_gateway_mode(mode: &str) -> &str {
    match mode.trim().to_ascii_lowercase().as_str() {
        GATEWAY_MODE_MANAGED => GATEWAY_MODE_MANAGED,
        GATEWAY_MODE_EXTERNAL => GATEWAY_MODE_EXTERNAL,
        _ => GATEWAY_MODE_DISABLED,
    }
}

pub fn is_managed_gateway(cfg: &GatewayConfig) -> bool {
    normalize_gateway_mode(&cfg.mode) == GATEWAY_MODE_MANAGED
}

pub fn gateway_required(cfg: &GatewayConfig) -> bool {
    if let Some(required) = cfg.required {
        return required;
    }
    is_managed_gateway(cfg)
}

pub fn gateway_allow_url_override(cfg: &GatewayConfig) -> bool {
    cfg.allow_url_override.unwrap_or(false)
}

pub fn managed_gateway_base_url(port: u16) -> String {
    format!("http://127.0.0.1:{port}/v1")
}

pub fn default_gateway_config_path(rex_root: &Path) -> PathBuf {
    rex_root.join("gateway").join("config.yaml")
}

pub fn resolve_gateway_config_path(cfg: &GatewayConfig, rex_root: &Path) -> PathBuf {
    let raw = cfg.config_path.trim();
    if raw.is_empty() {
        default_gateway_config_path(rex_root)
    } else if Path::new(raw).is_absolute() {
        PathBuf::from(raw)
    } else {
        rex_root.join(raw)
    }
}

/// Effective OpenAI-compat base URL after gateway mode rules.
pub fn resolve_effective_openai_compat_base_url(
    inference: &InferenceConfig,
    rex_root: &Path,
) -> String {
    let _ = rex_root;
    let configured = inference.openai_compat.base_url.trim();
    let gateway = &inference.gateway;
    if normalize_gateway_mode(&gateway.mode) == GATEWAY_MODE_MANAGED {
        let port = effective_gateway_port(gateway);
        if gateway_allow_url_override(gateway) && !configured.is_empty() {
            return configured.to_string();
        }
        return managed_gateway_base_url(port);
    }
    configured.to_string()
}

pub fn effective_gateway_port(cfg: &GatewayConfig) -> u16 {
    if cfg.port == 0 {
        DEFAULT_GATEWAY_PORT
    } else {
        cfg.port
    }
}

pub fn validate_gateway(cfg: &GatewayConfig) -> Result<(), String> {
    let raw = cfg.mode.trim().to_ascii_lowercase();
    if !raw.is_empty()
        && raw != GATEWAY_MODE_DISABLED
        && raw != GATEWAY_MODE_EXTERNAL
        && raw != GATEWAY_MODE_MANAGED
    {
        return Err(format!(
            "unknown inference.gateway.mode: {} (expected disabled, external, or managed)",
            cfg.mode
        ));
    }
    let mode = normalize_gateway_mode(&cfg.mode);
    if mode == GATEWAY_MODE_MANAGED {
        let port = effective_gateway_port(cfg);
        if port == 0 {
            return Err("inference.gateway.port must be between 1 and 65535".to_string());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{GatewayConfig, InferenceConfig, OpenAiCompatConfig};

    fn inference_with_gateway(
        mode: &str,
        base_url: &str,
        port: u16,
        allow_override: bool,
    ) -> InferenceConfig {
        InferenceConfig {
            runtime: "http-openai-compat".to_string(),
            openai_compat: OpenAiCompatConfig {
                base_url: base_url.to_string(),
                ..OpenAiCompatConfig::default()
            },
            gateway: GatewayConfig {
                mode: mode.to_string(),
                port,
                allow_url_override: Some(allow_override),
                ..GatewayConfig::default()
            },
            ..InferenceConfig::default()
        }
    }

    #[test]
    fn managed_injects_loopback_url() {
        let inf = inference_with_gateway("managed", "", 4000, false);
        let url = resolve_effective_openai_compat_base_url(&inf, Path::new("/tmp/rex"));
        assert_eq!(url, "http://127.0.0.1:4000/v1");
    }

    #[test]
    fn managed_respects_override_when_allowed() {
        let inf = inference_with_gateway("managed", "http://custom/v1", 4000, true);
        let url = resolve_effective_openai_compat_base_url(&inf, Path::new("/tmp/rex"));
        assert_eq!(url, "http://custom/v1");
    }

    #[test]
    fn disabled_uses_configured_url() {
        let inf = inference_with_gateway("disabled", "http://127.0.0.1:11434/v1", 4000, false);
        let url = resolve_effective_openai_compat_base_url(&inf, Path::new("/tmp/rex"));
        assert_eq!(url, "http://127.0.0.1:11434/v1");
    }

    #[test]
    fn gateway_required_defaults_true_when_managed() {
        let cfg = GatewayConfig {
            mode: "managed".to_string(),
            required: None,
            ..GatewayConfig::default()
        };
        assert!(gateway_required(&cfg));
    }
}
