//! Minimal JSON configuration for Rex operator, daemon, and sidecars.

use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const DEFAULT_REX_HOME: &str = "~/.rex";
pub const DEFAULT_DAEMON_SOCKET: &str = "/tmp/rex.sock";
pub const DEFAULT_SIDECAR_SOCKET: &str = "/tmp/rex-sidecar.sock";
pub const DEFAULT_PROTO_GEN_ROOT: &str = "~/.rex/proto/gen";

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("io error at {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("invalid config: {0}")]
    Invalid(String),
    #[error("config version {found} is not supported (expected {expected})")]
    UnsupportedVersion { found: u64, expected: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RexConfig {
    pub version: u64,
    #[serde(default)]
    pub daemon: DaemonConfig,
    #[serde(default)]
    pub sidecars: SidecarsConfig,
    #[serde(default)]
    pub proto: ProtoConfig,
    #[serde(default)]
    pub inference: InferenceConfig,
    #[serde(default)]
    pub workspace: WorkspaceConfig,
    #[serde(default)]
    pub agent: AgentConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct DaemonConfig {
    #[serde(default = "default_daemon_socket")]
    pub socket: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SidecarsConfig {
    #[serde(default = "default_active_sidecar")]
    pub active: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default = "default_sidecar_list")]
    pub list: Vec<SidecarEntry>,
}

impl Default for SidecarsConfig {
    fn default() -> Self {
        Self {
            active: default_active_sidecar(),
            required: false,
            list: default_sidecar_list(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SidecarEntry {
    pub name: String,
    pub binary: String,
    #[serde(default = "default_sidecar_socket")]
    pub socket: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub runtime: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProtoConfig {
    #[serde(default = "default_proto_gen_root")]
    pub gen_root: String,
}

impl Default for ProtoConfig {
    fn default() -> Self {
        Self {
            gen_root: default_proto_gen_root(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct InferenceConfig {
    #[serde(default = "default_inference_runtime")]
    pub runtime: String,
    #[serde(default)]
    pub openai_compat: OpenAiCompatConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct OpenAiCompatConfig {
    #[serde(default = "default_openai_base_url")]
    pub base_url: String,
    #[serde(default = "default_openai_model")]
    pub model: String,
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct WorkspaceConfig {
    #[serde(default = "default_workspace_root")]
    pub root: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct AgentConfig {
    #[serde(default = "default_max_tool_steps")]
    pub max_tool_steps: u32,
}

fn default_true() -> bool {
    true
}

fn default_daemon_socket() -> String {
    DEFAULT_DAEMON_SOCKET.to_string()
}

fn default_sidecar_socket() -> String {
    DEFAULT_SIDECAR_SOCKET.to_string()
}

fn default_active_sidecar() -> String {
    "agent".to_string()
}

fn default_proto_gen_root() -> String {
    DEFAULT_PROTO_GEN_ROOT.to_string()
}

fn default_inference_runtime() -> String {
    "http-openai-compat".to_string()
}

fn default_openai_base_url() -> String {
    "http://127.0.0.1:11434/v1".to_string()
}

fn default_openai_model() -> String {
    "llama3.2".to_string()
}

fn default_timeout_secs() -> u64 {
    120
}

fn default_workspace_root() -> String {
    ".".to_string()
}

fn default_max_tool_steps() -> u32 {
    8
}

fn default_sidecar_list() -> Vec<SidecarEntry> {
    vec![
        SidecarEntry {
            name: "agent".to_string(),
            binary: "rex-agent".to_string(),
            socket: DEFAULT_SIDECAR_SOCKET.to_string(),
            enabled: true,
            runtime: "python".to_string(),
        },
        SidecarEntry {
            name: "stub".to_string(),
            binary: "rex-sidecar-stub".to_string(),
            socket: "/tmp/rex-sidecar-stub.sock".to_string(),
            enabled: false,
            runtime: "rust".to_string(),
        },
    ]
}

impl Default for RexConfig {
    fn default() -> Self {
        Self {
            version: 1,
            daemon: DaemonConfig::default(),
            sidecars: SidecarsConfig::default(),
            proto: ProtoConfig::default(),
            inference: InferenceConfig::default(),
            workspace: WorkspaceConfig::default(),
            agent: AgentConfig::default(),
        }
    }
}

pub fn rex_home() -> PathBuf {
    if let Ok(home) = env::var("REX_HOME") {
        let trimmed = home.trim();
        if !trimmed.is_empty() {
            return expand_tilde(trimmed);
        }
    }
    expand_tilde(DEFAULT_REX_HOME)
}

pub fn user_config_path() -> PathBuf {
    rex_home().join("config.json")
}

pub fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Ok(home) = env::var("HOME") {
            return PathBuf::from(home).join(rest);
        }
    }
    PathBuf::from(path)
}

pub fn load_merged() -> Result<RexConfig, ConfigError> {
    let mut config = RexConfig::default();
    if let Some(user) = read_optional_file(&user_config_path())? {
        merge_config(&mut config, user);
    }
    if let Some(project) = find_project_config()? {
        merge_config(&mut config, project);
    }
    apply_env_overrides(&mut config);
    validate(&config)?;
    Ok(config)
}

pub fn init_user_config() -> Result<PathBuf, ConfigError> {
    let path = user_config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| ConfigError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    if path.exists() {
        return Ok(path);
    }
    let config = RexConfig::default();
    write_json(&path, &config)?;
    Ok(path)
}

pub fn write_json(path: &Path, config: &RexConfig) -> Result<(), ConfigError> {
    let body = serde_json::to_string_pretty(config)
        .map_err(|err| ConfigError::Invalid(err.to_string()))?;
    fs::write(path, format!("{body}\n")).map_err(|source| ConfigError::Io {
        path: path.to_path_buf(),
        source,
    })
}

fn read_optional_file(path: &Path) -> Result<Option<RexConfig>, ConfigError> {
    if !path.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(path).map_err(|source| ConfigError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let parsed: RexConfig = serde_json::from_str(&raw)
        .map_err(|err| ConfigError::Invalid(format!("{}: {err}", path.display())))?;
    Ok(Some(parsed))
}

fn find_project_config() -> Result<Option<RexConfig>, ConfigError> {
    let mut dir = env::current_dir().map_err(|source| ConfigError::Io {
        path: PathBuf::from("."),
        source,
    })?;
    loop {
        let candidate = dir.join(".rex").join("config.json");
        if candidate.is_file() {
            return read_optional_file(&candidate);
        }
        if !dir.pop() {
            break;
        }
    }
    Ok(None)
}

fn merge_config(base: &mut RexConfig, overlay: RexConfig) {
    if overlay.version != 0 {
        base.version = overlay.version;
    }
    if !overlay.daemon.socket.is_empty() {
        base.daemon.socket = overlay.daemon.socket;
    }
    if !overlay.sidecars.active.is_empty() {
        base.sidecars.active = overlay.sidecars.active;
    }
    base.sidecars.required = overlay.sidecars.required || base.sidecars.required;
    if !overlay.sidecars.list.is_empty() {
        base.sidecars.list = overlay.sidecars.list;
    }
    if !overlay.proto.gen_root.is_empty() {
        base.proto.gen_root = overlay.proto.gen_root;
    }
    if !overlay.inference.runtime.is_empty() {
        base.inference.runtime = overlay.inference.runtime;
    }
    if !overlay.inference.openai_compat.base_url.is_empty() {
        base.inference.openai_compat.base_url = overlay.inference.openai_compat.base_url;
    }
    if !overlay.inference.openai_compat.model.is_empty() {
        base.inference.openai_compat.model = overlay.inference.openai_compat.model;
    }
    if overlay.inference.openai_compat.timeout_secs != 0 {
        base.inference.openai_compat.timeout_secs = overlay.inference.openai_compat.timeout_secs;
    }
    if overlay.inference.openai_compat.api_key.is_some() {
        base.inference.openai_compat.api_key = overlay.inference.openai_compat.api_key;
    }
    if !overlay.workspace.root.is_empty() {
        base.workspace.root = overlay.workspace.root;
    }
    if overlay.agent.max_tool_steps != 0 {
        base.agent.max_tool_steps = overlay.agent.max_tool_steps;
    }
}

fn apply_env_overrides(config: &mut RexConfig) {
    if let Ok(v) = env::var("REX_DAEMON_SOCKET") {
        let trimmed = v.trim();
        if !trimmed.is_empty() {
            config.daemon.socket = trimmed.to_string();
        }
    }
    if let Ok(v) = env::var("REX_SIDECAR_ENABLED") {
        let enabled = matches!(
            v.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        );
        config.sidecars.required = enabled;
        if enabled && config.sidecars.active.is_empty() {
            config.sidecars.active = "stub".to_string();
        }
    }
    if let Ok(v) = env::var("REX_SIDECAR_BINARY") {
        let trimmed = v.trim();
        if !trimmed.is_empty() {
            if let Some(entry) = config.active_sidecar_entry_mut() {
                entry.binary = trimmed.to_string();
            }
        }
    }
    if let Ok(v) = env::var("REX_SIDECAR_SOCKET") {
        let trimmed = v.trim();
        if !trimmed.is_empty() {
            if let Some(entry) = config.active_sidecar_entry_mut() {
                entry.socket = trimmed.to_string();
            }
        }
    }
    if let Ok(v) = env::var("REX_OPENAI_COMPAT_BASE_URL") {
        let trimmed = v.trim();
        if !trimmed.is_empty() {
            config.inference.openai_compat.base_url = trimmed.to_string();
        }
    }
    if let Ok(v) = env::var("REX_OPENAI_COMPAT_MODEL") {
        let trimmed = v.trim();
        if !trimmed.is_empty() {
            config.inference.openai_compat.model = trimmed.to_string();
        }
    }
    if let Ok(v) = env::var("REX_WORKSPACE_ROOT") {
        let trimmed = v.trim();
        if !trimmed.is_empty() {
            config.workspace.root = trimmed.to_string();
        }
    }
}

impl RexConfig {
    pub fn active_sidecar_entry(&self) -> Option<&SidecarEntry> {
        self.sidecars
            .list
            .iter()
            .find(|entry| entry.name == self.sidecars.active)
    }

    pub fn active_sidecar_entry_mut(&mut self) -> Option<&mut SidecarEntry> {
        let active = self.sidecars.active.clone();
        self.sidecars.list.iter_mut().find(|e| e.name == active)
    }

    pub fn proto_python_path(&self) -> PathBuf {
        expand_tilde(&self.proto.gen_root).join("python")
    }

    pub fn sidecar_enabled(&self) -> bool {
        self.active_sidecar_entry()
            .is_some_and(|entry| entry.enabled)
    }
}

pub fn validate(config: &RexConfig) -> Result<(), ConfigError> {
    if config.version != 1 {
        return Err(ConfigError::UnsupportedVersion {
            found: config.version,
            expected: 1,
        });
    }
    let mut names = HashMap::new();
    let mut sockets = HashMap::new();
    for entry in &config.sidecars.list {
        if entry.name.trim().is_empty() {
            return Err(ConfigError::Invalid(
                "sidecars.list entry missing name".to_string(),
            ));
        }
        if names
            .insert(entry.name.clone(), entry.binary.clone())
            .is_some()
        {
            return Err(ConfigError::Invalid(format!(
                "duplicate sidecar name: {}",
                entry.name
            )));
        }
        if sockets
            .insert(entry.socket.clone(), entry.name.clone())
            .is_some()
        {
            return Err(ConfigError::Invalid(format!(
                "duplicate sidecar socket: {}",
                entry.socket
            )));
        }
    }
    if config
        .sidecars
        .list
        .iter()
        .all(|entry| entry.name != config.sidecars.active)
    {
        return Err(ConfigError::Invalid(format!(
            "sidecars.active '{}' not found in sidecars.list",
            config.sidecars.active
        )));
    }
    Ok(())
}

/// Apply merged config to process environment (daemon/sidecar startup).
pub fn apply_to_env(config: &RexConfig) {
    use std::env;
    if !config.daemon.socket.is_empty() {
        env::set_var("REX_DAEMON_SOCKET", &config.daemon.socket);
    }
    if !config.workspace.root.is_empty() {
        env::set_var("REX_WORKSPACE_ROOT", &config.workspace.root);
    }
    if !config.inference.openai_compat.base_url.is_empty() {
        env::set_var(
            "REX_OPENAI_COMPAT_BASE_URL",
            &config.inference.openai_compat.base_url,
        );
    }
    if !config.inference.openai_compat.model.is_empty() {
        env::set_var(
            "REX_OPENAI_COMPAT_MODEL",
            &config.inference.openai_compat.model,
        );
    }
    if config.inference.openai_compat.timeout_secs > 0 {
        env::set_var(
            "REX_OPENAI_COMPAT_TIMEOUT_SECS",
            config.inference.openai_compat.timeout_secs.to_string(),
        );
    }
    if let Some(key) = &config.inference.openai_compat.api_key {
        if !key.trim().is_empty() {
            env::set_var("REX_OPENAI_COMPAT_API_KEY", key);
        }
    }
    if let Some(entry) = config.active_sidecar_entry() {
        if entry.enabled {
            env::set_var("REX_SIDECAR_ENABLED", "1");
            if config.sidecars.required {
                env::set_var("REX_SIDECAR_REQUIRED", "1");
            }
            env::set_var("REX_SIDECAR_BINARY", &entry.binary);
            env::set_var("REX_SIDECAR_SOCKET", &entry.socket);
        } else {
            env::remove_var("REX_SIDECAR_ENABLED");
        }
    }
    if config.agent.max_tool_steps > 0 {
        env::set_var(
            "REX_AGENT_MAX_TOOL_STEPS",
            config.agent.max_tool_steps.to_string(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_validates() {
        validate(&RexConfig::default()).expect("default config");
    }

    #[test]
    fn proto_python_path_under_gen_root() {
        let cfg = RexConfig::default();
        let python_path = cfg.proto_python_path();
        let path = python_path.to_string_lossy();
        assert!(
            path.ends_with("proto/gen/python") || path.contains(".rex/proto/gen/python"),
            "unexpected path: {path}"
        );
    }
}
