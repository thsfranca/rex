//! Minimal inference routing hook (env-selected runtime today).

use crate::adapters::RuntimeKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RouteDecision {
    pub runtime: RuntimeKind,
    pub label: &'static str,
}

/// Phase-1 router: env default only; logs as `route=<label>`.
pub fn decide_route(_mode: &str, _model: &str) -> RouteDecision {
    let runtime = RuntimeKind::from_env();
    RouteDecision {
        runtime,
        label: runtime.log_label(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    struct EnvGuard {
        key: &'static str,
        previous: Option<String>,
    }

    impl EnvGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let previous = env::var(key).ok();
            env::set_var(key, value);
            Self { key, previous }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.previous {
                Some(v) => env::set_var(self.key, v),
                None => env::remove_var(self.key),
            }
        }
    }

    #[test]
    fn route_follows_rex_inference_runtime() {
        let _guard = EnvGuard::set("REX_INFERENCE_RUNTIME", "mock");
        let decision = decide_route("ask", "");
        assert_eq!(decision.runtime, RuntimeKind::Mock);
        assert_eq!(decision.label, "mock");
    }
}
