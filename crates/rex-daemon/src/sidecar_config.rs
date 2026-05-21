use std::env;
use std::path::PathBuf;

pub const DEFAULT_SIDECAR_SOCKET: &str = "/tmp/rex-sidecar.sock";

pub struct SidecarConfig {
    pub enabled: bool,
    pub required: bool,
    pub binary: PathBuf,
    pub socket_path: String,
}

impl SidecarConfig {
    pub fn from_env() -> Self {
        let enabled = parse_enabled();
        let required = parse_required(enabled);
        let socket_path = env::var("REX_SIDECAR_SOCKET")
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| DEFAULT_SIDECAR_SOCKET.to_string());
        let binary = env::var("REX_SIDECAR_BINARY")
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(default_stub_binary);
        Self {
            enabled,
            required,
            binary,
            socket_path,
        }
    }
}

fn parse_enabled() -> bool {
    match env::var("REX_SIDECAR_ENABLED")
        .ok()
        .map(|v| v.trim().to_ascii_lowercase())
        .as_deref()
    {
        Some("0") | Some("false") | Some("no") | Some("off") => false,
        Some("1") | Some("true") | Some("yes") | Some("on") => true,
        _ => false,
    }
}

fn parse_required(enabled: bool) -> bool {
    if !enabled {
        return false;
    }
    match env::var("REX_SIDECAR_REQUIRED")
        .ok()
        .map(|v| v.trim().to_ascii_lowercase())
        .as_deref()
    {
        Some("0") | Some("false") | Some("no") => false,
        _ => enabled,
    }
}

fn default_stub_binary() -> PathBuf {
    if let Ok(path) = env::var("REX_SIDECAR_BINARY") {
        if !path.trim().is_empty() {
            return PathBuf::from(path);
        }
    }
    PathBuf::from("rex-sidecar-stub")
}

pub fn sidecar_product_path_active() -> bool {
    parse_harness_only().is_none() && SidecarConfig::from_env().enabled
}

pub fn parse_harness_only() -> Option<&'static str> {
    let raw = env::var("REX_SIDECAR_HARNESS").ok()?;
    let normalized = raw.trim().to_ascii_lowercase();
    if normalized == "direct" || normalized == "1" || normalized == "true" {
        Some("direct")
    } else {
        None
    }
}
