use std::io::{self, BufRead, IsTerminal, Write};
use std::process;
use std::process::ExitCode;
use std::time::{Duration, SystemTime};

use rex_proto::rex::v1::{GetSystemStatusRequest, StreamInferenceRequest};
use serde_json::json;
use tokio::time::{sleep, timeout};
use tonic::Code;

use crate::command::{parse_command, print_usage, CliCommand, CompleteOutputFormat};
use crate::domain::{
    StreamLifecycle, REQUEST_TIMEOUT_SECONDS, STREAM_ITEM_TIMEOUT_SECONDS,
    STREAM_START_RETRY_ATTEMPTS, STREAM_START_RETRY_DELAY_MS,
};
use crate::error::CliError;
use crate::transport::connect_client;

pub async fn run_cli(args: impl Iterator<Item = String>) -> ExitCode {
    match parse_command(args) {
        Ok(command) => {
            let complete_format = command.output_format();
            match execute(command).await {
                Ok(()) => ExitCode::SUCCESS,
                Err(err) => {
                    if matches!(complete_format, Some(CompleteOutputFormat::Ndjson)) {
                        let line =
                            format_ndjson_error_event(err.to_string(), ndjson_error_code(&err));
                        if let Err(io_err) = emit_ndjson_line_stdout(&line) {
                            eprintln!("Error: {io_err}");
                            return ExitCode::from(1);
                        }
                    } else {
                        eprintln!("Error: {err}");
                    }
                    ExitCode::from(1)
                }
            }
        }
        Err(message) => {
            eprintln!("{message}");
            print_usage();
            ExitCode::from(2)
        }
    }
}

async fn execute(command: CliCommand) -> Result<(), CliError> {
    match command {
        CliCommand::Status => run_status().await,
        CliCommand::Complete {
            prompt,
            model,
            mode,
            approval_id,
            trace_id,
            active_file_path,
            language_id,
            selection_text,
            format,
            yes,
            verbose,
        } => {
            run_complete(
                prompt,
                model,
                mode,
                approval_id,
                trace_id,
                active_file_path,
                language_id,
                selection_text,
                format,
                yes,
                verbose,
            )
            .await
        }
    }
}

async fn run_status() -> Result<(), CliError> {
    let mut client = connect_client(None).await?;
    let mut request = tonic::Request::new(GetSystemStatusRequest {});
    request.set_timeout(Duration::from_secs(REQUEST_TIMEOUT_SECONDS));
    let response = client.get_system_status(request).await?;
    let status = response.into_inner();

    println!("daemon_version: {}", status.daemon_version);
    println!("uptime_seconds: {}", status.uptime_seconds);
    println!("active_model_id: {}", status.active_model_id);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn run_complete(
    prompt: String,
    model: String,
    mode: String,
    approval_id: String,
    trace_id: String,
    active_file_path: String,
    language_id: String,
    selection_text: String,
    format: CompleteOutputFormat,
    yes: bool,
    verbose: bool,
) -> Result<(), CliError> {
    let approval_id = resolve_approval_id(&mode, &approval_id, yes, format)?;
    let stream_idle_timeout_secs = stream_idle_timeout_for_mode(&mode);
    let trace_id = resolve_trace_id(trace_id);
    eprintln!("trace_id={trace_id} phase=start operation=complete");
    let mut attempt: u32 = 0;
    loop {
        let mut client = match connect_client(Some(&trace_id)).await {
            Ok(client) => client,
            Err(err) if should_retry_stream_start(&err, attempt) => {
                attempt += 1;
                sleep(Duration::from_millis(STREAM_START_RETRY_DELAY_MS)).await;
                continue;
            }
            Err(err) => return Err(err),
        };
        let client_hints =
            if active_file_path.is_empty() && language_id.is_empty() && selection_text.is_empty() {
                None
            } else {
                Some(rex_proto::rex::v1::ClientHints {
                    active_file_path: active_file_path.clone(),
                    language_id: language_id.clone(),
                    selection_text: selection_text.clone(),
                })
            };
        let mut request = tonic::Request::new(StreamInferenceRequest {
            prompt: prompt.clone(),
            model: model.clone(),
            mode: mode.clone(),
            approval_id: approval_id.clone(),
            client_hints,
        });
        let metadata_value =
            tonic::metadata::MetadataValue::try_from(trace_id.as_str()).map_err(|_| {
                map_status_error(tonic::Status::invalid_argument("invalid trace id metadata"))
            })?;
        request
            .metadata_mut()
            .insert("x-rex-trace-id", metadata_value);
        request.set_timeout(Duration::from_secs(REQUEST_TIMEOUT_SECONDS));
        let response = match client
            .stream_inference(request)
            .await
            .map_err(map_status_error)
        {
            Ok(response) => response,
            Err(err) if should_retry_stream_start(&err, attempt) => {
                attempt += 1;
                sleep(Duration::from_millis(STREAM_START_RETRY_DELAY_MS)).await;
                continue;
            }
            Err(err) => return Err(err),
        };
        let mut stream = response.into_inner();
        let lifecycle =
            consume_stream(&mut stream, format, verbose, stream_idle_timeout_secs).await?;
        return match lifecycle {
            StreamLifecycle::Completed => {
                eprintln!("trace_id={trace_id} phase=terminal result=done");
                Ok(())
            }
            StreamLifecycle::Incomplete => {
                eprintln!("trace_id={trace_id} phase=terminal result=stream_incomplete");
                Err(CliError::StreamIncomplete)
            }
        };
    }
}

fn map_status_error(status: tonic::Status) -> CliError {
    match status.code() {
        Code::Unavailable => CliError::DaemonUnavailable {
            socket_path: crate::domain::SOCKET_PATH.to_string(),
        },
        _ => CliError::Status(status),
    }
}

fn map_stream_status_error(status: tonic::Status) -> CliError {
    let message = status.message().to_string();
    match status.code() {
        Code::DeadlineExceeded => CliError::StreamTimeout {
            seconds: STREAM_ITEM_TIMEOUT_SECONDS,
        },
        Code::Unavailable => CliError::StreamInterrupted,
        Code::FailedPrecondition if message.to_ascii_lowercase().contains("sidecar required") => {
            CliError::SidecarUnavailable { detail: message }
        }
        Code::FailedPrecondition if message.to_ascii_lowercase().contains("inference runtime") => {
            CliError::InferenceConfig { detail: message }
        }
        _ => CliError::Status(status),
    }
}

async fn consume_stream(
    stream: &mut tonic::Streaming<rex_proto::rex::v1::StreamInferenceResponse>,
    format: CompleteOutputFormat,
    verbose: bool,
    stream_idle_timeout_secs: u64,
) -> Result<StreamLifecycle, CliError> {
    loop {
        let next = timeout(
            Duration::from_secs(stream_idle_timeout_secs),
            stream.message(),
        )
        .await
        .map_err(|_| CliError::StreamTimeout {
            seconds: stream_idle_timeout_secs,
        })?;

        let maybe_chunk = next.map_err(map_stream_status_error)?;

        if matches!(
            classify_stream_terminal(maybe_chunk.as_ref()),
            Some(StreamLifecycle::Incomplete)
        ) {
            return Ok(StreamLifecycle::Incomplete);
        }
        let chunk = maybe_chunk.expect("incomplete stream should return early");

        if chunk.done {
            match format {
                CompleteOutputFormat::Text => println!(),
                CompleteOutputFormat::Ndjson => {
                    let line = format_ndjson_done_event(chunk.index);
                    emit_ndjson_line_stdout(&line).map_err(CliError::Stdout)?;
                }
            }
            return Ok(StreamLifecycle::Completed);
        }

        if let Some(line) = format_ndjson_stream_event(&chunk) {
            match format {
                CompleteOutputFormat::Text => {
                    if chunk.event.is_empty() || chunk.event == "chunk" {
                        print!("{}", chunk.text);
                    } else if verbose {
                        emit_verbose_status_line(&chunk)?;
                    }
                }
                CompleteOutputFormat::Ndjson => {
                    emit_ndjson_line_stdout(&line).map_err(CliError::Stdout)?;
                }
            }
        }
    }
}

fn stream_idle_timeout_for_mode(mode: &str) -> u64 {
    rex_config::load_merged()
        .map(|loaded| loaded.stream_idle_timeout_secs(mode))
        .unwrap_or(if mode.trim().eq_ignore_ascii_case("agent") {
            120
        } else {
            STREAM_ITEM_TIMEOUT_SECONDS
        })
}

fn resolve_approval_id(
    mode: &str,
    approval_id: &str,
    yes: bool,
    format: CompleteOutputFormat,
) -> Result<String, CliError> {
    if !mode.trim().eq_ignore_ascii_case("agent") {
        return Ok(approval_id.to_string());
    }
    let approvals_enabled = rex_config::load_merged()
        .map(|loaded| loaded.approvals_enabled())
        .unwrap_or(false);
    if !approvals_enabled {
        return Ok(approval_id.to_string());
    }
    if !approval_id.trim().is_empty() {
        return Ok(approval_id.trim().to_string());
    }
    if yes {
        eprintln!("warning: auto-approving agent execution (--yes)");
        return Ok(format!("apr-cli-{}", process::id()));
    }
    if matches!(format, CompleteOutputFormat::Ndjson) {
        let line = format_ndjson_step_event(0, "awaiting_approval", "Approve agent execution");
        emit_ndjson_line_stdout(&line).map_err(CliError::Stdout)?;
    }
    if !std::io::stdin().is_terminal() {
        return Err(CliError::ApprovalRequired);
    }
    eprint!("Approve agent execution for this prompt? [y/N] ");
    io::stderr().flush().ok();
    let mut answer = String::new();
    io::stdin()
        .lock()
        .read_line(&mut answer)
        .map_err(CliError::Stdout)?;
    if answer.trim().eq_ignore_ascii_case("y") || answer.trim().eq_ignore_ascii_case("yes") {
        return Ok(format!("apr-cli-{}", process::id()));
    }
    Err(CliError::ApprovalDenied)
}

fn emit_verbose_status_line(chunk: &rex_proto::rex::v1::StreamInferenceResponse) -> io::Result<()> {
    let event = chunk.event.trim();
    match event {
        "tool" => writeln!(
            io::stderr(),
            "[tool] {} {} {}",
            chunk.tool_name.trim(),
            chunk.phase.trim(),
            chunk.detail.trim()
        ),
        "step" | "activity" => writeln!(
            io::stderr(),
            "[{event}] {} {}",
            chunk.phase.trim(),
            chunk.summary.trim()
        ),
        "plan" => writeln!(
            io::stderr(),
            "[plan] {} {}",
            chunk.phase.trim(),
            chunk.summary.trim()
        ),
        _ => Ok(()),
    }
}

fn classify_stream_terminal(
    maybe_chunk: Option<&rex_proto::rex::v1::StreamInferenceResponse>,
) -> Option<StreamLifecycle> {
    match maybe_chunk {
        None => Some(StreamLifecycle::Incomplete),
        Some(chunk) if chunk.done => Some(StreamLifecycle::Completed),
        Some(_) => None,
    }
}

fn should_retry_stream_start(error: &CliError, attempt: u32) -> bool {
    matches!(error, CliError::DaemonUnavailable { .. }) && attempt < STREAM_START_RETRY_ATTEMPTS
}

/// Emit one NDJSON line to **stdout** and flush so consumers on a pipe observe events promptly.
fn emit_ndjson_line_stdout(payload: &str) -> io::Result<()> {
    write_ndjson_line(&mut io::stdout().lock(), payload)
}

/// Write a single JSON object as one line and flush (`writeln!` + flush).
fn write_ndjson_line(w: &mut impl Write, payload: &str) -> io::Result<()> {
    writeln!(w, "{payload}")?;
    w.flush()
}

fn format_ndjson_chunk_event(index: u64, text: &str) -> String {
    json!({
        "event": "chunk",
        "index": index,
        "text": text
    })
    .to_string()
}

fn format_ndjson_tool_event(chunk: &rex_proto::rex::v1::StreamInferenceResponse) -> String {
    let mut value = json!({
        "event": "tool",
        "index": chunk.index,
        "name": chunk.tool_name.trim(),
        "phase": chunk.phase.trim(),
        "detail": chunk.detail.trim(),
    });
    if !chunk.tool_call_id.is_empty() {
        value["tool_call_id"] = json!(chunk.tool_call_id);
    }
    if chunk.sequence > 0 {
        value["sequence"] = json!(chunk.sequence);
    }
    if chunk.elapsed_ms > 0 {
        value["elapsed_ms"] = json!(chunk.elapsed_ms);
    }
    if !chunk.turn_id.is_empty() {
        value["turn_id"] = json!(chunk.turn_id);
    }
    value.to_string()
}

fn format_ndjson_step_event(index: u64, phase: &str, summary: &str) -> String {
    json!({
        "event": "step",
        "index": index,
        "phase": phase,
        "summary": summary
    })
    .to_string()
}

fn format_ndjson_activity_event(chunk: &rex_proto::rex::v1::StreamInferenceResponse) -> String {
    let mut value = json!({
        "event": "activity",
        "index": chunk.index,
        "phase": chunk.phase.trim(),
        "summary": chunk.summary.trim(),
    });
    if !chunk.detail.is_empty() {
        value["detail"] = json!(chunk.detail.trim());
    }
    if chunk.sequence > 0 {
        value["sequence"] = json!(chunk.sequence);
    }
    value.to_string()
}

fn format_ndjson_plan_event(index: u64, phase: &str, title: &str, detail: &str) -> String {
    json!({
        "event": "plan",
        "index": index,
        "phase": phase,
        "title": title,
        "detail": detail
    })
    .to_string()
}

fn format_ndjson_stream_event(
    chunk: &rex_proto::rex::v1::StreamInferenceResponse,
) -> Option<String> {
    let event = chunk.event.trim();
    match event {
        "" | "chunk" => {
            if chunk.text.is_empty() {
                None
            } else {
                Some(format_ndjson_chunk_event(chunk.index, &chunk.text))
            }
        }
        "tool" => Some(format_ndjson_tool_event(chunk)),
        "step" => Some(format_ndjson_step_event(
            chunk.index,
            chunk.phase.trim(),
            chunk.summary.trim(),
        )),
        "activity" => Some(format_ndjson_activity_event(chunk)),
        "plan" => Some(format_ndjson_plan_event(
            chunk.index,
            chunk.phase.trim(),
            chunk.summary.trim(),
            chunk.detail.trim(),
        )),
        _ => None,
    }
}

fn format_ndjson_done_event(index: u64) -> String {
    json!({
        "event": "done",
        "index": index
    })
    .to_string()
}

fn format_ndjson_error_event(message: String, code: &'static str) -> String {
    json!({
        "event": "error",
        "message": message,
        "code": code
    })
    .to_string()
}

fn ndjson_error_code(err: &CliError) -> &'static str {
    match err {
        CliError::DaemonUnavailable { .. } => "daemon_unavailable",
        CliError::StreamTimeout { .. } => "stream_timeout",
        CliError::StreamInterrupted => "stream_interrupted",
        CliError::StreamIncomplete => "stream_incomplete",
        CliError::SidecarUnavailable { .. } => "sidecar_unavailable",
        CliError::InferenceConfig { .. } => "inference_config",
        CliError::DaemonConnect { .. } => "daemon_unavailable",
        CliError::Endpoint(_) | CliError::Status(_) => "unknown",
        CliError::Stdout(_) => "unknown",
        CliError::ApprovalRequired | CliError::ApprovalDenied => "approval_required",
    }
}

/// Counts NDJSON lines whose `event` is `done` or `error` (contract: exactly one per successful parse).
#[cfg(test)]
fn ndjson_terminal_event_count_for_tests(output: &str) -> usize {
    output
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter(|value| {
            value
                .get("event")
                .and_then(|event| event.as_str())
                .is_some_and(|name| name == "done" || name == "error")
        })
        .count()
}

fn resolve_trace_id(explicit: String) -> String {
    if !explicit.trim().is_empty() {
        return explicit;
    }
    let millis = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|value| value.as_millis())
        .unwrap_or(0);
    format!("rex-cli-{millis}-{}", process::id())
}

impl CliCommand {
    fn output_format(&self) -> Option<CompleteOutputFormat> {
        match self {
            CliCommand::Complete { format, .. } => Some(*format),
            CliCommand::Status => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::{self, Write};

    use super::{
        classify_stream_terminal, format_ndjson_chunk_event, format_ndjson_done_event,
        format_ndjson_error_event, format_ndjson_plan_event, map_stream_status_error,
        ndjson_error_code, ndjson_terminal_event_count_for_tests, should_retry_stream_start,
        write_ndjson_line,
    };
    use crate::domain::StreamLifecycle;
    use crate::error::CliError;
    use rex_proto::rex::v1::StreamInferenceResponse;

    /// Records how many times `flush` runs after `write` traffic (for piped-NDJSON contract).
    #[derive(Default)]
    struct FlushObserver {
        buf: Vec<u8>,
        flush_count: usize,
    }

    impl Write for FlushObserver {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.buf.extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            self.flush_count += 1;
            Ok(())
        }
    }

    #[test]
    fn retry_policy_only_retries_daemon_unavailable_within_budget() {
        let unavailable = CliError::DaemonUnavailable {
            socket_path: "/tmp/rex.sock".to_string(),
        };
        assert!(should_retry_stream_start(&unavailable, 0));
        assert!(!should_retry_stream_start(
            &unavailable,
            crate::domain::STREAM_START_RETRY_ATTEMPTS
        ));
        let interrupted = CliError::StreamInterrupted;
        assert!(!should_retry_stream_start(&interrupted, 0));
    }

    #[test]
    fn ndjson_chunk_event_is_stable() {
        assert_eq!(
            format_ndjson_chunk_event(2, "hello"),
            r#"{"event":"chunk","index":2,"text":"hello"}"#
        );
    }

    #[test]
    fn ndjson_done_event_is_stable() {
        assert_eq!(format_ndjson_done_event(3), r#"{"event":"done","index":3}"#);
    }

    #[test]
    fn ndjson_plan_event_is_stable() {
        assert_eq!(
            format_ndjson_plan_event(3, "ready", "Planning tools slice", r#"{"steps":[]}"#),
            r#"{"detail":"{\"steps\":[]}","event":"plan","index":3,"phase":"ready","title":"Planning tools slice"}"#
        );
    }

    #[test]
    fn ndjson_error_event_is_stable() {
        let parsed: serde_json::Value =
            serde_json::from_str(&format_ndjson_error_event("boom".to_string(), "unknown"))
                .expect("ndjson error should be valid json");
        assert_eq!(
            parsed,
            serde_json::json!({
                "event": "error",
                "message": "boom",
                "code": "unknown"
            })
        );
    }

    #[test]
    fn ndjson_error_codes_are_stable() {
        assert_eq!(
            ndjson_error_code(&CliError::SidecarUnavailable {
                detail: "sidecar required but unavailable: test".to_string(),
            }),
            "sidecar_unavailable"
        );
        assert_eq!(
            ndjson_error_code(&CliError::InferenceConfig {
                detail: "inference runtime configuration failed".to_string(),
            }),
            "inference_config"
        );
        assert_eq!(
            ndjson_error_code(&CliError::StreamInterrupted),
            "stream_interrupted"
        );
        assert_eq!(
            ndjson_error_code(&CliError::StreamTimeout { seconds: 2 }),
            "stream_timeout"
        );
        assert_eq!(
            ndjson_error_code(&CliError::DaemonUnavailable {
                socket_path: "/tmp/rex.sock".to_string()
            }),
            "daemon_unavailable"
        );
    }

    #[test]
    fn stream_status_errors_map_to_typed_cli_errors() {
        let deadline = tonic::Status::deadline_exceeded("timeout");
        assert!(matches!(
            map_stream_status_error(deadline),
            CliError::StreamTimeout { .. }
        ));

        let unavailable = tonic::Status::unavailable("cursor failed");
        assert!(matches!(
            map_stream_status_error(unavailable),
            CliError::StreamInterrupted
        ));

        let internal = tonic::Status::internal("boom");
        assert!(matches!(
            map_stream_status_error(internal),
            CliError::Status(_)
        ));

        let sidecar = tonic::Status::failed_precondition("sidecar required but unavailable: down");
        assert!(matches!(
            map_stream_status_error(sidecar),
            CliError::SidecarUnavailable { .. }
        ));
    }

    #[test]
    fn stream_terminal_classification_is_deterministic() {
        let in_progress = StreamInferenceResponse {
            text: "x".to_string(),
            index: 0,
            done: false,
            ..Default::default()
        };
        assert_eq!(classify_stream_terminal(Some(&in_progress)), None);
        assert_eq!(
            classify_stream_terminal(None),
            Some(StreamLifecycle::Incomplete)
        );

        let terminal = StreamInferenceResponse {
            text: String::new(),
            index: 1,
            done: true,
            ..Default::default()
        };
        assert_eq!(
            classify_stream_terminal(Some(&terminal)),
            Some(StreamLifecycle::Completed)
        );
    }

    #[test]
    fn write_ndjson_line_flushes_after_each_line() {
        let payload = format_ndjson_chunk_event(0, "x");
        let mut observer = FlushObserver::default();
        write_ndjson_line(&mut observer, &payload).expect("write_ndjson_line");
        assert_eq!(
            observer.flush_count, 1,
            "each NDJSON line must flush for pipe consumers"
        );
        assert!(
            observer.buf.ends_with(b"\n"),
            "expected trailing newline, got {:?}",
            observer.buf
        );
    }

    #[test]
    fn ndjson_output_has_at_most_one_done_or_error_event_in_examples() {
        let chunk1 = r#"{"event":"chunk","index":0,"text":"a"}"#;
        let done1 = r#"{"event":"done","index":0}"#;
        assert_eq!(
            ndjson_terminal_event_count_for_tests(&format!("{chunk1}\n{done1}\n")),
            1
        );
        let err1 = r#"{"event":"error","message":"x","code":"y"}"#;
        assert_eq!(
            ndjson_terminal_event_count_for_tests(&format!("{err1}\n")),
            1
        );
        assert_eq!(
            ndjson_terminal_event_count_for_tests(&format!("{done1}\n{done1}\n")),
            2
        );
    }
}
