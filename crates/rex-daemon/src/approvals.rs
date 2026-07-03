//! Daemon-owned agent execution approval seam (ADR 0009 / R008).
//!
//! `ApprovalGate` is the single decision point for whether an `agent`-mode
//! request is allowed to proceed. Default `AlwaysAllow`; opt-in enforcement
//! via `agent.approvals_enabled` in config and client `approval_id` (extension/CLI). See
//! [`docs/architecture/decisions/0009-centralized-agent-approvals-and-checkpoints.md`].

use std::sync::Arc;

use crate::adapters::RuntimeKind;

/// Per-request inputs the gate may inspect when authorizing `agent` mode.
/// Owned (rather than borrowed) so the gate can move it across `await` points
/// without complicating lifetimes for the trait object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalContext {
    /// Raw mode string from the request (already known to be `agent`-class
    /// after `normalize_mode`).
    pub mode: String,
    /// Inference runtime selected for this request.
    pub runtime: RuntimeKind,
    /// Client-supplied approval id when the user approved in the extension UI.
    pub approval_id: Option<String>,
}

/// Outcome of an approval check. `Checkpoint` is reserved for future tool-step
/// gating per ADR 0009; pattern-matched in `service.rs` and covered by tests.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApprovalDecision {
    Allow,
    Deny { reason: String },
    Checkpoint { reason: String },
}

/// Daemon trait for authorizing `agent`-mode requests. Async so a future
/// interactive gate can await client-supplied approval context without
/// changing the trait shape.
#[tonic::async_trait]
pub trait ApprovalGate: Send + Sync {
    async fn check(&self, ctx: &ApprovalContext) -> ApprovalDecision;
}

/// No-op gate: every `agent`-mode request is allowed. Default for daemon
/// startup unless `agent.approvals_enabled` opts into enforcement.
pub struct AlwaysAllow;

#[tonic::async_trait]
impl ApprovalGate for AlwaysAllow {
    async fn check(&self, _ctx: &ApprovalContext) -> ApprovalDecision {
        ApprovalDecision::Allow
    }
}

/// Enforcement gate selected when `agent.approvals_enabled` is true. Denies
/// `agent` requests without a non-empty `approval_id` in `ApprovalContext`;
/// allows when the client supplied approval context (CLI).
pub struct EnforceWithoutContext;

/// Stable deny reason returned by `EnforceWithoutContext`. Kept as a constant
/// so tests, dashboards, and clients have one string to match on.
pub const ENFORCEMENT_DENY_REASON: &str =
    "agent.approvals_enabled is true and no approval context supplied for agent mode";

#[tonic::async_trait]
impl ApprovalGate for EnforceWithoutContext {
    async fn check(&self, ctx: &ApprovalContext) -> ApprovalDecision {
        if ctx
            .approval_id
            .as_ref()
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false)
        {
            return ApprovalDecision::Allow;
        }
        ApprovalDecision::Deny {
            reason: ENFORCEMENT_DENY_REASON.to_string(),
        }
    }
}

/// Selects the daemon's startup gate based on `agent.approvals_enabled`. Called
/// once at daemon boot in `runtime.rs`; tests construct the concrete gate
/// they need directly to keep config isolation simple.
pub fn approval_gate_from_config() -> Arc<dyn ApprovalGate> {
    if crate::settings::get().approvals_enabled() {
        Arc::new(EnforceWithoutContext)
    } else {
        Arc::new(AlwaysAllow)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(mode: &str) -> ApprovalContext {
        ApprovalContext {
            mode: mode.to_string(),
            runtime: RuntimeKind::Mock,
            approval_id: None,
        }
    }

    #[tokio::test]
    async fn always_allow_returns_allow_for_any_context() {
        let gate = AlwaysAllow;
        for mode in ["agent", "AGENT", "agent-thinking"] {
            assert_eq!(gate.check(&ctx(mode)).await, ApprovalDecision::Allow);
        }
    }

    /// Sanity: a custom gate can return Deny and Checkpoint, exercising the
    /// non-Allow variants so they cannot regress to dead code.
    struct ScriptedGate {
        decision: ApprovalDecision,
    }

    #[tonic::async_trait]
    impl ApprovalGate for ScriptedGate {
        async fn check(&self, _ctx: &ApprovalContext) -> ApprovalDecision {
            self.decision.clone()
        }
    }

    #[tokio::test]
    async fn enforce_without_context_denies_with_stable_reason() {
        let gate = EnforceWithoutContext;
        let decision = gate.check(&ctx("agent")).await;
        assert_eq!(
            decision,
            ApprovalDecision::Deny {
                reason: ENFORCEMENT_DENY_REASON.to_string(),
            }
        );
    }

    /// Mutates loaded config so the config-driven selector tests must run serially.
    #[test]
    #[serial_test::serial]
    fn approval_gate_from_config_defaults_to_always_allow() {
        crate::settings::reset_for_test();
        let cfg = rex_config::RexConfig::defaults();
        crate::settings::init_for_test(std::sync::Arc::new(rex_config::LoadedConfig::for_test(
            std::path::PathBuf::from("/tmp/rex-approvals-test"),
            cfg,
        )));

        let gate = approval_gate_from_config();
        let runtime = tokio::runtime::Runtime::new().expect("runtime should build");
        let decision = runtime.block_on(gate.check(&ApprovalContext {
            mode: "agent".to_string(),
            runtime: RuntimeKind::Mock,
            approval_id: None,
        }));
        assert_eq!(decision, ApprovalDecision::Allow);

        crate::settings::reset_for_test();
    }

    #[test]
    fn enforce_allows_when_approval_id_present() {
        let gate = EnforceWithoutContext;
        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        let decision = runtime.block_on(gate.check(&ApprovalContext {
            mode: "agent".to_string(),
            runtime: RuntimeKind::Mock,
            approval_id: Some("apr-test-1".to_string()),
        }));
        assert_eq!(decision, ApprovalDecision::Allow);
    }

    #[test]
    #[serial_test::serial]
    fn approval_gate_from_config_selects_enforcement_when_enabled() {
        crate::settings::reset_for_test();
        let mut cfg = rex_config::RexConfig::defaults();
        cfg.agent.approvals_enabled = Some(true);
        crate::settings::init_for_test(std::sync::Arc::new(rex_config::LoadedConfig::for_test(
            std::path::PathBuf::from("/tmp/rex-approvals-test"),
            cfg,
        )));
        let runtime = tokio::runtime::Runtime::new().expect("runtime should build");
        let gate = approval_gate_from_config();
        let decision = runtime.block_on(gate.check(&ApprovalContext {
            mode: "agent".to_string(),
            runtime: RuntimeKind::Mock,
            approval_id: None,
        }));
        assert_eq!(
            decision,
            ApprovalDecision::Deny {
                reason: ENFORCEMENT_DENY_REASON.to_string(),
            }
        );
        crate::settings::reset_for_test();
    }

    #[test]
    #[serial_test::serial]
    fn approval_gate_from_config_keeps_always_allow_when_disabled() {
        crate::settings::reset_for_test();
        let mut cfg = rex_config::RexConfig::defaults();
        cfg.agent.approvals_enabled = Some(false);
        crate::settings::init_for_test(std::sync::Arc::new(rex_config::LoadedConfig::for_test(
            std::path::PathBuf::from("/tmp/rex-approvals-test"),
            cfg,
        )));
        let runtime = tokio::runtime::Runtime::new().expect("runtime should build");
        let gate = approval_gate_from_config();
        let decision = runtime.block_on(gate.check(&ApprovalContext {
            mode: "agent".to_string(),
            runtime: RuntimeKind::Mock,
            approval_id: None,
        }));
        assert_eq!(decision, ApprovalDecision::Allow);
        crate::settings::reset_for_test();
    }

    #[tokio::test]
    async fn scripted_gate_emits_deny_and_checkpoint_variants() {
        let deny = ScriptedGate {
            decision: ApprovalDecision::Deny {
                reason: "no permission".to_string(),
            },
        };
        assert_eq!(
            deny.check(&ctx("agent")).await,
            ApprovalDecision::Deny {
                reason: "no permission".to_string()
            }
        );

        let checkpoint = ScriptedGate {
            decision: ApprovalDecision::Checkpoint {
                reason: "needs review".to_string(),
            },
        };
        assert_eq!(
            checkpoint.check(&ctx("agent")).await,
            ApprovalDecision::Checkpoint {
                reason: "needs review".to_string()
            }
        );
    }
}
