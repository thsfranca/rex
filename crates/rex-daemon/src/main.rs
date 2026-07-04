#[tokio::main]
async fn main() -> Result<(), rex_daemon::DaemonRuntimeError> {
    eprintln!("rex-daemon is deprecated; run `rex` (daemon auto-starts)");
    rex_daemon::run_daemon().await
}
