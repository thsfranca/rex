use std::path::Path;
use std::process::Command;

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
}
