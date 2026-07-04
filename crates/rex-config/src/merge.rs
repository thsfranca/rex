use std::path::PathBuf;

use crate::error::ConfigError;
use crate::model::RexConfig;

#[derive(Debug, Clone)]
pub struct LoadedConfig {
    pub rex_root: PathBuf,
    pub global_path: Option<PathBuf>,
    pub project_path: Option<PathBuf>,
    pub effective: RexConfig,
    pub resolved_daemon_socket: String,
    pub resolved_host_sidecar_socket: String,
}

impl LoadedConfig {
    pub fn from_effective(
        rex_root: PathBuf,
        global_path: Option<PathBuf>,
        project_path: Option<PathBuf>,
        effective: RexConfig,
    ) -> Result<Self, ConfigError> {
        let resolved = crate::sockets::resolve_sockets(&effective, &rex_root)?;
        Ok(Self {
            rex_root,
            global_path,
            project_path,
            effective,
            resolved_daemon_socket: resolved.daemon_socket,
            resolved_host_sidecar_socket: resolved.host_sidecar_socket,
        })
    }

    /// Unit-test helper when callers only need `effective` fields.
    pub fn for_test(rex_root: PathBuf, effective: RexConfig) -> Self {
        Self::from_effective(rex_root.clone(), None, None, effective.clone()).unwrap_or_else(|_| {
            Self {
                rex_root,
                global_path: None,
                project_path: None,
                effective: effective.clone(),
                resolved_daemon_socket: effective.daemon.resolved_socket().to_string(),
                resolved_host_sidecar_socket: effective
                    .sidecars
                    .list
                    .iter()
                    .find(|entry| entry.name == effective.host_sidecar_name())
                    .map(|entry| entry.socket.clone())
                    .unwrap_or_else(|| crate::model::DEFAULT_SIDECAR_SOCKET.to_string()),
            }
        })
    }

    pub fn daemon_socket(&self) -> &str {
        &self.resolved_daemon_socket
    }

    pub fn daemon_ready_timeout_secs(&self) -> u64 {
        let secs = self.effective.daemon.ready_timeout_secs;
        if secs == 0 {
            crate::model::DEFAULT_DAEMON_READY_TIMEOUT_SECS
        } else {
            secs
        }
    }

    pub fn daemon_idle_shutdown_secs(&self) -> u64 {
        self.effective.daemon.effective_idle_shutdown_secs()
    }

    pub fn daemon_log_path(&self) -> std::path::PathBuf {
        let raw = self.effective.daemon.log_path.trim();
        if raw.is_empty() {
            self.rex_root.join("daemon.log")
        } else {
            std::path::PathBuf::from(raw)
        }
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

    pub fn host_sidecar_name(&self) -> &str {
        self.effective.host_sidecar_name()
    }

    pub fn active_sidecar(&self) -> Option<&crate::model::SidecarEntry> {
        let name = self.host_sidecar_name();
        self.effective
            .sidecars
            .list
            .iter()
            .find(|entry| entry.name == name)
    }

    pub fn capability_sidecars(&self) -> &[crate::model::CapabilitySidecarEntry] {
        &self.effective.sidecars.capabilities
    }

    pub fn cache_bypass(&self) -> bool {
        self.effective.cache.bypass.unwrap_or(false)
    }

    pub fn approvals_enabled(&self) -> bool {
        self.effective.agent.approvals_enabled.unwrap_or(false)
    }

    pub fn tool_approvals_enabled(&self) -> bool {
        self.effective.agent.tool_approvals_enabled.unwrap_or(false)
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

    pub fn observability_enabled(&self) -> bool {
        crate::observability::observability_enabled(&self.effective.observability)
    }

    pub fn broker_max_tool_result_bytes(&self) -> usize {
        let bytes = self.effective.broker.max_tool_result_bytes;
        if bytes == 0 {
            crate::model::DEFAULT_MAX_TOOL_RESULT_BYTES as usize
        } else {
            bytes as usize
        }
    }

    pub fn stream_idle_timeout_secs(&self, mode: &str) -> u64 {
        let normalized = mode.trim().to_ascii_lowercase();
        if normalized == "agent" {
            self.effective.cli.stream_idle_timeout_secs_agent
        } else {
            self.effective.cli.stream_idle_timeout_secs_ask
        }
    }

    pub fn search_enabled(&self) -> bool {
        self.effective.search.enabled.unwrap_or(false)
    }

    pub fn search_provider(&self) -> &str {
        self.effective.search.provider.as_str()
    }

    pub fn search_max_results(&self) -> u32 {
        let n = self.effective.search.max_results;
        if n == 0 {
            5
        } else {
            n
        }
    }

    pub fn search_api_key_path(&self) -> &str {
        self.effective.search.api_key_path.as_str()
    }

    /// OpenAI-compat base URL after gateway injection rules.
    pub fn effective_openai_compat_base_url(&self) -> String {
        crate::gateway::resolve_effective_openai_compat_base_url(
            &self.effective.inference,
            &self.rex_root,
        )
    }

    /// Patch `effective.inference.openai_compat.base_url` for daemon HTTP adapter consumption.
    pub fn apply_effective_openai_compat_base_url(&mut self) {
        let url = self.effective_openai_compat_base_url();
        if !url.is_empty() {
            self.effective.inference.openai_compat.base_url = url;
        }
    }
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
    merge_cli(&mut base.cli, overlay.cli);
    merge_search(&mut base.search, overlay.search);
    merge_observability(&mut base.observability, overlay.observability);
}

fn merge_daemon(base: &mut crate::model::DaemonConfig, overlay: crate::model::DaemonConfig) {
    if let Some(socket) = overlay.socket {
        if !socket.is_empty() {
            base.socket = Some(socket);
        }
    }
    if overlay.socket_scope.is_some() {
        base.socket_scope = overlay.socket_scope;
    }
    if overlay.ready_timeout_secs != 0 {
        base.ready_timeout_secs = overlay.ready_timeout_secs;
    }
    if overlay.idle_shutdown_secs.is_some() {
        base.idle_shutdown_secs = overlay.idle_shutdown_secs;
    }
    if !overlay.log_path.is_empty() {
        base.log_path = overlay.log_path;
    }
}

fn merge_sidecars(base: &mut crate::model::SidecarsConfig, overlay: crate::model::SidecarsConfig) {
    if !overlay.active.is_empty() {
        base.active = overlay.active;
    }
    if overlay.host.is_some() {
        base.host = overlay.host;
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
    if !overlay.capabilities.is_empty() {
        base.capabilities = overlay.capabilities;
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
    if overlay.openai_compat.native_tools.is_some() {
        base.openai_compat.native_tools = overlay.openai_compat.native_tools;
    }
    for (k, v) in overlay.openai_compat.headers {
        base.openai_compat.headers.insert(k, v);
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
    merge_gateway(&mut base.gateway, overlay.gateway);
    merge_omlx(&mut base.omlx, overlay.omlx);
}

fn merge_omlx(base: &mut crate::model::OmlxConfig, overlay: crate::model::OmlxConfig) {
    if !overlay.mode.is_empty() {
        base.mode = overlay.mode;
    }
    if overlay.port != 0 {
        base.port = overlay.port;
    }
    if !overlay.command.is_empty() {
        base.command = overlay.command;
    }
    if !overlay.model_dir.is_empty() {
        base.model_dir = overlay.model_dir;
    }
    if !overlay.model.is_empty() {
        base.model = overlay.model;
    }
    if !overlay.health_path.is_empty() {
        base.health_path = overlay.health_path;
    }
    if overlay.discovery_on_ready.is_some() {
        base.discovery_on_ready = overlay.discovery_on_ready;
    }
    if overlay.startup_timeout_secs != 0 {
        base.startup_timeout_secs = overlay.startup_timeout_secs;
    }
    if overlay.required.is_some() {
        base.required = overlay.required;
    }
    if overlay.allow_url_override.is_some() {
        base.allow_url_override = overlay.allow_url_override;
    }
}

fn merge_gateway(base: &mut crate::model::GatewayConfig, overlay: crate::model::GatewayConfig) {
    if !overlay.mode.is_empty() {
        base.mode = overlay.mode;
    }
    if overlay.port != 0 {
        base.port = overlay.port;
    }
    if !overlay.config_path.is_empty() {
        base.config_path = overlay.config_path;
    }
    if !overlay.command.is_empty() {
        base.command = overlay.command;
    }
    if overlay.startup_timeout_secs != 0 {
        base.startup_timeout_secs = overlay.startup_timeout_secs;
    }
    if overlay.required.is_some() {
        base.required = overlay.required;
    }
    if overlay.allow_url_override.is_some() {
        base.allow_url_override = overlay.allow_url_override;
    }
    merge_gateway_ollama(&mut base.ollama, overlay.ollama);
}

fn merge_gateway_ollama(
    base: &mut crate::model::GatewayOllamaConfig,
    overlay: crate::model::GatewayOllamaConfig,
) {
    if overlay.enabled.is_some() {
        base.enabled = overlay.enabled;
    }
    if !overlay.api_base.is_empty() {
        base.api_base = overlay.api_base;
    }
    if overlay.discovery.is_some() {
        base.discovery = overlay.discovery;
    }
    if overlay.discovery_on_ready.is_some() {
        base.discovery_on_ready = overlay.discovery_on_ready;
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
    if overlay.allow_cwd_fallback.is_some() {
        base.allow_cwd_fallback = overlay.allow_cwd_fallback;
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
    if overlay.max_tool_result_bytes != 0 {
        base.max_tool_result_bytes = overlay.max_tool_result_bytes;
    }
}

fn merge_agent(base: &mut crate::model::AgentConfig, overlay: crate::model::AgentConfig) {
    if overlay.approvals_enabled.is_some() {
        base.approvals_enabled = overlay.approvals_enabled;
    }
    if overlay.tool_approvals_enabled.is_some() {
        base.tool_approvals_enabled = overlay.tool_approvals_enabled;
    }
    if overlay.max_tools_per_step != 0 {
        base.max_tools_per_step = overlay.max_tools_per_step;
    }
    if overlay.deterministic_init_enabled.is_some() {
        base.deterministic_init_enabled = overlay.deterministic_init_enabled;
    }
    if overlay.compaction_enabled.is_some() {
        base.compaction_enabled = overlay.compaction_enabled;
    }
    if overlay.compaction_suffix_fraction.is_some() {
        base.compaction_suffix_fraction = overlay.compaction_suffix_fraction;
    }
    if overlay.read_pruning_enabled.is_some() {
        base.read_pruning_enabled = overlay.read_pruning_enabled;
    }
}

fn merge_cli(base: &mut crate::model::CliConfig, overlay: crate::model::CliConfig) {
    if overlay.stream_idle_timeout_secs_agent != 0 {
        base.stream_idle_timeout_secs_agent = overlay.stream_idle_timeout_secs_agent;
    }
    if overlay.stream_idle_timeout_secs_ask != 0 {
        base.stream_idle_timeout_secs_ask = overlay.stream_idle_timeout_secs_ask;
    }
    merge_cli_ui(&mut base.ui, overlay.ui);
}

fn merge_cli_ui(base: &mut crate::model::CliUiConfig, overlay: crate::model::CliUiConfig) {
    if !overlay.enabled.is_empty() {
        base.enabled = overlay.enabled;
    }
    if !overlay.sync_output {
        base.sync_output = overlay.sync_output;
    }
}

fn merge_search(base: &mut crate::model::SearchConfig, overlay: crate::model::SearchConfig) {
    if overlay.enabled.is_some() {
        base.enabled = overlay.enabled;
    }
    if !overlay.provider.is_empty() {
        base.provider = overlay.provider;
    }
    if overlay.max_results != 0 {
        base.max_results = overlay.max_results;
    }
    if !overlay.api_key_path.is_empty() {
        base.api_key_path = overlay.api_key_path;
    }
}

fn merge_observability(
    base: &mut crate::model::ObservabilityConfig,
    overlay: crate::model::ObservabilityConfig,
) {
    if overlay.enabled.is_some() {
        base.enabled = overlay.enabled;
    }
    if !overlay.service_name.is_empty() {
        base.service_name = overlay.service_name;
    }
    if !overlay.otlp.endpoint.is_empty() {
        base.otlp.endpoint = overlay.otlp.endpoint;
    }
    if !overlay.otlp.protocol.is_empty() {
        base.otlp.protocol = overlay.otlp.protocol;
    }
}

#[allow(dead_code)]
pub fn validate_loaded(config: &LoadedConfig) -> Result<(), ConfigError> {
    config.effective.validate()
}
