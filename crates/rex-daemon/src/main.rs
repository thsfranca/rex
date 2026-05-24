#[tokio::main]
async fn main() -> Result<(), rex_daemon::DaemonRuntimeError> {
    eprintln!("rex-daemon is deprecated; use `rex daemon`");
    rex_daemon::run_daemon().await
}
