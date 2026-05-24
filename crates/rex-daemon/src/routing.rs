//! Minimal inference routing hook (config-selected runtime today).

use crate::adapters::RuntimeKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RouteDecision {
    pub runtime: RuntimeKind,
    pub label: &'static str,
}

/// Phase-1 router: config default only; logs as `route=<label>`.
pub fn decide_route(_mode: &str, _model: &str) -> RouteDecision {
    let runtime = RuntimeKind::from_config();
    RouteDecision {
        runtime,
        label: runtime.log_label(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    #[serial_test::serial]
    fn route_follows_config_inference_runtime() {
        crate::settings::reset_for_test();
        let mut cfg = rex_config::RexConfig::defaults();
        cfg.inference.runtime = "mock".to_string();
        crate::settings::init_for_test(Arc::new(rex_config::LoadedConfig {
            rex_root: std::path::PathBuf::from("/tmp/rex-route-test"),
            global_path: None,
            project_path: None,
            effective: cfg,
        }));
        let decision = decide_route("ask", "");
        assert_eq!(decision.runtime, RuntimeKind::Mock);
        assert_eq!(decision.label, "mock");
        crate::settings::reset_for_test();
    }
}
