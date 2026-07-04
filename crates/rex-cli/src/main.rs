use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    eprintln!("rex-cli is deprecated; use `rex`");
    rex_cli::run_tui_main(rex_cli::TuiLaunch::New).await
}
