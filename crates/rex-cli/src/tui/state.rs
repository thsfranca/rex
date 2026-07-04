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
    pub status_message: Option<String>,
}

impl AppState {
    pub fn new(workspace_root: String, model_id: String, daemon_version: String) -> Self {
        Self {
            workspace_root,
            model_id,
            daemon_version,
            mode: "ask".to_string(),
            composer: String::new(),
            activity: vec![ActivityItem {
                summary: "○ No active tasks".to_string(),
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
        // Protocol fields only in detail (progressive disclosure).
        self.push_activity("▸ Working…".to_string(), Some(self.mode.clone()));
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

    pub fn humanize_tool_phase(name: &str, phase: &str) -> (String, Option<String>) {
        // Technical id only when disclosed (`?` / focus).
        let detail = Some(format!("{name} · {phase}"));
        let action = human_tool_action(name);
        let summary = match phase {
            "running" => format!("▸ {action}…"),
            "completed" => format!("✓ {action}"),
            "failed" => format!("✖ {action} failed"),
            "approval_required" => "◎ Needs your approval".to_string(),
            _ => format!("· {action}"),
        };
        (summary, detail)
    }

    pub fn approval_summary(_pending: &PendingApproval) -> String {
        "Allow this action to continue?".to_string()
    }
}

fn human_tool_action(name: &str) -> &'static str {
    let n = name.to_ascii_lowercase();
    if n.contains("read") || n.contains("fs.read") || n.contains("file.read") {
        "Reading file"
    } else if n.contains("write") || n.contains("fs.write") || n.contains("file.write") {
        "Writing file"
    } else if n.contains("search") || n.contains("grep") || n.contains("find") {
        "Searching"
    } else if n.contains("shell") || n.contains("exec") || n.contains("command") {
        "Running command"
    } else if n.contains("plan") {
        "Updating plan"
    } else {
        "Working on a change"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_summary_is_human_not_protocol() {
        let (summary, detail) = AppState::humanize_tool_phase("fs.read", "running");
        assert!(summary.contains("Reading file"));
        assert!(!summary.contains("fs.read"));
        assert!(detail.unwrap().contains("fs.read"));
    }

    #[test]
    fn begin_stream_keeps_mode_in_detail_only() {
        let mut app = AppState::new("/tmp/ws".into(), "m".into(), "1".into());
        app.begin_stream();
        let last = app.activity.last().unwrap();
        assert_eq!(last.summary, "▸ Working…");
        assert_eq!(last.detail.as_deref(), Some("ask"));
        assert!(!last.summary.contains("mode="));
    }
}
