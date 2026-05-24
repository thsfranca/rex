use std::path::PathBuf;

use rex_config::{LoadedConfig, DEFAULT_SIDECAR_SOCKET};

pub struct SidecarConfig {
    pub enabled: bool,
    pub required: bool,
    pub binary: PathBuf,
    pub socket_path: String,
}

impl SidecarConfig {
    pub fn from_config(config: &LoadedConfig) -> Self {
        if config.sidecar_harness_direct() {
            return Self::disabled();
        }
        let Some(entry) = config.active_sidecar() else {
            return Self::disabled();
        };
        let required = config.effective.sidecars.required.unwrap_or(true) && entry.enabled;
        Self {
            enabled: entry.enabled,
            required,
            binary: PathBuf::from(&entry.binary),
            socket_path: entry.socket.clone(),
        }
    }

    fn disabled() -> Self {
        Self {
            enabled: false,
            required: false,
            binary: PathBuf::from("rex-sidecar-stub"),
            socket_path: DEFAULT_SIDECAR_SOCKET.to_string(),
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
