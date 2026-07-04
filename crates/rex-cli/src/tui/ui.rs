//! Ratatui layout and draw helpers (R080 presentation).
//!
//! Regions and breakpoints follow `docs/TUI_DESIGN.md` (spatial permanence).

use mdstream::BlockKind;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap};
use ratatui::Frame;

use super::state::{AppState, FocusPane, SessionPhase, TranscriptRole};

/// Micro profile: below this width, show “too small” only.
const MICRO_COLS: u16 = 60;
const TIMELINE_WIDE: u16 = 30;
const TIMELINE_STANDARD: u16 = 24;
const SHORT_HEIGHT: u16 = 24;
const COMPOSER_DEFAULT_H: u16 = 3;
const COMPOSER_SHORT_MAX_H: u16 = 5;

pub fn draw(frame: &mut Frame, app: &mut AppState) {
    let area = frame.area();
    if area.width < MICRO_COLS {
        let msg = format!(
            "Terminal too small — resize to continue.\n{} cols × {} rows (need ≥ {} cols)",
            area.width, area.height, MICRO_COLS
        );
        frame.render_widget(
            Paragraph::new(msg).style(app.theme.status_warning()),
            area,
        );
        return;
    }

    let header_h = 1u16;
    let footer_h = 1u16;
    let composer_h = composer_height(area.height, header_h, footer_h);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(header_h),
            Constraint::Min(3),
            Constraint::Length(composer_h),
            Constraint::Length(footer_h),
        ])
        .split(area);

    // Region targets for tachyonfx (post-process after widgets).
    app.motion.viewport = area;
    app.motion.header = chunks[0];
    app.motion.composer = chunks[2];
    app.motion.composer_hairline = Rect {
        x: chunks[2].x,
        y: chunks[2].y,
        width: chunks[2].width,
        height: 1,
    };
    match timeline_width(chunks[1].width) {
        None => {
            app.motion.transcript = pad_rect(chunks[1], transcript_pad(chunks[1].width), 0);
            app.motion.timeline = Rect::default();
        }
        Some(tl_w) => {
            let cols = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Min(10), Constraint::Length(tl_w)])
                .split(chunks[1]);
            app.motion.transcript = pad_rect(cols[0], transcript_pad(chunks[1].width), 0);
            app.motion.timeline = cols[1];
        }
    }
    let t = app.motion.transcript;
    app.motion.transcript_hairline = Rect {
        x: t.x,
        y: t.y,
        width: t.width,
        height: 1,
    };

    draw_header(frame, chunks[0], app);
    let layout_start = std::time::Instant::now();
    draw_body(frame, chunks[1], app);
    draw_composer(frame, chunks[2], app);
    draw_footer(frame, chunks[3], app);
    if app.pending_approval.is_some() {
        draw_approval_modal(frame, app);
    }
    let layout_ms = layout_start.elapsed().as_millis() as u64;
    app.motion.set_reflow_paused(layout_ms > 16);

    let theme = app.theme;
    app.motion.process(frame.buffer_mut(), &theme);
}

fn composer_height(total_h: u16, header_h: u16, footer_h: u16) -> u16 {
    if total_h <= SHORT_HEIGHT {
        // Transcript ≥ 50% of viewport; composer capped at 5 rows.
        let reserved = header_h + footer_h;
        let body_and_composer = total_h.saturating_sub(reserved);
        let min_transcript = (total_h / 2).max(1);
        let max_composer = body_and_composer.saturating_sub(min_transcript);
        COMPOSER_DEFAULT_H
            .min(COMPOSER_SHORT_MAX_H)
            .min(max_composer.max(1))
    } else {
        COMPOSER_DEFAULT_H
    }
}

fn transcript_pad(width: u16) -> u16 {
    if width >= 120 {
        2
    } else {
        1
    }
}

fn timeline_width(width: u16) -> Option<u16> {
    if width >= 120 {
        Some(TIMELINE_WIDE)
    } else if width >= 80 {
        Some(TIMELINE_STANDARD)
    } else {
        // Narrow (60–79): timeline unmounted.
        None
    }
}

fn draw_header(frame: &mut Frame, area: Rect, app: &AppState) {
    let phase_style = match app.session {
        SessionPhase::Idle => app.theme.status_success(),
        SessionPhase::Streaming => app.theme.status_working(),
        SessionPhase::Error => app.theme.status_error(),
    };
    // Calm status glyph only — no blink/spinner as primary activity signal.
    // Connect fade / error shift are tachyonfx post-process on this region.
    let phase = Span::styled(format!("{} ", app.phase_glyph()), phase_style);
    let name_style = app.theme.text_primary();

    let mut spans = vec![
        phase,
        Span::styled(app.workspace_basename().to_string(), name_style),
    ];
    if !app.session_title.is_empty() {
        spans.push(Span::raw(" · "));
        spans.push(Span::styled(
            app.session_title.clone(),
            app.theme.text_secondary(),
        ));
    }
    spans.push(Span::styled(format!(" {} ", app.mode_glyph()), app.theme.text_accent()));
    if app.bypass {
        spans.push(Span::styled("⚡", app.theme.status_warning()));
    }
    if app.help_expanded {
        spans.push(Span::styled(
            format!(
                "  {} · {} · {} · {}",
                app.phase_label(),
                app.mode,
                app.model_id,
                app.harness_session_id
            ),
            app.theme.text_tertiary(),
        ));
    }

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn draw_body(frame: &mut Frame, area: Rect, app: &mut AppState) {
    let pad = transcript_pad(area.width);
    match timeline_width(area.width) {
        None => {
            let transcript = pad_rect(area, pad, 0);
            draw_transcript(frame, transcript, app);
        }
        Some(tl_w) => {
            let cols = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Min(10), Constraint::Length(tl_w)])
                .split(area);
            let transcript = pad_rect(cols[0], pad, 0);
            draw_transcript(frame, transcript, app);
            draw_timeline(frame, cols[1], app);
        }
    }
}

fn pad_rect(area: Rect, pad_x: u16, pad_y: u16) -> Rect {
    let x = area.x.saturating_add(pad_x);
    let y = area.y.saturating_add(pad_y);
    let width = area.width.saturating_sub(pad_x.saturating_mul(2));
    let height = area.height.saturating_sub(pad_y.saturating_mul(2));
    Rect {
        x,
        y,
        width,
        height,
    }
}

fn draw_timeline(frame: &mut Frame, area: Rect, app: &mut AppState) {
    let focused = app.focus == FocusPane::Activity;
    let disclose = app.help_expanded || focused;
    let expanded = app.motion.expanded_timeline();

    let mut items: Vec<ListItem> = vec![ListItem::new(Span::styled(
        "○ Timeline".to_string(),
        app.theme.text_tertiary(),
    ))];

    if app.timeline_idle() {
        items.push(ListItem::new(Span::styled(
            "  No active tasks".to_string(),
            app.theme.text_tertiary(),
        )));
    } else {
        for (idx, item) in app.activity.iter().enumerate() {
            let line = if disclose {
                if let Some(detail) = &item.detail {
                    format!("  {}  ({})", item.summary, detail)
                } else {
                    format!("  {}", item.summary)
                }
            } else {
                format!("  {}", item.summary)
            };
            let style = if expanded == Some(idx) {
                app.theme.text_accent()
            } else {
                app.theme.text_tertiary()
            };
            items.push(ListItem::new(Span::styled(line, style)));
            if expanded == Some(idx) {
                if let Some(detail) = &item.detail {
                    items.push(ListItem::new(Span::styled(
                        format!("    ↳ {detail}"),
                        app.theme.text_secondary(),
                    )));
                }
            }
        }
    }

    // Left hairline only — no titled box (Quiet Chrome). Raised surface.
    let list = List::new(items).block(
        Block::default()
            .borders(Borders::LEFT)
            .border_style(app.theme.hairline(focused))
            .style(app.theme.surface_raised()),
    );
    frame.render_widget(list, area);
}

fn draw_transcript(frame: &mut Frame, area: Rect, app: &AppState) {
    let lines = transcript_lines(app);
    let line_count = lines.len() as u16;
    let visible = area.height.max(1);
    let max_scroll_from_bottom = line_count.saturating_sub(visible);
    let scroll_from_top = max_scroll_from_bottom
        .saturating_sub(app.transcript_scroll.min(max_scroll_from_bottom));
    let widget = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((scroll_from_top, 0));
    frame.render_widget(widget, area);
}

/// Transcript as the stage: role labels, message separation, code left accent bar.
fn transcript_lines(app: &AppState) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();

    if app.messages.is_empty()
        && app.viewport.prefetch_older.is_empty()
        && app.session != SessionPhase::Streaming
    {
        // Idle wireframe: calm stage with operator role cue.
        lines.push(Line::from(Span::styled(
            "[Operator]".to_string(),
            app.theme.text_tertiary(),
        )));
        return lines;
    }

    let all = app.transcript_messages();
    for (i, msg) in all.iter().enumerate() {
        let label = match msg.role {
            TranscriptRole::Operator => "[Operator]",
            TranscriptRole::Agent => "[Agent]",
        };
        lines.push(Line::from(Span::styled(
            label.to_string(),
            app.theme.text_tertiary(),
        )));
        lines.extend(render_message_body(&msg.body, msg.role, app));
        if i + 1 < all.len() || app.session == SessionPhase::Streaming {
            lines.push(Line::from(""));
        }
    }

    if app.session == SessionPhase::Streaming {
        lines.push(Line::from(Span::styled(
            "[Agent]".to_string(),
            app.theme.text_tertiary(),
        )));
        let blocks = app.agent_markdown_blocks();
        if blocks.is_empty() {
            lines.push(Line::from(Span::styled(
                "…".to_string(),
                app.theme.text_tertiary(),
            )));
        } else {
            for (kind, text) in blocks {
                lines.extend(render_md_block(kind, &text, app));
            }
        }
    }

    lines
}

fn render_message_body(
    body: &str,
    role: TranscriptRole,
    app: &AppState,
) -> Vec<Line<'static>> {
    match role {
        TranscriptRole::Operator => body
            .lines()
            .map(|line| {
                Line::from(Span::styled(
                    line.to_string(),
                    app.theme.text_primary(),
                ))
            })
            .collect(),
        TranscriptRole::Agent => {
            // Finalized agent text: treat as markdown-ish blocks split on blank lines.
            let mut lines = Vec::new();
            let mut in_code = false;
            for para in body.split("\n\n") {
                for line in para.lines() {
                    let trimmed = line.trim_start();
                    if trimmed.starts_with("```") {
                        in_code = !in_code;
                        lines.push(Line::from(Span::styled(
                            line.to_string(),
                            app.theme.text_tertiary(),
                        )));
                        continue;
                    }
                    if in_code {
                        lines.push(code_line(line, app));
                    } else {
                        lines.push(Line::from(Span::styled(
                            line.to_string(),
                            app.theme.text_secondary(),
                        )));
                    }
                }
                lines.push(Line::from(""));
            }
            if lines.last().is_some_and(|l| l.spans.is_empty()) {
                lines.pop();
            }
            lines
        }
    }
}

fn render_md_block(kind: BlockKind, text: &str, app: &AppState) -> Vec<Line<'static>> {
    match kind {
        BlockKind::CodeFence => {
            let mut lines = Vec::new();
            for (i, line) in text.lines().enumerate() {
                let trimmed = line.trim_start();
                if i == 0 || trimmed.starts_with("```") {
                    lines.push(Line::from(Span::styled(
                        line.to_string(),
                        app.theme.text_tertiary(),
                    )));
                } else {
                    lines.push(code_line(line, app));
                }
            }
            lines.push(Line::from(""));
            lines
        }
        BlockKind::Heading => text
            .lines()
            .map(|line| {
                Line::from(Span::styled(
                    line.to_string(),
                    app.theme.text_primary(),
                ))
            })
            .chain(std::iter::once(Line::from("")))
            .collect(),
        _ => text
            .lines()
            .map(|line| {
                Line::from(Span::styled(
                    line.to_string(),
                    app.theme.text_secondary(),
                ))
            })
            .chain(std::iter::once(Line::from("")))
            .collect(),
    }
}

fn code_line(line: &str, app: &AppState) -> Line<'static> {
    Line::from(vec![
        Span::styled("▌".to_string(), app.theme.text_accent()),
        Span::styled(format!(" {line}"), app.theme.text_secondary()),
    ])
}

fn draw_composer(frame: &mut Frame, area: Rect, app: &AppState) {
    let focused = app.focus == FocusPane::Composer;
    let line = if app.session == SessionPhase::Streaming {
        Line::from(Span::styled(
            "[ Agent is typing… ]".to_string(),
            app.theme.text_tertiary(),
        ))
    } else {
        let mut spans = vec![Span::styled("❯ ".to_string(), app.theme.text_accent())];
        if app.help_expanded {
            spans.push(Span::styled(
                format!("{} ", app.mode),
                app.theme.text_tertiary(),
            ));
        }
        spans.push(if app.composer.is_empty() {
            Span::styled("Type your prompt…".to_string(), app.theme.text_tertiary())
        } else {
            Span::styled(app.composer.clone(), app.theme.text_primary())
        });
        Line::from(spans)
    };
    // Flux / stream slide are tachyonfx region effects (not blink or spacer hacks).
    let composer = Paragraph::new(line).block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(app.theme.hairline(focused))
            .style(app.theme.surface_raised()),
    );
    frame.render_widget(composer, area);
}

fn draw_footer(frame: &mut Frame, area: Rect, app: &AppState) {
    // Minimal key glyphs by default; full help, path, version on `?`.
    let line = if app.help_expanded {
        let name = app.workspace_basename();
        let ver = &app.daemon_version;
        Line::from(Span::styled(
            format!(
                "↵ submit  esc cancel  ⇧⇥ mode  ? less  ^y bypass  ^c×2 quit  ·  {name}  ·  v{ver}"
            ),
            app.theme.text_tertiary(),
        ))
    } else if app.viewport.fetching_history {
        Line::from(Span::styled(
            "Loading earlier messages…".to_string(),
            app.theme.status_working(),
        ))
    } else if let Some(msg) = &app.status_message {
        Line::from(Span::styled(msg.clone(), app.theme.text_tertiary()))
    } else {
        let (glyph, label, style) = match app.session {
            SessionPhase::Idle => ("○", "Ready", app.theme.status_idle()),
            SessionPhase::Streaming => ("●", "Working…", app.theme.status_working()),
            SessionPhase::Error => ("✖", "Error", app.theme.status_error()),
        };
        Line::from(vec![
            Span::styled(format!("{glyph} "), style),
            Span::styled(format!("{label}  "), app.theme.text_tertiary()),
            Span::styled("[?]".to_string(), app.theme.text_tertiary()),
        ])
    };
    frame.render_widget(
        Paragraph::new(line).alignment(Alignment::Left),
        area,
    );
}

fn draw_approval_modal(frame: &mut Frame, app: &mut AppState) {
    let Some(pending) = app.pending_approval.as_ref() else {
        return;
    };
    let area = centered_rect(60, 40, frame.area());
    frame.render_widget(
        Block::default().style(app.theme.surface_dimmed()),
        frame.area(),
    );
    frame.render_widget(Clear, area);
    let summary = AppState::approval_summary(pending);
    let scrub = app.motion.diff_scrub_index();
    let mut body = format!(
        "◎ Action required\n\n{summary}\n\n←→ scrub diff ({scrub}) · [A] Approve · [D] Reject · [?] Details"
    );
    if app.help_expanded {
        body.push_str(&format!("\n\n{} · {}", pending.name, pending.detail));
    }
    let modal = Paragraph::new(body)
        .style(app.theme.text_primary())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(app.theme.hairline(true))
                .style(app.theme.surface_overlay()),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(modal, area);

    let diff_area = Rect {
        x: area.x.saturating_add(2),
        y: area.y.saturating_add(4),
        width: area.width.saturating_sub(4),
        height: 2,
    };
    let intensity = if app.motion.diff_scrub_max() == 0 {
        0.5
    } else {
        scrub as f32 / app.motion.diff_scrub_max().max(1) as f32
    };
    use super::compositor::effects;
    effects::apply_diff_scrub_line(frame.buffer_mut(), diff_area, true, intensity);
    let removed = Rect {
        x: diff_area.x,
        y: diff_area.y.saturating_add(1),
        width: diff_area.width,
        height: 1,
    };
    effects::apply_diff_scrub_line(frame.buffer_mut(), removed, false, 1.0 - intensity);
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeline_widths_match_breakpoints() {
        assert_eq!(timeline_width(130), Some(30));
        assert_eq!(timeline_width(100), Some(24));
        assert_eq!(timeline_width(70), None);
        assert_eq!(timeline_width(59), None);
    }

    #[test]
    fn transcript_padding_matches_breakpoints() {
        assert_eq!(transcript_pad(130), 2);
        assert_eq!(transcript_pad(100), 1);
        assert_eq!(transcript_pad(70), 1);
    }

    #[test]
    fn short_height_caps_composer() {
        let h = composer_height(20, 1, 1);
        assert!(h <= COMPOSER_SHORT_MAX_H);
        assert!(h >= 1);
    }

    #[test]
    fn idle_transcript_shows_operator_cue() {
        let app = AppState::new("/tmp/ws".into(), "m".into(), "1".into());
        let lines = transcript_lines(&app);
        let joined: String = lines
            .iter()
            .map(|l| {
                l.spans
                    .iter()
                    .map(|s| s.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(joined.contains("[Operator]"));
        assert!(!joined.contains("model="));
        assert!(!joined.contains("mode="));
    }

    #[test]
    fn transcript_code_uses_accent_bar() {
        let mut app = AppState::new("/tmp/ws".into(), "m".into(), "1".into());
        app.submit_prompt("q".into());
        app.append_output("```\ncode\n```\n");
        let lines = transcript_lines(&app);
        let joined: String = lines
            .iter()
            .map(|l| {
                l.spans
                    .iter()
                    .map(|s| s.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(joined.contains('▌'));
        assert!(joined.contains("code"));
        assert!(joined.contains("[Agent]"));
    }

    #[test]
    fn transcript_has_no_blink_caret_on_plain_text() {
        let mut app = AppState::new("/tmp/ws".into(), "m".into(), "1".into());
        app.submit_prompt("q".into());
        app.append_output("hi");
        let lines = transcript_lines(&app);
        let joined: String = lines
            .iter()
            .map(|l| {
                l.spans
                    .iter()
                    .map(|s| s.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("");
        assert!(joined.contains("hi"));
        assert!(!joined.contains('▌'));
    }
}
