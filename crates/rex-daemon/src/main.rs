mod access_policy;
mod adapters;
mod approvals;
mod broker;
mod domain;
mod http_openai_compat;
mod l1_cache;
mod plugins;
mod policy;
mod routing;
mod runtime;
mod service;
mod sidecar_client;
mod sidecar_config;
mod supervisor;

#[tokio::main]
async fn main() -> Result<(), runtime::DaemonRuntimeError> {
    runtime::run_daemon().await
}
