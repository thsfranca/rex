//! Operator-facing message templates per [CLI_OPERATOR_UX.md].

/// Daemon ensure lifecycle phases for header / loading copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecyclePhase {
    ProbeSuccess,
    ProbeFailNoAutostart,
    StartingSpawn,
    PollWaiting,
    Ready,
    Timeout,
    SpawnError,
}

/// Curated operator strings mapped from lifecycle and stream events.
#[derive(Debug, Clone, Default)]
pub struct OperatorMessaging;

impl OperatorMessaging {
    pub fn lifecycle_message(phase: LifecyclePhase, context: &LifecycleContext<'_>) -> String {
        match phase {
            LifecyclePhase::ProbeSuccess => "Ready — connected to Rex".to_string(),
            LifecyclePhase::ProbeFailNoAutostart => {
                "Rex is not running. Enable `daemon.auto_start` and run `rex`".to_string()
            }
            LifecyclePhase::StartingSpawn => "Starting Rex…".to_string(),
            LifecyclePhase::PollWaiting => "Waiting for Rex to become ready…".to_string(),
            LifecyclePhase::Ready => "Rex is ready".to_string(),
            LifecyclePhase::Timeout => format!(
                "Rex did not become ready within {}s — see {}",
                context.timeout_secs.unwrap_or(10),
                context.log_path.unwrap_or("daemon log")
            ),
            LifecyclePhase::SpawnError => format!(
                "Could not start Rex: {}",
                context.reason.unwrap_or("unknown error")
            ),
        }
    }

    pub fn activity_message(phase: &str, summary: &str) -> String {
        match phase.trim() {
            "thinking" => "Thinking…".to_string(),
            "tool_running" => "Running tools…".to_string(),
            "broker_wait" => "Waiting on broker…".to_string(),
            "compacting" => "Compacting context…".to_string(),
            "preparing" => "Preparing response…".to_string(),
            _ => {
                let s = summary.trim();
                if s.is_empty() {
                    "Working…".to_string()
                } else {
                    s.to_string()
                }
            }
        }
    }

    pub fn tool_message(name: &str, phase: &str, detail: &str) -> String {
        let name = name.trim();
        let phase = phase.trim();
        let detail = detail.trim();
        match phase {
            "running" => {
                if detail.is_empty() {
                    format!("{name}: running")
                } else {
                    format!("{name}: {detail}")
                }
            }
            "completed" => format!("{name} done"),
            "failed" => {
                if detail.is_empty() {
                    format!("{name} failed")
                } else {
                    format!("{name} failed: {detail}")
                }
            }
            "approval_required" => format!("{name} needs approval"),
            _ => {
                if detail.is_empty() {
                    format!("{name}: {phase}")
                } else {
                    format!("{name} {phase}: {detail}")
                }
            }
        }
    }

    pub fn step_message(summary: &str) -> String {
        let s = summary.trim();
        if s.is_empty() {
            "Step update".to_string()
        } else {
            s.to_string()
        }
    }

    pub fn plan_message(title: &str) -> String {
        let t = title.trim();
        if t.is_empty() {
            "Plan update".to_string()
        } else {
            format!("Plan: {t}")
        }
    }

    pub fn error_hint(code: &str, message: &str) -> String {
        let hint = match code.trim() {
            "daemon_unavailable" => "Daemon is not reachable — enable auto-start and run `rex`",
            "sidecar_unavailable" => {
                "Sidecar is not running — check `rex sidecar doctor`"
            }
            "inference_config" => {
                "Inference is not configured — edit `inference.openai_compat` in config"
            }
            "stream_timeout" => "Stream timed out — retry is usually safe",
            "stream_interrupted" => "Stream interrupted — retry is usually safe",
            "approval_required" => "Approval required — confirm execution",
            "workspace_not_configured" => "Workspace root not configured",
            "workspace_mismatch" => "Workspace does not match daemon",
            _ => message.trim(),
        };
        if hint.is_empty() {
            message.trim().to_string()
        } else {
            hint.to_string()
        }
    }
}

/// Optional context for lifecycle template expansion.
#[derive(Debug, Clone, Default)]
pub struct LifecycleContext<'a> {
    pub timeout_secs: Option<u64>,
    pub log_path: Option<&'a str>,
    pub reason: Option<&'a str>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activity_thinking_is_stable() {
        assert_eq!(
            OperatorMessaging::activity_message("thinking", ""),
            "Thinking…"
        );
    }

    #[test]
    fn tool_approval_required_template() {
        assert_eq!(
            OperatorMessaging::tool_message("fs.write", "approval_required", "src/a.rs"),
            "fs.write needs approval"
        );
    }
}
