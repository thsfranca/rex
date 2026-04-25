use std::process::ExitCode;
use std::time::Duration;

use rex_proto::rex::v1::{GetSystemStatusRequest, StreamInferenceRequest};
use tokio::time::timeout;

use crate::command::{parse_command, print_usage, CliCommand};
use crate::domain::{REQUEST_TIMEOUT_SECONDS, STREAM_ITEM_TIMEOUT_SECONDS};
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
    let response = client.stream_inference(request).await?;
    let mut stream = response.into_inner();

    while let Some(chunk) = timeout(
        Duration::from_secs(STREAM_ITEM_TIMEOUT_SECONDS),
        stream.message(),
    )
    .await
    .map_err(|_| CliError::StreamTimeout {
        seconds: STREAM_ITEM_TIMEOUT_SECONDS,
    })?? {
        if !chunk.text.is_empty() {
            print!("{}", chunk.text);
        }
        if chunk.done {
            println!();
            break;
        }
    }
    Ok(())
}
