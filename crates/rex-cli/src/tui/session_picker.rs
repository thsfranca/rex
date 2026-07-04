//! Pre-chat closed-session picker (`rex --continue`) — horizontal carousel.

use std::io;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Terminal;

use crate::session_resume::{format_relative_closed_at, ClosedSessionItem};

use super::compositor::carousel_adjacent_glyph;
use super::motion::MotionState;
use super::theme::Theme;

const MICRO_COLS: u16 = 60;

pub fn run_picker(
    workspace_basename: &str,
    sessions: &[ClosedSessionItem],
) -> Result<Option<String>, String> {
    let mut terminal = setup_terminal()?;
    let result = picker_loop(&mut terminal, workspace_basename, sessions);
    teardown_terminal(&mut terminal)?;
    result
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>, String> {
    crossterm::terminal::enable_raw_mode().map_err(|e| e.to_string())?;
    if rex_config::load_merged()
        .map(|c| c.effective.cli.ui.sync_output)
        .unwrap_or(true)
    {
        let _ = crossterm::execute!(
            io::stdout(),
            crossterm::terminal::BeginSynchronizedUpdate
        );
    }
    crossterm::execute!(io::stdout(), crossterm::terminal::EnterAlternateScreen)
        .map_err(|e| e.to_string())?;
    let backend = CrosstermBackend::new(io::stdout());
    Terminal::new(backend).map_err(|e| e.to_string())
}

fn teardown_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<(), String> {
    if rex_config::load_merged()
        .map(|c| c.effective.cli.ui.sync_output)
        .unwrap_or(true)
    {
        let _ = crossterm::execute!(
            io::stdout(),
            crossterm::terminal::EndSynchronizedUpdate
        );
    }
    crossterm::terminal::disable_raw_mode().map_err(|e| e.to_string())?;
    crossterm::execute!(terminal.backend_mut(), crossterm::terminal::LeaveAlternateScreen)
        .map_err(|e| e.to_string())?;
    terminal.show_cursor().map_err(|e| e.to_string())
}

fn picker_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    workspace_basename: &str,
    sessions: &[ClosedSessionItem],
) -> Result<Option<String>, String> {
    let theme = Theme::default_adaptive();
    let mut motion = MotionState::default();
    let mut selected = 0usize;
    let mut needs_draw = true;
    let mut was_animating = false;
    let sync_output = rex_config::load_merged()
        .map(|c| c.effective.cli.ui.sync_output)
        .unwrap_or(true);

    loop {
        let animating = motion.animating();
        if motion.wants_paint() || (was_animating && !animating) {
            needs_draw = true;
        }
        was_animating = animating;

        if needs_draw {
            let sync_this = motion.sync_output_enabled(sync_output);
            if sync_this {
                let _ = crossterm::execute!(
                    io::stdout(),
                    crossterm::terminal::BeginSynchronizedUpdate
                );
            }
            terminal
                .draw(|f| {
                    draw_picker(f, workspace_basename, sessions, selected, &theme, &mut motion)
                })
                .map_err(|e| e.to_string())?;
            if sync_this {
                let _ = crossterm::execute!(
                    io::stdout(),
                    crossterm::terminal::EndSynchronizedUpdate
                );
            }
            needs_draw = motion.wants_paint();
        }

        let poll_ms = motion.poll_ms();
        if event::poll(Duration::from_millis(poll_ms)).map_err(|e| e.to_string())? {
            match event::read().map_err(|e| e.to_string())? {
                Event::Key(key) => match key.code {
                    KeyCode::Esc => return Ok(None),
                    KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        return Ok(None)
                    }
                    KeyCode::Up | KeyCode::Char('k') | KeyCode::Left => {
                        if selected > 0 {
                            selected -= 1;
                            motion.on_composer_input();
                            needs_draw = true;
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') | KeyCode::Right => {
                        if selected + 1 < sessions.len() {
                            selected += 1;
                            motion.on_composer_input();
                            needs_draw = true;
                        }
                    }
                    KeyCode::Enter if !sessions.is_empty() => {
                        return Ok(Some(sessions[selected].harness_session_id.clone()));
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }
}

fn draw_picker(
    frame: &mut ratatui::Frame,
    workspace_basename: &str,
    sessions: &[ClosedSessionItem],
    selected: usize,
    theme: &Theme,
    motion: &mut MotionState,
) {
    let area = frame.area();
    if area.width < MICRO_COLS {
        let msg = format!(
            "Terminal too small — resize to continue.\n{} cols × {} rows (need ≥ {} cols)",
            area.width, area.height, MICRO_COLS
        );
        frame.render_widget(Paragraph::new(msg).style(theme.status_warning()), area);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(3),
            Constraint::Length(1),
        ])
        .split(area);

    motion.viewport = area;
    motion.header = chunks[0];

    let header = Line::from(vec![
        Span::styled("● ", theme.status_success()),
        Span::styled(workspace_basename.to_string(), theme.text_primary()),
        Span::styled(" ○", theme.text_accent()),
    ]);
    frame.render_widget(Paragraph::new(header), chunks[0]);

    if sessions.is_empty() {
        frame.render_widget(
            Paragraph::new("No closed sessions").style(theme.text_tertiary()),
            chunks[1],
        );
    } else {
        let prev = selected.checked_sub(1);
        let next = selected.checked_add(1).filter(|&i| i < sessions.len());
        let cur = &sessions[selected];
        let time = format_relative_closed_at(&cur.closed_at);
        let mut spans = vec![
            Span::styled("◁ ", theme.text_tertiary()),
        ];
        if let Some(p) = prev {
            spans.push(Span::styled(
                format!("{} ", carousel_adjacent_glyph(0.3)),
                theme.text_tertiary(),
            ));
            spans.push(Span::styled(
                format!("{} ", sessions[p].title),
                theme.text_tertiary(),
            ));
        }
        spans.push(Span::styled(cur.title.clone(), theme.text_accent()));
        if let Some(n) = next {
            spans.push(Span::styled(
                format!(" {}", sessions[n].title),
                theme.text_tertiary(),
            ));
            spans.push(Span::styled(
                format!("{}", carousel_adjacent_glyph(0.3)),
                theme.text_tertiary(),
            ));
        }
        spans.push(Span::styled(" ▷", theme.text_tertiary()));
        frame.render_widget(
            Paragraph::new(Line::from(spans)).alignment(Alignment::Center),
            chunks[1],
        );
        frame.render_widget(
            Paragraph::new(time).style(theme.text_tertiary()).alignment(Alignment::Center),
            Rect {
                x: chunks[1].x,
                y: chunks[1].y.saturating_add(2),
                width: chunks[1].width,
                height: 1,
            },
        );
    }

    let footer = Paragraph::new("← → select · Enter open · Esc quit                        [?]")
        .style(theme.text_tertiary())
        .alignment(Alignment::Left);
    frame.render_widget(footer, chunks[2]);

    motion.process(frame.buffer_mut(), theme);
}
