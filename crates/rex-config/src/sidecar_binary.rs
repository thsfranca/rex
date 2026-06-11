use std::path::Path;
use std::process::Command;

/// Operator-facing install hint when a product sidecar binary is missing.
pub fn sidecar_install_hint(binary: &str) -> Option<&'static str> {
    let name = binary.rsplit('/').next().unwrap_or(binary);
    if name == "rex-agent" {
        Some("Install with: rex proto install && pip install -e sidecars/rex-agent")
    } else {
        None
    }
}

/// Whether a sidecar `binary` config value can be executed (absolute path exists or on PATH).
pub fn sidecar_binary_resolvable(binary: &str) -> bool {
    if binary.contains('/') || binary.contains('\\') {
        return Path::new(binary).exists();
    }
    Command::new("sh")
        .arg("-lc")
        .arg(format!("command -v {binary}"))
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_absolute_path_is_not_resolvable() {
        assert!(!sidecar_binary_resolvable(
            "/nonexistent/rex-agent-test-binary"
        ));
    }

    #[test]
    fn sh_is_resolvable_on_path() {
        assert!(sidecar_binary_resolvable("sh"));
    }

    #[test]
    fn rex_agent_has_install_hint() {
        assert!(sidecar_install_hint("rex-agent").is_some());
        assert!(sidecar_install_hint("/path/to/rex-agent").is_some());
        assert!(sidecar_install_hint("rex-sidecar-stub").is_none());
    }
}
