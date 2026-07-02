mod command;
mod daemon_lifecycle;
mod domain;
mod error;
mod runtime;
mod stream_render;
mod transport;
mod tui;

use std::process::ExitCode;

pub async fn run_cli(args: impl Iterator<Item = String>) -> ExitCode {
    runtime::run_cli(args).await
}
