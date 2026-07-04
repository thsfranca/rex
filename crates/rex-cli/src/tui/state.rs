//! TUI application state.

use std::path::Path;

use mdstream::{DocumentState, MdStream, Options};
use rex_stream_ui::TurnPhase;

use super::motion::MotionState;
use super::theme::Theme;
use super::viewport::ViewportCache;
use crate::harness_session;
use crate::session_meta::read_meta;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TranscriptRole {
    Operator,
    Agent,
}

#[derive(Debug, Clone)]
pub struct TranscriptMessage {
    pub role: TranscriptRole,
    pub body: String,
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
    /// Completed transcript turns (operator prompts + finalized agent replies).
    pub messages: Vec<TranscriptMessage>,
    /// Incremental markdown for the in-flight agent reply.
    md_stream: MdStream,
    md_doc: DocumentState,
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
    pub motion: MotionState,
    /// Per-terminal harness session; scopes daemon prefix/L1 caches (parallel harness).
    pub harness_session_id: String,
    /// Human-readable session title (header chrome; from `.meta.json`).
    pub session_title: String,
    pub viewport: ViewportCache,
    /// Lines scrolled up from the transcript bottom (0 = pinned to tail).
    pub transcript_scroll: u16,
    pub needs_viewport_sync: bool,
}

impl AppState {
    pub fn new(workspace_root: String, model_id: String, daemon_version: String) -> Self {
        Self {
            workspace_root,
            model_id,
            daemon_version,
            mode: "ask".to_string(),
            composer: String::new(),
            activity: Vec::new(),
            messages: Vec::new(),
            md_stream: MdStream::new(Options::default()),
            md_doc: DocumentState::new(),
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
            motion: MotionState::default(),
            harness_session_id: harness_session::new_harness_session_id(),
            session_title: String::new(),
            viewport: ViewportCache::default(),
            transcript_scroll: 0,
            needs_viewport_sync: false,
        }
    }

    pub fn from_run_config(
        workspace_root: String,
        model_id: String,
        daemon_version: String,
        harness_session_id: String,
        session_title: String,
    ) -> Self {
        let mut state = Self::new(workspace_root, model_id, daemon_version);
        state.harness_session_id = harness_session_id;
        state.session_title = session_title;
        state
    }

    pub fn refresh_session_title(&mut self, workspace: &Path) {
        let meta = read_meta(workspace, &self.harness_session_id);
        if !meta.title.is_empty() {
            self.session_title = meta.title;
        }
    }

    pub fn restore_from_events(&mut self, events: &[rex_proto::rex::v1::SessionEvent]) {
        use super::viewport::events_to_messages;
        self.messages = events_to_messages(events);
        if let Some(last) = events.last() {
            self.viewport.head_sequence = last.sequence;
            self.viewport.newest_sequence = last.sequence;
            if let Some(first) = events.first() {
                self.viewport.oldest_loaded_sequence = first.sequence;
            }
        }
        self.transcript_scroll = 0;
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

    pub fn timeline_idle(&self) -> bool {
        self.activity.is_empty()
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
        self.motion.on_timeline_add();
        if self.activity.len() > 200 {
            let drain = self.activity.len() - 200;
            self.activity.drain(0..drain);
        }
    }

    /// Commit operator prompt and start agent markdown stream.
    pub fn submit_prompt(&mut self, prompt: String) {
        self.messages.push(TranscriptMessage {
            role: TranscriptRole::Operator,
            body: prompt,
        });
        self.reset_agent_markdown();
        self.begin_stream();
    }

    pub fn append_output(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        let _ = self.md_doc.apply(self.md_stream.append(text));
    }

    /// Snapshot of committed + pending markdown blocks for the live agent reply.
    pub fn agent_markdown_blocks(&self) -> Vec<(mdstream::BlockKind, String)> {
        let mut blocks = Vec::new();
        for b in self.md_doc.committed() {
            blocks.push((b.kind, b.display_or_raw().to_string()));
        }
        if let Some(p) = self.md_doc.pending() {
            blocks.push((p.kind, p.display_or_raw().to_string()));
        }
        blocks
    }

    fn reset_agent_markdown(&mut self) {
        self.md_stream = MdStream::new(Options::default());
        self.md_doc = DocumentState::new();
    }

    fn finalize_agent_markdown(&mut self) {
        let _ = self.md_doc.apply(self.md_stream.finalize());
        let body = self
            .md_doc
            .committed()
            .iter()
            .map(|b| b.display_or_raw().to_string())
            .collect::<Vec<_>>()
            .join("\n\n");
        if !body.trim().is_empty() {
            self.messages.push(TranscriptMessage {
                role: TranscriptRole::Agent,
                body,
            });
        }
        self.reset_agent_markdown();
    }

    pub fn begin_stream(&mut self) {
        self.session = SessionPhase::Streaming;
        self.status_message = None;
        // Protocol fields only in detail (progressive disclosure).
        self.push_activity("▸ Working…".to_string(), Some(self.mode.clone()));
        self.motion.on_stream_start();
    }

    /// Discard in-flight agent markdown (cancel / interrupt).
    pub fn cancel_stream(&mut self) {
        self.reset_agent_markdown();
        self.session = SessionPhase::Idle;
        self.turn_phase = TurnPhase::Idle;
        self.motion.on_stream_end();
    }

    pub fn end_stream_ok(&mut self) {
        self.finalize_agent_markdown();
        self.session = SessionPhase::Idle;
        self.turn_phase = TurnPhase::Idle;
        self.push_activity("✓ Done", None);
        self.motion.on_stream_end();
        self.transcript_scroll = 0;
        self.needs_viewport_sync = true;
    }

    pub fn end_stream_error(&mut self, message: String) {
        self.finalize_agent_markdown();
        self.session = SessionPhase::Error;
        self.status_message = Some(message.clone());
        self.push_activity(format!("✖ {message}"), None);
        self.motion.on_error();
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

    pub fn transcript_messages(&self) -> Vec<&TranscriptMessage> {
        let mut out: Vec<&TranscriptMessage> = self.viewport.prefetch_older.iter().collect();
        out.extend(self.messages.iter());
        out
    }

    pub fn scroll_transcript_up(&mut self, lines: u16) {
        self.transcript_scroll = self.transcript_scroll.saturating_add(lines);
    }

    pub fn scroll_transcript_down(&mut self, lines: u16) {
        self.transcript_scroll = self.transcript_scroll.saturating_sub(lines);
    }

    pub fn approval_summary(pending: &PendingApproval) -> String {
        // Human-first permission copy; tool ids stay in detail for disclose.
        let action = human_tool_action(&pending.name);
        format!("Agent requests permission to continue ({action}).")
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

    #[test]
    fn idle_timeline_starts_empty() {
        let app = AppState::new("/tmp/ws".into(), "m".into(), "1".into());
        assert!(app.timeline_idle());
        assert!(app.messages.is_empty());
    }

    #[test]
    fn submit_prompt_records_operator_turn() {
        let mut app = AppState::new("/tmp/ws".into(), "m".into(), "1".into());
        app.submit_prompt("hello".into());
        assert_eq!(app.messages.len(), 1);
        assert_eq!(app.messages[0].role, TranscriptRole::Operator);
        assert_eq!(app.messages[0].body, "hello");
        assert_eq!(app.session, SessionPhase::Streaming);
    }

    #[test]
    fn append_output_feeds_mdstream() {
        let mut app = AppState::new("/tmp/ws".into(), "m".into(), "1".into());
        app.submit_prompt("q".into());
        app.append_output("# Title\n\n");
        app.append_output("body\n");
        let blocks = app.agent_markdown_blocks();
        assert!(!blocks.is_empty());
        let joined: String = blocks
            .iter()
            .map(|(_, t)| t.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(joined.contains("Title") || joined.contains("body"));
    }

    #[test]
    fn restore_from_events_rebuilds_messages() {
        use rex_proto::rex::v1::SessionEvent;
        let mut app = AppState::new("/tmp/ws".into(), "m".into(), "1".into());
        let events = vec![
            SessionEvent {
                sequence: 1,
                event: "operator_prompt".to_string(),
                text: "hello".to_string(),
                ..Default::default()
            },
            SessionEvent {
                sequence: 2,
                event: "chunk".to_string(),
                text: "hi".to_string(),
                ..Default::default()
            },
            SessionEvent {
                sequence: 3,
                event: "done".to_string(),
                text: String::new(),
                ..Default::default()
            },
        ];
        app.restore_from_events(&events);
        assert_eq!(app.messages.len(), 2);
        assert_eq!(app.viewport.newest_sequence, 3);
    }
}
