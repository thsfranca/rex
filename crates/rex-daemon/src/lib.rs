pub mod access_policy;
pub mod adapters;
pub mod approvals;
pub mod broker;
pub mod domain;
pub mod http_openai_compat;
pub mod l1_cache;
pub mod plugins;
pub mod policy;
pub mod routing;
pub mod runtime;
pub mod service;
pub mod sidecar_client;
pub mod sidecar_config;
pub mod supervisor;

pub use runtime::{run_daemon, run_daemon_on_socket, DaemonRuntimeError};

pub fn apply_operator_config(config: &rex_config::RexConfig) {
    rex_config::apply_to_env(config);
}
