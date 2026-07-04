//! Ratatui application loop.

use std::io;
use std::time::{Duration, SystemTime};

use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use rex_proto::rex::v1::GetSystemStatusRequest;
use rex_stream_ui::UiEffect;
use tokio::sync::mpsc::Receiver;
use tokio::time::timeout;

use crate::domain::REQUEST_TIMEOUT_SECONDS;
use crate::error::CliError;
use crate::transport::connect_client;

use super::approval::respond_to_tool_approval;

use super::state::{AppState, PendingApproval, SessionPhase};
use super::stream_task::{spawn_stream_task, StreamUpdate};
use super::ui;

pub async fn run() -> Result<(), String> {
    let mut terminal = setup_terminal()?;
    let result = run_app(&mut terminal).await;
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

async fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<(), String> {
    let status = fetch_status().await.map_err(|e| e.to_string())?;
    let mut app = AppState::new(
        status.workspace_root,
        status.active_model_id,
        status.daemon_version,
    );
    let mut stream_rx: Option<Receiver<StreamUpdate>> = None;
    // Paint only when dirty. Idle with no changes must not write CSI (tuiwright Quiet ≥300ms).
    let mut needs_draw = true;
    let mut was_animating = false;
    let mut last_size = terminal.size().map_err(|e| e.to_string())?;
    // Load once; avoid per-frame config I/O that could perturb timing.
    let sync_output = rex_config::load_merged()
        .map(|c| c.effective.cli.ui.sync_output)
        .unwrap_or(true);

    loop {
        // Prefer size polling over Resize events alone (some hosts omit events).
        if let Ok(size) = terminal.size() {
            if size.width != last_size.width || size.height != last_size.height {
                last_size = size;
                needs_draw = true;
            }
        }

        let animating = app.motion.animating();
        if animating || (was_animating && !animating) {
            // Continuous frames while motion runs; one settle frame when it ends.
            needs_draw = true;
        }
        was_animating = animating;

        if needs_draw {
            app.tick = app.tick.wrapping_add(1);
            // Tear-free frames: wrap each draw in synchronized output when enabled.
            if sync_output {
                let _ = crossterm::execute!(
                    io::stdout(),
                    crossterm::terminal::BeginSynchronizedUpdate
                );
            }
            terminal
                .draw(|f| ui::draw(f, &app))
                .map_err(|e| e.to_string())?;
            if sync_output {
                let _ = crossterm::execute!(
                    io::stdout(),
                    crossterm::terminal::EndSynchronizedUpdate
                );
            }
            // Stay dirty only while animating (~15–30 FPS via poll_ms).
            needs_draw = app.motion.animating();
        }

        if let Some(rx) = stream_rx.as_mut() {
            let mut got_update = false;
            while let Ok(update) = rx.try_recv() {
                apply_stream_update(&mut app, update);
                got_update = true;
            }
            if got_update {
                needs_draw = true;
            }
        }

        // ~30 FPS while motion cues run; idle blocks longer on poll (no paint unless dirty).
        let poll_ms = app.motion.poll_ms();

        if event::poll(Duration::from_millis(poll_ms)).map_err(|e| e.to_string())? {
            match event::read().map_err(|e| e.to_string())? {
                Event::Resize(cols, rows) => {
                    // Ignore no-op / spurious resize events (rmux can emit these).
                    if cols != last_size.width || rows != last_size.height {
                        last_size.width = cols;
                        last_size.height = rows;
                        needs_draw = true;
                    }
                }
                Event::Key(key) => {
                    let mut dirty = false;
                    match key.code {
                        KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            break
                        }
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            if app.ctrl_c_armed {
                                break;
                            }
                            app.ctrl_c_armed = true;
                            app.status_message = Some("Press Ctrl+C again to exit".to_string());
                            if app.session == SessionPhase::Streaming {
                                stream_rx = None;
                                app.cancel_stream();
                                app.status_message = Some("Turn canceled".to_string());
                            }
                            dirty = true;
                        }
                        KeyCode::Esc => {
                            app.ctrl_c_armed = false;
                            if app.session == SessionPhase::Streaming {
                                stream_rx = None;
                                app.cancel_stream();
                                app.status_message = Some("Turn canceled".to_string());
                            }
                            dirty = true;
                        }
                        KeyCode::Tab if key.modifiers.contains(KeyModifiers::SHIFT) => {
                            app.cycle_mode();
                            dirty = true;
                        }
                        KeyCode::Tab => {
                            app.cycle_focus();
                            dirty = true;
                        }
                        KeyCode::Char('?') => {
                            app.toggle_help();
                            dirty = true;
                        }
                        KeyCode::Char('a') | KeyCode::Char('A')
                            if app.pending_approval.is_some() =>
                        {
                            if let Some(pending) = app.pending_approval.take() {
                                app.motion.on_approval_close();
                                let token = pending.approval_token.clone();
                                let tool_call_id = pending.tool_call_id.clone();
                                match respond_to_tool_approval(&token, &tool_call_id, true).await {
                                    Ok(()) => {
                                        app.push_activity("✓ Approved", Some(pending.name))
                                    }
                                    Err(err) => app.push_activity(
                                        format!("✖ Approval failed: {err}"),
                                        None,
                                    ),
                                }
                                app.status_message = None;
                                dirty = true;
                            }
                        }
                        KeyCode::Char('d') | KeyCode::Char('D')
                            if app.pending_approval.is_some() =>
                        {
                            if let Some(pending) = app.pending_approval.take() {
                                app.motion.on_approval_close();
                                let token = pending.approval_token.clone();
                                let tool_call_id = pending.tool_call_id.clone();
                                let _ =
                                    respond_to_tool_approval(&token, &tool_call_id, false).await;
                                app.push_activity("○ Denied", Some(pending.name));
                                app.status_message = None;
                                dirty = true;
                            }
                        }
                        KeyCode::Char('y') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            app.bypass = !app.bypass;
                            app.status_message = Some(if app.bypass {
                                "Bypass on".to_string()
                            } else {
                                "Bypass off".to_string()
                            });
                            dirty = true;
                        }
                        KeyCode::Enter if app.session == SessionPhase::Idle => {
                            let prompt = app.composer.trim().to_string();
                            if !prompt.is_empty() {
                                app.composer.clear();
                                app.submit_prompt(prompt.clone());
                                let trace_id = resolve_trace_id();
                                match spawn_stream_task(prompt, app.mode.clone(), trace_id).await {
                                    Ok(rx) => stream_rx = Some(rx),
                                    Err(err) => app.end_stream_error(err.to_string()),
                                }
                                dirty = true;
                            }
                        }
                        KeyCode::Char(ch) if app.session == SessionPhase::Idle => {
                            app.composer.push(ch);
                            dirty = true;
                        }
                        KeyCode::Backspace if app.session == SessionPhase::Idle => {
                            app.composer.pop();
                            dirty = true;
                        }
                        _ => {}
                    }
                    if dirty {
                        needs_draw = true;
                    }
                }
                _ => {}
            }
        }

        if let Some(rx) = stream_rx.as_ref() {
            if rx.is_closed() && app.session == SessionPhase::Streaming {
                stream_rx = None;
                app.end_stream_ok();
                needs_draw = true;
            }
        }
    }
    Ok(())
}

fn apply_stream_update(app: &mut AppState, update: StreamUpdate) {
    match update {
        StreamUpdate::Effects(effects) => {
            for effect in effects {
                match effect {
                    UiEffect::AppendChunk(text) => app.append_output(&text),
                    UiEffect::OperatorMessage(msg) => app.push_activity(msg, None),
                    UiEffect::ToolUpdated(card) => {
                        let (summary, detail) =
                            AppState::humanize_tool_phase(&card.name, &card.phase);
                        app.push_activity(summary, detail);
                        if card.phase == "approval_required" {
                            let token = card
                                .detail
                                .strip_prefix("approval_required:")
                                .unwrap_or(&card.detail)
                                .to_string();
                            app.pending_approval = Some(PendingApproval {
                                tool_call_id: card.tool_call_id.clone(),
                                name: card.name.clone(),
                                detail: card.detail.clone(),
                                approval_token: token,
                            });
                            app.motion.on_approval_open();
                            app.status_message = Some("A approve · D deny".to_string());
                        }
                    }
                    UiEffect::PhaseChanged(phase) => app.turn_phase = phase,
                    UiEffect::Ignored => {}
                    UiEffect::TerminalDone => app.end_stream_ok(),
                    UiEffect::TerminalError { code, message } => {
                        app.end_stream_error(format!("{code}: {message}"));
                    }
                }
            }
        }
        StreamUpdate::Completed => app.end_stream_ok(),
        StreamUpdate::Failed(msg) => app.end_stream_error(msg),
    }
}

async fn fetch_status() -> Result<rex_proto::rex::v1::GetSystemStatusResponse, CliError> {
    let mut client = connect_client(None).await?;
    let mut request = tonic::Request::new(GetSystemStatusRequest {});
    request.set_timeout(Duration::from_secs(REQUEST_TIMEOUT_SECONDS));
    let response = timeout(
        Duration::from_secs(REQUEST_TIMEOUT_SECONDS),
        client.get_system_status(request),
    )
    .await
    .map_err(|_| CliError::StreamTimeout {
        seconds: REQUEST_TIMEOUT_SECONDS,
    })?
    .map_err(CliError::Status)?;
    Ok(response.into_inner())
}

fn resolve_trace_id() -> String {
    let millis = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|v| v.as_millis())
        .unwrap_or(0);
    format!("rex-tui-{millis}-{}", std::process::id())
}
