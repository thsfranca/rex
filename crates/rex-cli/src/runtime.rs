use std::process::ExitCode;
use std::time::Duration;

use rex_proto::rex::v1::GetSystemStatusRequest;

use crate::command::{parse_command, print_usage, CliCommand};
use crate::daemon_lifecycle::{ensure_daemon_ready, EnsureOptions};
use crate::domain::REQUEST_TIMEOUT_SECONDS;
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
        CliCommand::Status {
            no_daemon_autostart,
        } => {
            run_status(EnsureOptions {
                no_autostart: no_daemon_autostart,
            })
            .await
        }
        CliCommand::Tui {
            no_daemon_autostart,
        } => crate::tui::run_tui(no_daemon_autostart).await,
    }
}

async fn run_status(ensure_opts: EnsureOptions) -> Result<(), CliError> {
    ensure_daemon_ready(ensure_opts).await?;
    let mut client = connect_client(None).await?;
    let mut request = tonic::Request::new(GetSystemStatusRequest {});
    request.set_timeout(Duration::from_secs(REQUEST_TIMEOUT_SECONDS));
    let response = client.get_system_status(request).await?;
    let status = response.into_inner();

    println!("daemon_version: {}", status.daemon_version);
    println!("uptime_seconds: {}", status.uptime_seconds);
    println!("active_model_id: {}", status.active_model_id);
    println!("workspace_root: {}", status.workspace_root);
    println!("lifecycle_state: {}", status.lifecycle_state);
    println!("idle_seconds: {}", status.idle_seconds);
    println!("seconds_until_shutdown: {}", status.seconds_until_shutdown);
    Ok(())
}
