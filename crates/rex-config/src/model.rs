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
                },
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
                max_tool_steps: 8,
            },
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
        if matches!(
            runtime.as_str(),
            "http-openai-compat" | "openai-compat" | "http"
        ) && self.inference.openai_compat.base_url.trim().is_empty()
        {
            return Err(ConfigError::Validation(
                "inference.openai_compat.base_url is required when runtime is http-openai-compat"
                    .to_string(),
            ));
        }
        if self.sidecars.active.trim().is_empty() {
            return Err(ConfigError::Validation(
                "sidecars.active must not be empty".to_string(),
            ));
        }
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
    pub cursor_cli: CursorCliConfig,
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
