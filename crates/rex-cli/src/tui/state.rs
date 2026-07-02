//! TUI application state.

use rex_stream_ui::TurnPhase;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionPhase {
    Idle,
    Streaming,
    Error,
}

#[derive(Debug, Clone)]
pub struct PendingApproval {
    pub tool_call_id: String,
    pub name: String,
    pub detail: String,
    pub approval_token: String,
}

#[derive(Debug)]
pub struct AppState {
    pub workspace_root: String,
    pub model_id: String,
    pub daemon_version: String,
    pub mode: String,
    pub composer: String,
    pub activity: Vec<String>,
    pub output_lines: Vec<String>,
    pub footer: String,
    pub session: SessionPhase,
    pub turn_phase: TurnPhase,
    pub bypass: bool,
    pub ctrl_c_armed: bool,
    pub pending_approval: Option<PendingApproval>,
}

impl AppState {
    pub fn new(workspace_root: String, model_id: String, daemon_version: String) -> Self {
        Self {
            workspace_root,
            model_id,
            daemon_version,
            mode: "ask".to_string(),
            composer: String::new(),
            activity: vec!["Rex is ready — type a prompt and press Enter.".to_string()],
            output_lines: Vec::new(),
            footer: "Enter: submit | Esc: cancel | Shift+Tab: mode | q: quit".to_string(),
            session: SessionPhase::Idle,
            turn_phase: TurnPhase::Idle,
            bypass: false,
            ctrl_c_armed: false,
            pending_approval: None,
        }
    }

    pub fn cycle_mode(&mut self) {
        self.mode = match self.mode.as_str() {
            "ask" => "plan".to_string(),
            "plan" => "agent".to_string(),
            _ => "ask".to_string(),
        };
        self.footer = format!("Mode: {}", self.mode);
    }

    pub fn push_activity(&mut self, line: String) {
        self.activity.push(line);
        if self.activity.len() > 200 {
            let drain = self.activity.len() - 200;
            self.activity.drain(0..drain);
        }
    }

    pub fn append_output(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        if let Some(last) = self.output_lines.last_mut() {
            last.push_str(text);
        } else {
            self.output_lines.push(text.to_string());
        }
    }

    pub fn begin_stream(&mut self) {
        self.session = SessionPhase::Streaming;
        self.push_activity(format!("Prompt submitted (mode={})", self.mode));
    }

    pub fn end_stream_ok(&mut self) {
        self.session = SessionPhase::Idle;
        self.turn_phase = TurnPhase::Idle;
        self.push_activity("Turn complete.".to_string());
    }

    pub fn end_stream_error(&mut self, message: String) {
        self.session = SessionPhase::Error;
        self.footer = message.clone();
        self.push_activity(message);
    }
}
