//! Daemon-owned agent execution approval seam (ADR 0009 / R008).
//!
//! `ApprovalGate` is the single decision point for whether an `agent`-mode
//! request is allowed to proceed. The default impl `AlwaysAllow` keeps the
//! seam behavior-preserving until enforcement ships behind
//! `REX_AGENT_APPROVALS` (R008 PR B2). See
//! [`docs/architecture/decisions/0009-centralized-agent-approvals-and-checkpoints.md`].

use std::env;
use std::sync::Arc;

use crate::adapters::RuntimeKind;

/// Environment variable that opts the daemon into agent-mode approval
/// enforcement. Default off; when set to `1` or `true` (case-insensitive),
/// `agent` requests are denied until a client supplies approval context. See
/// `docs/CONFIGURATION.md` and ADR 0009.
pub const APPROVALS_ENV: &str = "REX_AGENT_APPROVALS";

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
}

/// Outcome of an approval check. `Checkpoint` is **reserved** for future
/// tool-step gating per ADR 0009; today no shipped gate emits it. The bin
/// build only constructs `Allow` via `AlwaysAllow`, so the other variants
/// look dead until R008 PR B2 ships an enforcement gate; the variants are
/// already pattern-matched by `service.rs` and exercised by tests.
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
/// startup unless `REX_AGENT_APPROVALS` opts into enforcement.
pub struct AlwaysAllow;

#[tonic::async_trait]
impl ApprovalGate for AlwaysAllow {
    async fn check(&self, _ctx: &ApprovalContext) -> ApprovalDecision {
        ApprovalDecision::Allow
    }
}

/// Enforcement gate selected by `REX_AGENT_APPROVALS=1`. Today no client
/// supplies approval context yet, so the gate denies every `agent` request
/// with a stable reason matching ADR 0009. When the extension wires up
/// approval context, this gate (or its successor) reads it from the
/// `ApprovalContext` and returns `Allow` for permitted runs.
pub struct EnforceWithoutContext;

/// Stable deny reason returned by `EnforceWithoutContext`. Kept as a constant
/// so tests, dashboards, and clients have one string to match on.
pub const ENFORCEMENT_DENY_REASON: &str =
    "REX_AGENT_APPROVALS=1 and no approval context supplied for agent mode";

#[tonic::async_trait]
impl ApprovalGate for EnforceWithoutContext {
    async fn check(&self, _ctx: &ApprovalContext) -> ApprovalDecision {
        ApprovalDecision::Deny {
            reason: ENFORCEMENT_DENY_REASON.to_string(),
        }
    }
}

/// Selects the daemon's startup gate based on `REX_AGENT_APPROVALS`. Called
/// once at daemon boot in `runtime.rs`; tests construct the concrete gate
/// they need directly to keep environment isolation simple.
pub fn approval_gate_from_env() -> Arc<dyn ApprovalGate> {
    let raw = env::var(APPROVALS_ENV).unwrap_or_default();
    let trimmed = raw.trim();
    if trimmed == "1" || trimmed.eq_ignore_ascii_case("true") {
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

    /// Mutates `REX_AGENT_APPROVALS` so the env-driven selector tests must run
    /// serially. `serial_test::serial` is already a daemon dev-dependency.
    #[test]
    #[serial_test::serial]
    fn approval_gate_from_env_defaults_to_always_allow() {
        let prev = env::var(APPROVALS_ENV).ok();
        env::remove_var(APPROVALS_ENV);

        let gate = approval_gate_from_env();
        let runtime = tokio::runtime::Runtime::new().expect("runtime should build");
        let decision = runtime.block_on(gate.check(&ApprovalContext {
            mode: "agent".to_string(),
            runtime: RuntimeKind::Mock,
        }));
        assert_eq!(decision, ApprovalDecision::Allow);

        if let Some(value) = prev {
            env::set_var(APPROVALS_ENV, value);
        }
    }

    #[test]
    #[serial_test::serial]
    fn approval_gate_from_env_selects_enforcement_for_truthy_values() {
        let prev = env::var(APPROVALS_ENV).ok();
        let runtime = tokio::runtime::Runtime::new().expect("runtime should build");
        for value in ["1", "true", "TRUE", "  true  "] {
            env::set_var(APPROVALS_ENV, value);
            let gate = approval_gate_from_env();
            let decision = runtime.block_on(gate.check(&ApprovalContext {
                mode: "agent".to_string(),
                runtime: RuntimeKind::Mock,
            }));
            assert_eq!(
                decision,
                ApprovalDecision::Deny {
                    reason: ENFORCEMENT_DENY_REASON.to_string(),
                },
                "value {value:?} should select enforcement",
            );
        }

        if let Some(value) = prev {
            env::set_var(APPROVALS_ENV, value);
        } else {
            env::remove_var(APPROVALS_ENV);
        }
    }

    #[test]
    #[serial_test::serial]
    fn approval_gate_from_env_ignores_unknown_values() {
        let prev = env::var(APPROVALS_ENV).ok();
        let runtime = tokio::runtime::Runtime::new().expect("runtime should build");
        for value in ["0", "false", "yes", "no", ""] {
            env::set_var(APPROVALS_ENV, value);
            let gate = approval_gate_from_env();
            let decision = runtime.block_on(gate.check(&ApprovalContext {
                mode: "agent".to_string(),
                runtime: RuntimeKind::Mock,
            }));
            assert_eq!(
                decision,
                ApprovalDecision::Allow,
                "value {value:?} should keep AlwaysAllow",
            );
        }

        if let Some(value) = prev {
            env::set_var(APPROVALS_ENV, value);
        } else {
            env::remove_var(APPROVALS_ENV);
        }
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
