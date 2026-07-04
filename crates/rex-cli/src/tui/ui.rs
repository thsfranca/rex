//! Ratatui layout and draw helpers (R080 presentation).
//!
//! Regions and breakpoints follow `docs/TUI_DESIGN.md` (spatial permanence).

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap};
use ratatui::Frame;

use super::state::{AppState, FocusPane, SessionPhase};

/// Micro profile: below this width, show “too small” only.
const MICRO_COLS: u16 = 60;
const TIMELINE_WIDE: u16 = 30;
const TIMELINE_STANDARD: u16 = 24;
const SHORT_HEIGHT: u16 = 24;
const COMPOSER_DEFAULT_H: u16 = 3;
const COMPOSER_SHORT_MAX_H: u16 = 5;

pub fn draw(frame: &mut Frame, app: &AppState) {
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

    draw_header(frame, chunks[0], app);
    draw_body(frame, chunks[1], app);
    draw_composer(frame, chunks[2], app);
    draw_footer(frame, chunks[3], app);
    if app.pending_approval.is_some() {
        draw_approval_modal(frame, app);
    }
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
    let phase = if app.session == SessionPhase::Streaming {
        Span::styled(format!("{} ", app.spinner_frame()), phase_style)
    } else {
        Span::styled(format!("{} ", app.phase_glyph()), phase_style)
    };

    let mut spans = vec![
        phase,
        Span::styled(app.workspace_basename(), app.theme.text_primary()),
        Span::styled(format!(" {} ", app.mode_glyph()), app.theme.text_accent()),
    ];
    if app.bypass {
        spans.push(Span::styled("⚡", app.theme.status_warning()));
    }
    if app.help_expanded {
        spans.push(Span::styled(
            format!(
                "  {} · {} · {}",
                app.phase_label(),
                app.mode,
                app.model_id
            ),
            app.theme.text_tertiary(),
        ));
    }

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn draw_body(frame: &mut Frame, area: Rect, app: &AppState) {
    let pad = transcript_pad(area.width);
    match timeline_width(area.width) {
        None => {
            let transcript = pad_rect(area, pad, 0);
            draw_transcript(frame, transcript, app);
        }
        Some(tl_w) => {
            let cols = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Min(10),
                    Constraint::Length(tl_w),
                ])
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

fn draw_timeline(frame: &mut Frame, area: Rect, app: &AppState) {
    let focused = app.focus == FocusPane::Activity;
    let items: Vec<ListItem> = app
        .activity
        .iter()
        .map(|item| {
            let line = if app.help_expanded {
                if let Some(detail) = &item.detail {
                    format!("{}  ({})", item.summary, detail)
                } else {
                    item.summary.clone()
                }
            } else {
                item.summary.clone()
            };
            ListItem::new(Span::styled(line, app.theme.text_tertiary()))
        })
        .collect();
    // Left hairline only — no titled box (Quiet Chrome).
    let list = List::new(items).block(
        Block::default()
            .borders(Borders::LEFT)
            .border_style(app.theme.hairline(focused)),
    );
    frame.render_widget(list, area);
}

fn draw_transcript(frame: &mut Frame, area: Rect, app: &AppState) {
    let focused = app.focus == FocusPane::Output;
    let mut text = app.output_lines.join("");
    if app.session == SessionPhase::Streaming {
        text.push_str(if app.tick % 2 == 0 { "▌" } else { " " });
    }
    // No outer box; focus is reserved for hairlines on adjacent chrome.
    let _ = focused;
    let widget = Paragraph::new(text)
        .style(app.theme.text_secondary())
        .wrap(Wrap { trim: false });
    frame.render_widget(widget, area);
}

fn draw_composer(frame: &mut Frame, area: Rect, app: &AppState) {
    let focused = app.focus == FocusPane::Composer;
    let line = Line::from(vec![
        Span::styled(format!("{} ", app.mode_glyph()), app.theme.text_accent()),
        if app.composer.is_empty() {
            Span::styled("Type your prompt…", app.theme.text_tertiary())
        } else {
            Span::styled(app.composer.as_str(), app.theme.text_primary())
        },
    ]);
    // Top hairline only; focus uses hairline.focus.
    let composer = Paragraph::new(line).block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(app.theme.hairline(focused)),
    );
    frame.render_widget(composer, area);
}

fn draw_footer(frame: &mut Frame, area: Rect, app: &AppState) {
    let line = if app.help_expanded {
        let path = &app.workspace_root;
        let ver = &app.daemon_version;
        format!(
            "↵ submit  esc cancel  ⇧⇥ mode  ? less  ^y bypass  ^c×2 quit  ·  {path}  ·  v{ver}"
        )
    } else if let Some(msg) = &app.status_message {
        msg.clone()
    } else {
        "↵  esc  ⇧⇥  ?".to_string()
    };
    frame.render_widget(
        Paragraph::new(line).style(app.theme.text_tertiary()),
        area,
    );
}

fn draw_approval_modal(frame: &mut Frame, app: &AppState) {
    let Some(pending) = app.pending_approval.as_ref() else {
        return;
    };
    let area = centered_rect(60, 40, frame.area());
    frame.render_widget(Clear, area);
    let summary = AppState::approval_summary(pending);
    let mut body = format!("{summary}\n\nA approve   D deny");
    if app.help_expanded {
        body.push_str(&format!("\n\n{} · {}", pending.name, pending.detail));
    }
    let modal = Paragraph::new(body)
        .style(app.theme.text_primary())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(app.theme.hairline(true))
                .title(Span::styled(
                    " ◎ ",
                    app.theme.status_warning().add_modifier(Modifier::BOLD),
                )),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(modal, area);
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
}
