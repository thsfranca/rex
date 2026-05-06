//! Daemon-owned agent execution approval seam (ADR 0009 / R008).
//!
//! `ApprovalGate` is the single decision point for whether an `agent`-mode
//! request is allowed to proceed. The default impl `AlwaysAllow` keeps the
//! seam behavior-preserving until enforcement ships behind
//! `REX_AGENT_APPROVALS` (R008 PR B2). See
//! [`docs/architecture/decisions/0009-centralized-agent-approvals-and-checkpoints.md`].

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
/// startup so introducing the gate is a behavior-preserving refactor.
pub struct AlwaysAllow;

#[tonic::async_trait]
impl ApprovalGate for AlwaysAllow {
    async fn check(&self, _ctx: &ApprovalContext) -> ApprovalDecision {
        ApprovalDecision::Allow
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
