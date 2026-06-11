use std::collections::BTreeMap;

use crate::error::ConfigError;

pub const DEFAULT_DAEMON_SOCKET: &str = "/tmp/rex.sock";
pub const DEFAULT_SIDECAR_SOCKET: &str = "/tmp/rex-sidecar.sock";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Default)]
pub struct RexConfig {
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub daemon: DaemonConfig,
    #[serde(default)]
    pub sidecars: SidecarsConfig,
    #[serde(default)]
    pub inference: InferenceConfig,
    #[serde(default)]
    pub workspace: WorkspaceConfig,
    #[serde(default)]
    pub context: ContextConfig,
    #[serde(default)]
    pub cache: CacheConfig,
    #[serde(default)]
    pub broker: BrokerConfig,
    #[serde(default)]
    pub agent: AgentConfig,
    #[serde(default)]
    pub cli: CliConfig,
    #[serde(default)]
    pub search: SearchConfig,
    #[serde(default)]
    pub observability: ObservabilityConfig,
}

impl RexConfig {
    pub fn defaults() -> Self {
        Self {
            version: 1,
            daemon: DaemonConfig {
                socket: DEFAULT_DAEMON_SOCKET.to_string(),
            },
            sidecars: SidecarsConfig {
                active: "stub".to_string(),
                host: None,
                required: Some(true),
                harness: None,
                list: vec![SidecarEntry {
                    name: "stub".to_string(),
                    binary: "rex-sidecar-stub".to_string(),
                    enabled: true,
                    socket: DEFAULT_SIDECAR_SOCKET.to_string(),
                }],
                capabilities: Vec::new(),
            },
            inference: InferenceConfig {
                runtime: "mock".to_string(),
                openai_compat: OpenAiCompatConfig {
                    base_url: String::new(),
                    api_key: None,
                    model: "gpt-4o-mini".to_string(),
                    timeout_secs: 120,
                    native_tools: None,
                    headers: BTreeMap::new(),
                },
                gateway: GatewayConfig::default(),
                cursor_cli: CursorCliConfig {
                    path: "cursor-agent".to_string(),
                    command: None,
                    timeout_secs: 20,
                },
            },
            workspace: WorkspaceConfig {
                root: String::new(),
                indexer: "workspace".to_string(),
                allow_cwd_fallback: None,
            },
            context: ContextConfig {
                max_prompt_tokens: 512,
                max_context_tokens: 192,
            },
            cache: CacheConfig {
                bypass: Some(false),
            },
            broker: BrokerConfig {
                shell_allowlist: vec!["echo".to_string(), "printf".to_string(), "true".to_string()],
                max_tool_result_bytes: default_max_tool_result_bytes(),
            },
            agent: AgentConfig {
                approvals_enabled: Some(false),
                max_tool_steps: 12,
                max_tool_steps_ask: default_max_tool_steps_ask(),
            },
            cli: CliConfig::default(),
            search: SearchConfig::default(),
            observability: ObservabilityConfig::default(),
        }
    }

    /// Template written by `rex config init` for operator installs.
    /// CI and tests continue to use [`Self::defaults`] (stub sidecar).
    pub fn operator_init_template() -> Self {
        Self {
            sidecars: SidecarsConfig {
                active: "agent".to_string(),
                host: None,
                required: Some(true),
                harness: None,
                list: vec![
                    SidecarEntry {
                        name: "stub".to_string(),
                        binary: "rex-sidecar-stub".to_string(),
                        enabled: false,
                        socket: DEFAULT_SIDECAR_SOCKET.to_string(),
                    },
                    SidecarEntry {
                        name: "agent".to_string(),
                        binary: "rex-agent".to_string(),
                        enabled: true,
                        socket: DEFAULT_SIDECAR_SOCKET.to_string(),
                    },
                ],
                capabilities: Vec::new(),
            },
            agent: AgentConfig {
                approvals_enabled: Some(true),
                max_tool_steps: 12,
                max_tool_steps_ask: default_max_tool_steps_ask(),
            },
            ..Self::defaults()
        }
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.version != 1 {
            return Err(ConfigError::Validation(format!(
                "unsupported config version {}",
                self.version
            )));
        }
        let runtime = self.inference.runtime.trim().to_ascii_lowercase();
        match runtime.as_str() {
            "mock" | "http-openai-compat" | "openai-compat" | "http" | "cursor-cli" | "cursor" => {}
            other => {
                return Err(ConfigError::Validation(format!(
                    "unknown inference.runtime: {other}"
                )));
            }
        }
        crate::gateway::validate_gateway(&self.inference.gateway)
            .map_err(ConfigError::Validation)?;
        if matches!(
            runtime.as_str(),
            "http-openai-compat" | "openai-compat" | "http"
        ) {
            let effective_url = crate::gateway::resolve_effective_openai_compat_base_url(
                &self.inference,
                &crate::paths::rex_root(),
            );
            if effective_url.trim().is_empty() {
                return Err(ConfigError::Validation(
                    "inference.openai_compat.base_url is required when runtime is http-openai-compat (or set inference.gateway.mode to managed)"
                        .to_string(),
                ));
            }
        }
        if self.sidecars.active.trim().is_empty() {
            return Err(ConfigError::Validation(
                "sidecars.active must not be empty".to_string(),
            ));
        }
        crate::observability::validate_observability(&self.observability)?;
        crate::openai_compat::validate_openai_compat(&self.inference.openai_compat)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct DaemonConfig {
    pub socket: String,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct SidecarsConfig {
    pub active: String,
    /// Host sidecar name; when empty, falls back to [`Self::active`].
    #[serde(default)]
    pub host: Option<String>,
    #[serde(default)]
    pub required: Option<bool>,
    #[serde(default)]
    pub harness: Option<String>,
    #[serde(default)]
    pub list: Vec<SidecarEntry>,
    #[serde(default)]
    pub capabilities: Vec<CapabilitySidecarEntry>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct SidecarEntry {
    pub name: String,
    pub binary: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub socket: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct CapabilitySidecarEntry {
    pub name: String,
    pub binary: String,
    #[serde(default)]
    pub enabled: bool,
    pub socket: String,
    #[serde(default)]
    pub provides: Vec<String>,
    #[serde(default)]
    pub required: Option<bool>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct InferenceConfig {
    pub runtime: String,
    #[serde(default)]
    pub openai_compat: OpenAiCompatConfig,
    #[serde(default)]
    pub gateway: GatewayConfig,
    #[serde(default)]
    pub cursor_cli: CursorCliConfig,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct GatewayConfig {
    #[serde(default = "default_gateway_mode")]
    pub mode: String,
    #[serde(default)]
    pub port: u16,
    #[serde(default)]
    pub config_path: String,
    #[serde(default = "default_gateway_command")]
    pub command: String,
    #[serde(default)]
    pub startup_timeout_secs: u64,
    #[serde(default)]
    pub required: Option<bool>,
    #[serde(default)]
    pub allow_url_override: Option<bool>,
    #[serde(default)]
    pub ollama: GatewayOllamaConfig,
}

fn default_gateway_mode() -> String {
    "disabled".to_string()
}

fn default_gateway_command() -> String {
    crate::gateway::DEFAULT_GATEWAY_COMMAND.to_string()
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            mode: default_gateway_mode(),
            port: crate::gateway::DEFAULT_GATEWAY_PORT,
            config_path: String::new(),
            command: default_gateway_command(),
            startup_timeout_secs: crate::gateway::DEFAULT_GATEWAY_STARTUP_TIMEOUT_SECS,
            required: None,
            allow_url_override: None,
            ollama: GatewayOllamaConfig::default(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct GatewayOllamaConfig {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub api_base: String,
    #[serde(default)]
    pub discovery: Option<bool>,
    #[serde(default)]
    pub discovery_on_ready: Option<bool>,
}

impl Default for GatewayOllamaConfig {
    fn default() -> Self {
        Self {
            enabled: None,
            api_base: "http://127.0.0.1:11434".to_string(),
            discovery: None,
            discovery_on_ready: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NativeToolsMode {
    #[default]
    Auto,
    True,
    False,
}

impl NativeToolsMode {
    pub fn from_config_str(raw: &str) -> Result<Self, String> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "" | "auto" => Ok(Self::Auto),
            "true" => Ok(Self::True),
            "false" => Ok(Self::False),
            other => Err(format!(
                "invalid inference.openai_compat.native_tools: {other} (expected auto, true, or false)"
            )),
        }
    }
}

impl serde::Serialize for NativeToolsMode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(match self {
            Self::Auto => "auto",
            Self::True => "true",
            Self::False => "false",
        })
    }
}

impl<'de> serde::Deserialize<'de> for NativeToolsMode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        Self::from_config_str(&raw).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct OpenAiCompatConfig {
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub timeout_secs: u64,
    #[serde(default)]
    pub native_tools: Option<NativeToolsMode>,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
}

impl OpenAiCompatConfig {
    pub fn effective_native_tools(&self) -> NativeToolsMode {
        self.native_tools.unwrap_or(NativeToolsMode::Auto)
    }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct CursorCliConfig {
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub timeout_secs: u64,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct WorkspaceConfig {
    #[serde(default)]
    pub root: String,
    #[serde(default)]
    pub indexer: String,
    #[serde(default)]
    pub allow_cwd_fallback: Option<bool>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct ContextConfig {
    #[serde(default)]
    pub max_prompt_tokens: usize,
    #[serde(default)]
    pub max_context_tokens: usize,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct CacheConfig {
    #[serde(default)]
    pub bypass: Option<bool>,
}

pub const DEFAULT_MAX_TOOL_RESULT_BYTES: u32 = 8192;

fn default_max_tool_result_bytes() -> u32 {
    DEFAULT_MAX_TOOL_RESULT_BYTES
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct BrokerConfig {
    #[serde(default)]
    pub shell_allowlist: Vec<String>,
    #[serde(default = "default_max_tool_result_bytes")]
    pub max_tool_result_bytes: u32,
}

impl Default for BrokerConfig {
    fn default() -> Self {
        Self {
            shell_allowlist: Vec::new(),
            max_tool_result_bytes: default_max_tool_result_bytes(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct AgentConfig {
    #[serde(default)]
    pub approvals_enabled: Option<bool>,
    #[serde(default = "default_max_tool_steps")]
    pub max_tool_steps: u32,
    #[serde(default = "default_max_tool_steps_ask")]
    pub max_tool_steps_ask: u32,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            approvals_enabled: None,
            max_tool_steps: default_max_tool_steps(),
            max_tool_steps_ask: default_max_tool_steps_ask(),
        }
    }
}

fn default_max_tool_steps() -> u32 {
    12
}

fn default_max_tool_steps_ask() -> u32 {
    5
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct CliConfig {
    #[serde(default = "default_stream_idle_timeout_agent")]
    pub stream_idle_timeout_secs_agent: u64,
    #[serde(default = "default_stream_idle_timeout_ask")]
    pub stream_idle_timeout_secs_ask: u64,
}

impl Default for CliConfig {
    fn default() -> Self {
        Self {
            stream_idle_timeout_secs_agent: default_stream_idle_timeout_agent(),
            stream_idle_timeout_secs_ask: default_stream_idle_timeout_ask(),
        }
    }
}

fn default_stream_idle_timeout_agent() -> u64 {
    120
}

fn default_stream_idle_timeout_ask() -> u64 {
    15
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct SearchConfig {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub provider: String,
    #[serde(default = "default_search_max_results")]
    pub max_results: u32,
    #[serde(default)]
    pub api_key_path: String,
}

fn default_search_max_results() -> u32 {
    5
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct ObservabilityConfig {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default = "default_obs_service_name")]
    pub service_name: String,
    #[serde(default = "default_true")]
    pub custom_sidecar_metrics: bool,
    #[serde(default)]
    pub otlp: OtlpConfig,
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            enabled: None,
            service_name: default_obs_service_name(),
            custom_sidecar_metrics: true,
            otlp: OtlpConfig::default(),
        }
    }
}

fn default_obs_service_name() -> String {
    crate::observability::DEFAULT_OBS_SERVICE_NAME.to_string()
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct OtlpConfig {
    #[serde(default)]
    pub endpoint: String,
    #[serde(default = "default_otlp_protocol")]
    pub protocol: String,
}

fn default_otlp_protocol() -> String {
    crate::observability::DEFAULT_OTLP_PROTOCOL.to_string()
}

#[cfg(test)]
mod native_tools_tests {
    use super::*;
    use crate::merge;

    #[test]
    fn native_tools_defaults_to_auto_when_omitted() {
        let cfg: OpenAiCompatConfig = serde_json::from_str("{}").expect("parse");
        assert_eq!(cfg.effective_native_tools(), NativeToolsMode::Auto);
    }

    #[test]
    fn native_tools_merge_overlay_false() {
        let mut base = RexConfig::defaults();
        let mut overlay = RexConfig::defaults();
        overlay.inference.openai_compat.native_tools = Some(NativeToolsMode::False);
        merge::merge_config(&mut base, overlay);
        assert_eq!(
            base.inference.openai_compat.effective_native_tools(),
            NativeToolsMode::False
        );
    }

    #[test]
    fn headers_merge_overlay_keys() {
        let mut base = RexConfig::defaults();
        base.inference
            .openai_compat
            .headers
            .insert("X-Base".to_string(), "a".to_string());
        let mut overlay = RexConfig::defaults();
        overlay
            .inference
            .openai_compat
            .headers
            .insert("X-Overlay".to_string(), "b".to_string());
        overlay
            .inference
            .openai_compat
            .headers
            .insert("X-Base".to_string(), "override".to_string());
        merge::merge_config(&mut base, overlay);
        assert_eq!(
            base.inference.openai_compat.headers.get("X-Base"),
            Some(&"override".to_string())
        );
        assert_eq!(
            base.inference.openai_compat.headers.get("X-Overlay"),
            Some(&"b".to_string())
        );
    }
}
