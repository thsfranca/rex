use std::path::PathBuf;

use rex_config::{CapabilitySidecarEntry, LoadedConfig, DEFAULT_SIDECAR_SOCKET};

#[derive(Debug, Clone)]
pub struct SidecarProcessConfig {
    pub name: String,
    pub enabled: bool,
    pub required: bool,
    pub binary: PathBuf,
    pub socket_path: String,
    pub is_capability: bool,
}

#[derive(Debug, Clone)]
pub struct SidecarFleetConfig {
    pub host: SidecarProcessConfig,
    pub capabilities: Vec<SidecarProcessConfig>,
}

impl SidecarFleetConfig {
    pub fn from_config(config: &LoadedConfig) -> Self {
        if config.sidecar_harness_direct() {
            return Self::disabled();
        }
        let Some(entry) = config.active_sidecar() else {
            return Self::disabled();
        };
        let host_required = config.effective.sidecars.required.unwrap_or(true) && entry.enabled;
        let host = SidecarProcessConfig {
            name: entry.name.clone(),
            enabled: entry.enabled,
            required: host_required,
            binary: PathBuf::from(&entry.binary),
            socket_path: entry.socket.clone(),
            is_capability: false,
        };
        let capabilities = config
            .capability_sidecars()
            .iter()
            .map(capability_entry_to_process)
            .collect();
        Self { host, capabilities }
    }

    fn disabled() -> Self {
        Self {
            host: SidecarProcessConfig {
                name: "stub".to_string(),
                enabled: false,
                required: false,
                binary: PathBuf::from("rex-sidecar-stub"),
                socket_path: DEFAULT_SIDECAR_SOCKET.to_string(),
                is_capability: false,
            },
            capabilities: Vec::new(),
        }
    }
}

fn capability_entry_to_process(entry: &CapabilitySidecarEntry) -> SidecarProcessConfig {
    SidecarProcessConfig {
        name: entry.name.clone(),
        enabled: entry.enabled,
        required: entry.required.unwrap_or(false) && entry.enabled,
        binary: PathBuf::from(&entry.binary),
        socket_path: entry.socket.clone(),
        is_capability: true,
    }
}

/// Host sidecar config for callers that only need the active host slot.
#[derive(Debug, Clone)]
pub struct SidecarConfig {
    pub enabled: bool,
    pub required: bool,
    pub socket_path: String,
}

impl From<&SidecarProcessConfig> for SidecarConfig {
    fn from(process: &SidecarProcessConfig) -> Self {
        Self {
            enabled: process.enabled,
            required: process.required,
            socket_path: process.socket_path.clone(),
        }
    }
}

pub fn sidecar_harness_direct() -> bool {
    crate::settings::get().sidecar_harness_direct()
}

pub fn parse_harness_only() -> Option<&'static str> {
    if sidecar_harness_direct() {
        Some("direct")
    } else {
        None
    }
}
