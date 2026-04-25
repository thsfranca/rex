use std::process::ExitCode;
use std::time::Duration;

use rex_proto::rex::v1::{GetSystemStatusRequest, StreamInferenceRequest};
use tokio::time::timeout;
use tonic::Code;

use crate::command::{parse_command, print_usage, CliCommand};
use crate::domain::{StreamLifecycle, REQUEST_TIMEOUT_SECONDS, STREAM_ITEM_TIMEOUT_SECONDS};
use crate::error::CliError;
use crate::transport::connect_client;

pub async fn run_cli(args: impl Iterator<Item = String>) -> ExitCode {
    match parse_command(args) {
        Ok(command) => match execute(command).await {
            Ok(()) => ExitCode::SUCCESS,
            Err(err) => {
                eprintln!("Error: {err}");
                ExitCode::from(1)
            }
        },
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
        CliCommand::Complete { prompt } => run_complete(prompt).await,
    }
}

async fn run_status() -> Result<(), CliError> {
    let mut client = connect_client().await?;
    let mut request = tonic::Request::new(GetSystemStatusRequest {});
    request.set_timeout(Duration::from_secs(REQUEST_TIMEOUT_SECONDS));
    let response = client.get_system_status(request).await?;
    let status = response.into_inner();

    println!("daemon_version: {}", status.daemon_version);
    println!("uptime_seconds: {}", status.uptime_seconds);
    println!("active_model_id: {}", status.active_model_id);
    Ok(())
}

async fn run_complete(prompt: String) -> Result<(), CliError> {
    let mut client = connect_client().await?;
    let mut request = tonic::Request::new(StreamInferenceRequest { prompt });
    request.set_timeout(Duration::from_secs(REQUEST_TIMEOUT_SECONDS));
    let response = client
        .stream_inference(request)
        .await
        .map_err(map_status_error)?;
    let mut stream = response.into_inner();
    let lifecycle = consume_stream(&mut stream).await?;
    match lifecycle {
        StreamLifecycle::Completed => Ok(()),
        StreamLifecycle::Cancelled => Err(CliError::StreamIncomplete),
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

async fn consume_stream(
    stream: &mut tonic::Streaming<rex_proto::rex::v1::StreamInferenceResponse>,
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

        let maybe_chunk = next.map_err(|status| match status.code() {
            Code::Unavailable => map_status_error(status),
            _ => CliError::StreamInterrupted,
        })?;

        let Some(chunk) = maybe_chunk else {
            return Ok(StreamLifecycle::Cancelled);
        };

        if !chunk.text.is_empty() {
            print!("{}", chunk.text);
        }
        if chunk.done {
            println!();
            return Ok(StreamLifecycle::Completed);
        }
    }
}
