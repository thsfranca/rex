mod domain;
mod runtime;
mod service;

#[tokio::main]
async fn main() -> Result<(), runtime::DaemonRuntimeError> {
    runtime::run_daemon().await
}
