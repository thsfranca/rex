use std::collections::HashMap;

use crate::event::StreamEvent;
use crate::messaging::OperatorMessaging;

/// High-level turn phase for harness UI state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TurnPhase {
    #[default]
    Idle,
    Generating,
    ToolRunning,
    ToolApproval,
    Terminal,
}

/// Active or completed tool card keyed by `tool_call_id`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCard {
    pub tool_call_id: String,
    pub name: String,
    pub phase: String,
    pub detail: String,
    pub completed: bool,
}

/// Side effects produced when the state machine applies a stream event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiEffect {
    AppendChunk(String),
    OperatorMessage(String),
    ToolUpdated(ToolCard),
    PhaseChanged(TurnPhase),
    TerminalDone,
    TerminalError { code: String, message: String },
    Ignored,
}

/// Pure turn state machine for NDJSON stream projection.
#[derive(Debug, Clone, Default)]
pub struct TurnState {
    pub phase: TurnPhase,
    pub active_tools: HashMap<String, ToolCard>,
    pub output_text: String,
    pub last_error: Option<(String, String)>,
}

impl TurnState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset_to_idle(&mut self) {
        self.phase = TurnPhase::Idle;
        self.active_tools.clear();
        self.last_error = None;
    }

    pub fn apply(&mut self, event: &StreamEvent) -> Vec<UiEffect> {
        let mut effects = Vec::new();

        match event {
            StreamEvent::Chunk { text, .. } => {
                if !text.is_empty() {
                    if self.phase == TurnPhase::Idle {
                        self.phase = TurnPhase::Generating;
                        effects.push(UiEffect::PhaseChanged(TurnPhase::Generating));
                    } else if self.phase == TurnPhase::Terminal {
                        self.phase = TurnPhase::Generating;
                        effects.push(UiEffect::PhaseChanged(TurnPhase::Generating));
                    }
                    self.output_text.push_str(text);
                    effects.push(UiEffect::AppendChunk(text.clone()));
                }
            }
            StreamEvent::Activity { phase, summary, .. } => {
                if self.phase == TurnPhase::Idle {
                    self.phase = TurnPhase::Generating;
                    effects.push(UiEffect::PhaseChanged(TurnPhase::Generating));
                }
                effects.push(UiEffect::OperatorMessage(
                    OperatorMessaging::activity_message(phase, summary),
                ));
            }
            StreamEvent::Tool {
                name,
                phase,
                detail,
                tool_call_id,
                ..
            } => {
                let id = tool_call_id
                    .clone()
                    .unwrap_or_else(|| format!("{name}:{phase}:{detail}"));
                let completed = matches!(phase.as_str(), "completed" | "failed");
                let card = ToolCard {
                    tool_call_id: id.clone(),
                    name: name.clone(),
                    phase: phase.clone(),
                    detail: detail.clone(),
                    completed,
                };
                self.active_tools.insert(id.clone(), card.clone());

                if phase == "approval_required" {
                    self.phase = TurnPhase::ToolApproval;
                    effects.push(UiEffect::PhaseChanged(TurnPhase::ToolApproval));
                } else if phase == "running" {
                    self.phase = TurnPhase::ToolRunning;
                    effects.push(UiEffect::PhaseChanged(TurnPhase::ToolRunning));
                } else if completed && self.phase == TurnPhase::ToolRunning {
                    self.phase = TurnPhase::Generating;
                    effects.push(UiEffect::PhaseChanged(TurnPhase::Generating));
                } else if self.phase == TurnPhase::Idle {
                    self.phase = TurnPhase::Generating;
                    effects.push(UiEffect::PhaseChanged(TurnPhase::Generating));
                }

                effects.push(UiEffect::OperatorMessage(
                    OperatorMessaging::tool_message(name, phase, detail),
                ));
                effects.push(UiEffect::ToolUpdated(card));
            }
            StreamEvent::Step { summary, .. } => {
                if self.phase == TurnPhase::Idle {
                    self.phase = TurnPhase::Generating;
                    effects.push(UiEffect::PhaseChanged(TurnPhase::Generating));
                }
                effects.push(UiEffect::OperatorMessage(
                    OperatorMessaging::step_message(summary),
                ));
            }
            StreamEvent::Plan { title, .. } => {
                if self.phase == TurnPhase::Idle {
                    self.phase = TurnPhase::Generating;
                    effects.push(UiEffect::PhaseChanged(TurnPhase::Generating));
                }
                effects.push(UiEffect::OperatorMessage(
                    OperatorMessaging::plan_message(title),
                ));
            }
            StreamEvent::Done { .. } => {
                self.phase = TurnPhase::Terminal;
                effects.push(UiEffect::PhaseChanged(TurnPhase::Terminal));
                effects.push(UiEffect::TerminalDone);
                self.phase = TurnPhase::Idle;
                effects.push(UiEffect::PhaseChanged(TurnPhase::Idle));
            }
            StreamEvent::Error { message, code } => {
                self.phase = TurnPhase::Terminal;
                self.last_error = Some((code.clone(), message.clone()));
                effects.push(UiEffect::PhaseChanged(TurnPhase::Terminal));
                effects.push(UiEffect::TerminalError {
                    code: code.clone(),
                    message: message.clone(),
                });
                effects.push(UiEffect::OperatorMessage(
                    OperatorMessaging::error_hint(code, message),
                ));
                self.phase = TurnPhase::Idle;
                effects.push(UiEffect::PhaseChanged(TurnPhase::Idle));
            }
        }

        effects
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::StreamEvent;

    #[test]
    fn chunk_transitions_idle_to_generating() {
        let mut state = TurnState::new();
        let effects = state.apply(&StreamEvent::Chunk {
            index: 0,
            text: "hi".to_string(),
            turn_id: None,
            sequence: None,
        });
        assert_eq!(state.phase, TurnPhase::Generating);
        assert!(effects.iter().any(|e| matches!(e, UiEffect::AppendChunk(t) if t == "hi")));
    }

    #[test]
    fn tool_running_then_completed_returns_to_generating() {
        let mut state = TurnState::new();
        state.apply(&StreamEvent::Tool {
            index: 0,
            name: "fs.read".into(),
            phase: "running".into(),
            detail: "a.rs".into(),
            tool_call_id: Some("t1".into()),
            turn_id: None,
            sequence: None,
            elapsed_ms: None,
        });
        assert_eq!(state.phase, TurnPhase::ToolRunning);
        state.apply(&StreamEvent::Tool {
            index: 1,
            name: "fs.read".into(),
            phase: "completed".into(),
            detail: "ok".into(),
            tool_call_id: Some("t1".into()),
            turn_id: None,
            sequence: None,
            elapsed_ms: Some(10),
        });
        assert_eq!(state.phase, TurnPhase::Generating);
        assert!(state.active_tools.get("t1").unwrap().completed);
    }

    #[test]
    fn approval_required_enters_tool_approval() {
        let mut state = TurnState::new();
        state.apply(&StreamEvent::Tool {
            index: 0,
            name: "fs.write".into(),
            phase: "approval_required".into(),
            detail: "out.txt".into(),
            tool_call_id: Some("w1".into()),
            turn_id: None,
            sequence: None,
            elapsed_ms: None,
        });
        assert_eq!(state.phase, TurnPhase::ToolApproval);
    }
}
