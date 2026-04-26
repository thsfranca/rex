mod adapters;
mod domain;
mod l1_cache;
mod plugins;
mod runtime;
mod service;

#[tokio::main]
async fn main() -> Result<(), runtime::DaemonRuntimeError> {
    runtime::run_daemon().await
}
