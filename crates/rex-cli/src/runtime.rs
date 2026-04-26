use std::process::ExitCode;
use std::time::Duration;
use std::{env, process, time::SystemTime};

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
                        println!(
                            "{}",
                            format_ndjson_error_event(err.to_string(), ndjson_error_code(&err))
                        );
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
        CliCommand::Complete { prompt, format } => run_complete(prompt, format).await,
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

async fn run_complete(prompt: String, format: CompleteOutputFormat) -> Result<(), CliError> {
    let trace_id = resolve_trace_id();
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
        let mut request = tonic::Request::new(StreamInferenceRequest {
            prompt: prompt.clone(),
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
        let lifecycle = consume_stream(&mut stream, format).await?;
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
    match status.code() {
        Code::DeadlineExceeded => CliError::StreamTimeout {
            seconds: STREAM_ITEM_TIMEOUT_SECONDS,
        },
        Code::Unavailable => CliError::StreamInterrupted,
        _ => CliError::Status(status),
    }
}

async fn consume_stream(
    stream: &mut tonic::Streaming<rex_proto::rex::v1::StreamInferenceResponse>,
    format: CompleteOutputFormat,
) -> Result<StreamLifecycle, CliError> {
    loop {
        let next = timeout(
            Duration::from_secs(STREAM_ITEM_TIMEOUT_SECONDS),
            stream.message(),
        )
        .await
        .map_err(|_| CliError::StreamTimeout {
            seconds: STREAM_ITEM_TIMEOUT_SECONDS,
        })?;

        let maybe_chunk = next.map_err(map_stream_status_error)?;

        if matches!(
            classify_stream_terminal(maybe_chunk.as_ref()),
            Some(StreamLifecycle::Incomplete)
        ) {
            return Ok(StreamLifecycle::Incomplete);
        }
        let chunk = maybe_chunk.expect("incomplete stream should return early");

        if !chunk.text.is_empty() {
            match format {
                CompleteOutputFormat::Text => print!("{}", chunk.text),
                CompleteOutputFormat::Ndjson => {
                    println!("{}", format_ndjson_chunk_event(chunk.index, &chunk.text));
                }
            }
        }
        if chunk.done {
            match format {
                CompleteOutputFormat::Text => println!(),
                CompleteOutputFormat::Ndjson => {
                    println!("{}", format_ndjson_done_event(chunk.index));
                }
            }
            return Ok(StreamLifecycle::Completed);
        }
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

fn format_ndjson_chunk_event(index: u64, text: &str) -> String {
    json!({
        "event": "chunk",
        "index": index,
        "text": text
    })
    .to_string()
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
        CliError::DaemonConnect { .. } => "daemon_unavailable",
        CliError::Endpoint(_) | CliError::Status(_) => "unknown",
    }
}

fn resolve_trace_id() -> String {
    if let Ok(existing) = env::var("REX_TRACE_ID") {
        if !existing.trim().is_empty() {
            return existing;
        }
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
    use super::{
        classify_stream_terminal, format_ndjson_chunk_event, format_ndjson_done_event,
        format_ndjson_error_event, map_stream_status_error, ndjson_error_code,
        should_retry_stream_start,
    };
    use crate::domain::StreamLifecycle;
    use crate::error::CliError;
    use rex_proto::rex::v1::StreamInferenceResponse;

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
    }

    #[test]
    fn stream_terminal_classification_is_deterministic() {
        let in_progress = StreamInferenceResponse {
            text: "x".to_string(),
            index: 0,
            done: false,
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
        };
        assert_eq!(
            classify_stream_terminal(Some(&terminal)),
            Some(StreamLifecycle::Completed)
        );
    }
}
