//! Ratatui layout and draw helpers (R080 presentation, R081 motion cues).

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap};
use ratatui::Frame;

use super::state::{AppState, FocusPane, SessionPhase};

const MIN_COLS: u16 = 40;
const MIN_ROWS: u16 = 10;

pub fn draw(frame: &mut Frame, app: &AppState) {
    let area = frame.area();
    if area.width < MIN_COLS || area.height < MIN_ROWS {
        frame.render_widget(
            Paragraph::new("Terminal too small — resize to continue.")
                .style(app.theme.warning()),
            area,
        );
        return;
    }

    let header_h = if app.help_expanded { 2 } else { 1 };
    let composer_h = 3u16;
    let footer_h = if app.help_expanded { 2 } else { 1 };
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

fn draw_header(frame: &mut Frame, area: Rect, app: &AppState) {
    let phase_style = match app.session {
        SessionPhase::Idle => app.theme.success(),
        SessionPhase::Streaming => app.theme.accent(),
        SessionPhase::Error => app.theme.error(),
    };
    let phase = if app.session == SessionPhase::Streaming && !app.reduce_motion {
        Span::styled(format!("{} ", app.spinner_frame()), phase_style)
    } else {
        Span::styled(format!("{} ", app.phase_glyph()), phase_style)
    };

    let mut spans = vec![
        phase,
        Span::styled(app.workspace_basename(), app.theme.bright()),
        Span::styled(format!(" {} ", app.mode_glyph()), app.theme.accent()),
    ];
    if app.bypass {
        spans.push(Span::styled("⚡", app.theme.warning()));
    }
    if app.help_expanded {
        spans.push(Span::styled(
            format!(
                "  {} · {} · {}",
                app.phase_label(),
                app.mode,
                app.model_id
            ),
            app.theme.muted(),
        ));
    }

    // No block border here: a 1-row header cannot host both content and a border.
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn draw_body(frame: &mut Frame, area: Rect, app: &AppState) {
    let narrow = area.width < 80;
    if narrow {
        draw_output(frame, area, app, FocusPane::Output);
        return;
    }

    let activity_pct = if area.width < 120 { 28 } else { 32 };
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(activity_pct),
            Constraint::Percentage(100 - activity_pct),
        ])
        .split(area);

    draw_activity(frame, cols[0], app);
    draw_output(frame, cols[1], app, FocusPane::Output);
}

fn draw_activity(frame: &mut Frame, area: Rect, app: &AppState) {
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
            ListItem::new(Span::styled(line, app.theme.text()))
        })
        .collect();
    let title = if focused { " · " } else { " " };
    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(app.theme.border(focused))
            .title(Span::styled(title, app.theme.muted())),
    );
    frame.render_widget(list, area);
}

fn draw_output(frame: &mut Frame, area: Rect, app: &AppState, pane: FocusPane) {
    let focused = app.focus == pane;
    let mut text = app.output_lines.join("");
    if app.session == SessionPhase::Streaming {
        let caret = if app.reduce_motion {
            "▌"
        } else if app.tick % 2 == 0 {
            "▌"
        } else {
            " "
        };
        text.push_str(caret);
    }
    let title = if focused { " · " } else { " " };
    let widget = Paragraph::new(text)
        .style(app.theme.text())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(app.theme.border(focused))
                .title(Span::styled(title, app.theme.muted())),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(widget, area);
}

fn draw_composer(frame: &mut Frame, area: Rect, app: &AppState) {
    let focused = app.focus == FocusPane::Composer;
    let line = Line::from(vec![
        Span::styled(format!("{} ", app.mode_glyph()), app.theme.accent()),
        if app.composer.is_empty() {
            Span::styled("Type your prompt…", app.theme.muted())
        } else {
            Span::styled(app.composer.as_str(), app.theme.text())
        },
    ]);
    let composer = Paragraph::new(line).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(app.theme.border(focused)),
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
        Paragraph::new(line).style(app.theme.muted()),
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
        .style(app.theme.text())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(app.theme.border(true))
                .title(Span::styled(" ◎ ", app.theme.warning().add_modifier(Modifier::BOLD))),
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
