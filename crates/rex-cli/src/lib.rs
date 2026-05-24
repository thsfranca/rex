mod command;
mod domain;
mod error;
mod runtime;
mod transport;

use std::process::ExitCode;

pub async fn run_cli(args: impl Iterator<Item = String>) -> ExitCode {
    runtime::run_cli(args).await
}
