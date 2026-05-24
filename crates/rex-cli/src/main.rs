use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    eprintln!("rex-cli is deprecated; use `rex status` or `rex complete`");
    rex_cli::run_cli(std::env::args().skip(1)).await
}
