mod command;
mod domain;
mod error;
mod runtime;
mod transport;

use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    runtime::run_cli(std::env::args().skip(1)).await
}
