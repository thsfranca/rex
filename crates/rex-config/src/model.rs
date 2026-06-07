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
                required: Some(true),
                harness: None,
                list: vec![SidecarEntry {
                    name: "stub".to_string(),
                    binary: "rex-sidecar-stub".to_string(),
                    enabled: true,
                    socket: DEFAULT_SIDECAR_SOCKET.to_string(),
                }],
            },
            inference: InferenceConfig {
                runtime: "mock".to_string(),
                openai_compat: OpenAiCompatConfig {
                    base_url: String::new(),
                    api_key: None,
                    model: "gpt-4o-mini".to_string(),
                    timeout_secs: 120,
                    native_tools: None,
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
            },
            observability: ObservabilityConfig::default(),
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
    #[serde(default)]
    pub required: Option<bool>,
    #[serde(default)]
    pub harness: Option<String>,
    #[serde(default)]
    pub list: Vec<SidecarEntry>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct SidecarEntry {
    pub name: String,
    pub binary: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub socket: String,
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

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct AgentConfig {
    #[serde(default)]
    pub approvals_enabled: Option<bool>,
    #[serde(default)]
    pub max_tool_steps: u32,
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
    pub read_api: ReadApiConfig,
    #[serde(default)]
    pub ui: UiConfig,
    #[serde(default)]
    pub otlp: OtlpConfig,
    #[serde(default)]
    pub store: StoreConfig,
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            enabled: None,
            service_name: default_obs_service_name(),
            custom_sidecar_metrics: true,
            read_api: ReadApiConfig::default(),
            ui: UiConfig::default(),
            otlp: OtlpConfig::default(),
            store: StoreConfig::default(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct ReadApiConfig {
    #[serde(default = "default_read_api_listen")]
    pub listen: String,
}

impl Default for ReadApiConfig {
    fn default() -> Self {
        Self {
            listen: default_read_api_listen(),
        }
    }
}

fn default_read_api_listen() -> String {
    "127.0.0.1:9470".to_string()
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct UiConfig {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub grafana: GrafanaUiConfig,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct GrafanaUiConfig {
    #[serde(default = "default_grafana_port")]
    pub port: u16,
}

impl Default for GrafanaUiConfig {
    fn default() -> Self {
        Self {
            port: default_grafana_port(),
        }
    }
}

fn default_grafana_port() -> u16 {
    3000
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct StoreConfig {
    #[serde(default = "default_store_engine")]
    pub engine: String,
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub format_version: u32,
}

impl Default for StoreConfig {
    fn default() -> Self {
        Self {
            engine: default_store_engine(),
            path: String::new(),
            format_version: 1,
        }
    }
}

fn default_store_engine() -> String {
    crate::observability::DEFAULT_STORE_ENGINE_SQLITE.to_string()
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
}
