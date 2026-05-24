use std::path::PathBuf;

use crate::error::ConfigError;
use crate::model::RexConfig;

#[derive(Debug, Clone)]
pub struct LoadedConfig {
    pub rex_root: PathBuf,
    pub global_path: Option<PathBuf>,
    pub project_path: Option<PathBuf>,
    pub effective: RexConfig,
}

impl LoadedConfig {
    pub fn daemon_socket(&self) -> &str {
        &self.effective.daemon.socket
    }

    pub fn sidecar_harness_direct(&self) -> bool {
        self.effective.sidecars.harness.as_deref().is_some_and(|v| {
            matches!(
                v.trim().to_ascii_lowercase().as_str(),
                "direct" | "1" | "true"
            )
        })
    }

    pub fn sidecar_product_path_active(&self) -> bool {
        !self.sidecar_harness_direct() && self.active_sidecar().is_some_and(|e| e.enabled)
    }

    pub fn active_sidecar(&self) -> Option<&crate::model::SidecarEntry> {
        let name = self.effective.sidecars.active.as_str();
        self.effective
            .sidecars
            .list
            .iter()
            .find(|entry| entry.name == name)
    }

    pub fn cache_bypass(&self) -> bool {
        self.effective.cache.bypass.unwrap_or(false)
    }

    pub fn approvals_enabled(&self) -> bool {
        self.effective.agent.approvals_enabled.unwrap_or(false)
    }

    pub fn workspace_root(&self) -> PathBuf {
        let raw = self.effective.workspace.root.trim();
        if raw.is_empty() || raw == "." {
            env_current_dir()
        } else {
            PathBuf::from(raw)
        }
    }

    pub fn workspace_indexer_mode(&self) -> &str {
        self.effective.workspace.indexer.as_str()
    }

    pub fn token_budget(&self) -> (usize, usize) {
        (
            self.effective.context.max_prompt_tokens,
            self.effective.context.max_context_tokens,
        )
    }

    pub fn broker_shell_allowlist(&self) -> &[String] {
        &self.effective.broker.shell_allowlist
    }
}

fn env_current_dir() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

pub fn merge_config(base: &mut RexConfig, overlay: RexConfig) {
    if overlay.version != 0 {
        base.version = overlay.version;
    }
    merge_daemon(&mut base.daemon, overlay.daemon);
    merge_sidecars(&mut base.sidecars, overlay.sidecars);
    merge_inference(&mut base.inference, overlay.inference);
    merge_workspace(&mut base.workspace, overlay.workspace);
    merge_context(&mut base.context, overlay.context);
    merge_cache(&mut base.cache, overlay.cache);
    merge_broker(&mut base.broker, overlay.broker);
    merge_agent(&mut base.agent, overlay.agent);
}

fn merge_daemon(base: &mut crate::model::DaemonConfig, overlay: crate::model::DaemonConfig) {
    if !overlay.socket.is_empty() {
        base.socket = overlay.socket;
    }
}

fn merge_sidecars(base: &mut crate::model::SidecarsConfig, overlay: crate::model::SidecarsConfig) {
    if !overlay.active.is_empty() {
        base.active = overlay.active;
    }
    if overlay.required.is_some() {
        base.required = overlay.required;
    }
    if overlay.harness.is_some() {
        base.harness = overlay.harness;
    }
    if !overlay.list.is_empty() {
        base.list = overlay.list;
    }
}

fn merge_inference(
    base: &mut crate::model::InferenceConfig,
    overlay: crate::model::InferenceConfig,
) {
    if !overlay.runtime.is_empty() {
        base.runtime = overlay.runtime;
    }
    if !overlay.openai_compat.base_url.is_empty() {
        base.openai_compat.base_url = overlay.openai_compat.base_url;
    }
    if overlay.openai_compat.api_key.is_some() {
        base.openai_compat.api_key = overlay.openai_compat.api_key;
    }
    if !overlay.openai_compat.model.is_empty() {
        base.openai_compat.model = overlay.openai_compat.model;
    }
    if overlay.openai_compat.timeout_secs != 0 {
        base.openai_compat.timeout_secs = overlay.openai_compat.timeout_secs;
    }
    if !overlay.cursor_cli.path.is_empty() {
        base.cursor_cli.path = overlay.cursor_cli.path;
    }
    if overlay.cursor_cli.command.is_some() {
        base.cursor_cli.command = overlay.cursor_cli.command;
    }
    if overlay.cursor_cli.timeout_secs != 0 {
        base.cursor_cli.timeout_secs = overlay.cursor_cli.timeout_secs;
    }
}

fn merge_workspace(
    base: &mut crate::model::WorkspaceConfig,
    overlay: crate::model::WorkspaceConfig,
) {
    if !overlay.root.is_empty() {
        base.root = overlay.root;
    }
    if !overlay.indexer.is_empty() {
        base.indexer = overlay.indexer;
    }
}

fn merge_context(base: &mut crate::model::ContextConfig, overlay: crate::model::ContextConfig) {
    if overlay.max_prompt_tokens != 0 {
        base.max_prompt_tokens = overlay.max_prompt_tokens;
    }
    if overlay.max_context_tokens != 0 {
        base.max_context_tokens = overlay.max_context_tokens;
    }
}

fn merge_cache(base: &mut crate::model::CacheConfig, overlay: crate::model::CacheConfig) {
    if overlay.bypass.is_some() {
        base.bypass = overlay.bypass;
    }
}

fn merge_broker(base: &mut crate::model::BrokerConfig, overlay: crate::model::BrokerConfig) {
    if !overlay.shell_allowlist.is_empty() {
        base.shell_allowlist = overlay.shell_allowlist;
    }
}

fn merge_agent(base: &mut crate::model::AgentConfig, overlay: crate::model::AgentConfig) {
    if overlay.approvals_enabled.is_some() {
        base.approvals_enabled = overlay.approvals_enabled;
    }
    if overlay.max_tool_steps != 0 {
        base.max_tool_steps = overlay.max_tool_steps;
    }
}

#[allow(dead_code)]
pub fn validate_loaded(config: &LoadedConfig) -> Result<(), ConfigError> {
    config.effective.validate()
}
