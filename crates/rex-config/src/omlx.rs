use crate::model::{InferenceConfig, OmlxConfig};

pub const OMLX_MODE_DISABLED: &str = "disabled";
pub const OMLX_MODE_EXTERNAL: &str = "external";
pub const OMLX_MODE_MANAGED: &str = "managed";

pub const DEFAULT_OMLX_PORT: u16 = 8000;
pub const DEFAULT_OMLX_COMMAND: &str = "omlx";
pub const DEFAULT_OMLX_STARTUP_TIMEOUT_SECS: u64 = 30;
pub const DEFAULT_OMLX_HEALTH_PATH: &str = "/v1/models";

/// Normalize `inference.omlx.mode` to a known token.
pub fn normalize_omlx_mode(mode: &str) -> &str {
    match mode.trim().to_ascii_lowercase().as_str() {
        OMLX_MODE_MANAGED => OMLX_MODE_MANAGED,
        OMLX_MODE_EXTERNAL => OMLX_MODE_EXTERNAL,
        _ => OMLX_MODE_DISABLED,
    }
}

pub fn is_managed_omlx(cfg: &OmlxConfig) -> bool {
    normalize_omlx_mode(&cfg.mode) == OMLX_MODE_MANAGED
}

pub fn omlx_required(cfg: &OmlxConfig) -> bool {
    if let Some(required) = cfg.required {
        return required;
    }
    is_managed_omlx(cfg)
}

pub fn omlx_allow_url_override(cfg: &OmlxConfig) -> bool {
    cfg.allow_url_override.unwrap_or(false)
}

pub fn managed_omlx_base_url(port: u16) -> String {
    format!("http://127.0.0.1:{port}/v1")
}

pub fn effective_omlx_port(cfg: &OmlxConfig) -> u16 {
    if cfg.port == 0 {
        DEFAULT_OMLX_PORT
    } else {
        cfg.port
    }
}

pub fn effective_omlx_health_path(cfg: &OmlxConfig) -> &str {
    let raw = cfg.health_path.trim();
    if raw.is_empty() {
        DEFAULT_OMLX_HEALTH_PATH
    } else {
        raw
    }
}

/// Validate mutual exclusion and oMLX field constraints.
pub fn validate_omlx(inference: &InferenceConfig) -> Result<(), String> {
    let omlx = &inference.omlx;
    let raw = omlx.mode.trim().to_ascii_lowercase();
    if !raw.is_empty()
        && raw != OMLX_MODE_DISABLED
        && raw != OMLX_MODE_EXTERNAL
        && raw != OMLX_MODE_MANAGED
    {
        return Err(format!(
            "unknown inference.omlx.mode: {} (expected disabled, external, or managed)",
            omlx.mode
        ));
    }
    if is_managed_omlx(omlx) && crate::gateway::is_managed_gateway(&inference.gateway) {
        return Err(
            "inference.omlx.mode and inference.gateway.mode cannot both be managed; enable at most one managed URL injector"
                .to_string(),
        );
    }
    if is_managed_omlx(omlx) {
        let port = effective_omlx_port(omlx);
        if port == 0 {
            return Err("inference.omlx.port must be between 1 and 65535".to_string());
        }
    }
    Ok(())
}

pub fn validate_omlx_config(cfg: &OmlxConfig) -> Result<(), String> {
    let raw = cfg.mode.trim().to_ascii_lowercase();
    if !raw.is_empty()
        && raw != OMLX_MODE_DISABLED
        && raw != OMLX_MODE_EXTERNAL
        && raw != OMLX_MODE_MANAGED
    {
        return Err(format!(
            "unknown inference.omlx.mode: {} (expected disabled, external, or managed)",
            cfg.mode
        ));
    }
    if is_managed_omlx(cfg) {
        let port = effective_omlx_port(cfg);
        if port == 0 {
            return Err("inference.omlx.port must be between 1 and 65535".to_string());
        }
    }
    Ok(())
}

/// Effective model id for broker requests (openai_compat.model with managed oMLX fallback).
pub fn resolve_effective_openai_compat_model(inference: &InferenceConfig) -> String {
    let configured = inference.openai_compat.model.trim();
    if !configured.is_empty() {
        return configured.to_string();
    }
    if is_managed_omlx(&inference.omlx) {
        let omlx_model = inference.omlx.model.trim();
        if !omlx_model.is_empty() {
            return omlx_model.to_string();
        }
    }
    configured.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gateway::resolve_effective_openai_compat_base_url;
    use crate::model::{InferenceConfig, OpenAiCompatConfig, OmlxConfig};

    fn inference_with_omlx(
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
            omlx: OmlxConfig {
                mode: mode.to_string(),
                port,
                allow_url_override: Some(allow_override),
                ..OmlxConfig::default()
            },
            ..InferenceConfig::default()
        }
    }

    #[test]
    fn managed_injects_loopback_url() {
        let inf = inference_with_omlx("managed", "", 8000, false);
        let url = resolve_effective_openai_compat_base_url(&inf, std::path::Path::new("/tmp/rex"));
        assert_eq!(url, "http://127.0.0.1:8000/v1");
    }

    #[test]
    fn managed_respects_override_when_allowed() {
        let inf = inference_with_omlx("managed", "http://custom/v1", 8000, true);
        let url = resolve_effective_openai_compat_base_url(&inf, std::path::Path::new("/tmp/rex"));
        assert_eq!(url, "http://custom/v1");
    }

    #[test]
    fn omlx_required_defaults_true_when_managed() {
        let cfg = OmlxConfig {
            mode: "managed".to_string(),
            required: None,
            ..OmlxConfig::default()
        };
        assert!(omlx_required(&cfg));
    }

    #[test]
    fn mutual_exclusion_rejects_both_managed() {
        let mut inf = inference_with_omlx("managed", "", 8000, false);
        inf.gateway.mode = "managed".to_string();
        let err = validate_omlx(&inf).expect_err("both managed");
        assert!(err.contains("cannot both be managed"));
    }

    #[test]
    fn managed_omlx_model_fallback_when_openai_model_empty() {
        let mut inf = inference_with_omlx("managed", "", 8000, false);
        inf.openai_compat.model = String::new();
        inf.omlx.model = "qwen2.5-coder-32b".to_string();
        assert_eq!(
            resolve_effective_openai_compat_model(&inf),
            "qwen2.5-coder-32b"
        );
    }
}
