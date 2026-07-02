//! Ratatui layout and draw helpers.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};
use ratatui::Frame;

use super::state::{AppState, SessionPhase};

pub fn draw(frame: &mut Frame, app: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(4),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(frame.area());

    draw_header(frame, chunks[0], app);
    draw_body(frame, chunks[1], app);
    draw_composer(frame, chunks[2], app);
    draw_footer(frame, chunks[3], app);
}

fn draw_header(frame: &mut Frame, area: Rect, app: &AppState) {
    let status = match app.session {
        SessionPhase::Idle => "idle",
        SessionPhase::Streaming => "streaming",
        SessionPhase::Error => "error",
    };
    let bypass = if app.bypass { " bypass:on" } else { "" };
    let header = format!(
        " Rex {} | {} | model={} | mode={}{} ",
        app.daemon_version, app.workspace_root, app.model_id, app.mode, bypass
    );
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            header,
            Style::default().add_modifier(Modifier::BOLD),
        ));
    frame.render_widget(Paragraph::new(Line::from(status)).block(block), area);
}

fn draw_body(frame: &mut Frame, area: Rect, app: &AppState) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(area);

    let activity_items: Vec<ListItem> = app
        .activity
        .iter()
        .map(|line| ListItem::new(line.as_str()))
        .collect();
    let activity = List::new(activity_items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Activity "),
    );
    frame.render_widget(activity, cols[0]);

    let output = app.output_lines.join("");
    let output_widget = Paragraph::new(output)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Output "),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(output_widget, cols[1]);
}

fn draw_composer(frame: &mut Frame, area: Rect, app: &AppState) {
    let prompt = if app.composer.is_empty() {
        "Type your prompt…".to_string()
    } else {
        app.composer.clone()
    };
    let style = if app.composer.is_empty() {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default()
    };
    let composer = Paragraph::new(prompt)
        .style(style)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Composer "),
        );
    frame.render_widget(composer, area);
}

fn draw_footer(frame: &mut Frame, area: Rect, app: &AppState) {
    let footer = Paragraph::new(app.footer.as_str()).style(Style::default().fg(Color::Cyan));
    frame.render_widget(footer, area);
}
