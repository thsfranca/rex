//! TUI application state.

use std::path::Path;

use rex_stream_ui::TurnPhase;

use super::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionPhase {
    Idle,
    Streaming,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusPane {
    Composer,
    Output,
    Activity,
}

impl FocusPane {
    pub fn next(self) -> Self {
        match self {
            Self::Composer => Self::Output,
            Self::Output => Self::Activity,
            Self::Activity => Self::Composer,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PendingApproval {
    pub tool_call_id: String,
    pub name: String,
    pub detail: String,
    pub approval_token: String,
}

#[derive(Debug, Clone)]
pub struct ActivityItem {
    /// Glyph + short human phrase (default timeline).
    pub summary: String,
    /// Technical detail (tool id / capability) — shown only when help is expanded.
    pub detail: Option<String>,
}

#[derive(Debug)]
pub struct AppState {
    pub workspace_root: String,
    pub model_id: String,
    pub daemon_version: String,
    pub mode: String,
    pub composer: String,
    pub activity: Vec<ActivityItem>,
    pub output_lines: Vec<String>,
    pub session: SessionPhase,
    pub turn_phase: TurnPhase,
    pub bypass: bool,
    pub ctrl_c_armed: bool,
    pub pending_approval: Option<PendingApproval>,
    pub focus: FocusPane,
    pub help_expanded: bool,
    pub theme: Theme,
    pub tick: u64,
    pub reduce_motion: bool,
    pub status_message: Option<String>,
}

impl AppState {
    pub fn new(workspace_root: String, model_id: String, daemon_version: String) -> Self {
        let reduce_motion = std::env::var_os("NO_COLOR").is_some()
            || std::env::var("REX_TUI_REDUCE_MOTION")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(false);
        Self {
            workspace_root,
            model_id,
            daemon_version,
            mode: "ask".to_string(),
            composer: String::new(),
            activity: vec![ActivityItem {
                summary: "● Ready — type a prompt and press Enter".to_string(),
                detail: None,
            }],
            output_lines: Vec::new(),
            session: SessionPhase::Idle,
            turn_phase: TurnPhase::Idle,
            bypass: false,
            ctrl_c_armed: false,
            pending_approval: None,
            focus: FocusPane::Composer,
            help_expanded: false,
            theme: Theme::default_adaptive(),
            tick: 0,
            reduce_motion,
            status_message: None,
        }
    }

    pub fn workspace_basename(&self) -> &str {
        Path::new(&self.workspace_root)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(&self.workspace_root)
    }

    pub fn mode_glyph(&self) -> &'static str {
        match self.mode.as_str() {
            "plan" => "◇",
            "agent" => "◆",
            _ => "○",
        }
    }

    pub fn phase_glyph(&self) -> &'static str {
        match self.session {
            SessionPhase::Idle => "●",
            SessionPhase::Streaming => "◉",
            SessionPhase::Error => "✖",
        }
    }

    pub fn phase_label(&self) -> &'static str {
        match self.session {
            SessionPhase::Idle => "ready",
            SessionPhase::Streaming => "working",
            SessionPhase::Error => "error",
        }
    }

    pub fn cycle_mode(&mut self) {
        self.mode = match self.mode.as_str() {
            "ask" => "plan".to_string(),
            "plan" => "agent".to_string(),
            _ => "ask".to_string(),
        };
        self.status_message = Some(format!("Mode {}", self.mode));
    }

    pub fn cycle_focus(&mut self) {
        self.focus = self.focus.next();
    }

    pub fn toggle_help(&mut self) {
        self.help_expanded = !self.help_expanded;
    }

    pub fn push_activity(&mut self, summary: impl Into<String>, detail: Option<String>) {
        self.activity.push(ActivityItem {
            summary: summary.into(),
            detail,
        });
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
        self.status_message = None;
        self.push_activity(
            format!("{} Working…", self.mode_glyph()),
            Some(format!("mode={}", self.mode)),
        );
    }

    pub fn end_stream_ok(&mut self) {
        self.session = SessionPhase::Idle;
        self.turn_phase = TurnPhase::Idle;
        self.push_activity("✓ Done", None);
    }

    pub fn end_stream_error(&mut self, message: String) {
        self.session = SessionPhase::Error;
        self.status_message = Some(message.clone());
        self.push_activity(format!("✖ {message}"), None);
    }

    pub fn spinner_frame(&self) -> char {
        if self.reduce_motion {
            return '…';
        }
        const FRAMES: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
        FRAMES[(self.tick as usize) % FRAMES.len()]
    }

    pub fn humanize_tool_phase(name: &str, phase: &str) -> (String, Option<String>) {
        let detail = Some(format!("{name} · {phase}"));
        let summary = match phase {
            "running" => "▸ Working on a change…".to_string(),
            "completed" => "✓ Step finished".to_string(),
            "failed" => "✖ Step failed".to_string(),
            "approval_required" => "◎ Needs your approval".to_string(),
            _ => format!("· {phase}"),
        };
        (summary, detail)
    }

    pub fn approval_summary(_pending: &PendingApproval) -> String {
        "Allow this action to continue?".to_string()
    }
}
