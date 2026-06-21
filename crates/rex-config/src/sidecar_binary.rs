use std::path::{Path, PathBuf};
use std::process::Command;

const REX_AGENT_MIN_PYTHON_MAJOR: u32 = 3;
const REX_AGENT_MIN_PYTHON_MINOR: u32 = 10;

/// Operator-facing install hint when a product sidecar binary is missing.
pub fn sidecar_install_hint(binary: &str) -> Option<&'static str> {
    let name = binary.rsplit('/').next().unwrap_or(binary);
    if name == "rex-agent" {
        Some("Install with: ./scripts/install-agent-sidecar.sh (Python >= 3.10, venv at $REX_ROOT/venv)")
    } else {
        None
    }
}

fn is_rex_agent_binary(binary: &str) -> bool {
    binary.rsplit('/').next().unwrap_or(binary) == "rex-agent"
}

fn rex_agent_venv_python(rex_root: &Path) -> PathBuf {
    rex_root.join("venv").join("bin").join("python")
}

/// Extra doctor checks when the active sidecar is rex-agent (venv + proto import smoke).
pub fn rex_agent_doctor_checks(rex_root: &Path, proto_gen: &Path) -> Result<(), String> {
    let venv_python = rex_agent_venv_python(rex_root);
    if !venv_python.is_file() {
        return Err(format!(
            "rex-agent venv python missing at {} (run ./scripts/install-agent-sidecar.sh)",
            venv_python.display()
        ));
    }
    let version_out = Command::new(&venv_python)
        .args([
            "-c",
            "import sys; print(f\"{sys.version_info.major}.{sys.version_info.minor}\")",
        ])
        .output()
        .map_err(|err| format!("rex-agent venv python failed to run: {err}"))?;
    if !version_out.status.success() {
        return Err("rex-agent venv python is not runnable".to_string());
    }
    let version = String::from_utf8_lossy(&version_out.stdout)
        .trim()
        .to_string();
    let (major, minor) = parse_python_version(&version)
        .ok_or_else(|| format!("could not parse rex-agent venv python version: {version}"))?;
    if major < REX_AGENT_MIN_PYTHON_MAJOR
        || (major == REX_AGENT_MIN_PYTHON_MAJOR && minor < REX_AGENT_MIN_PYTHON_MINOR)
    {
        return Err(format!(
            "rex-agent venv uses Python {version} (requires >= {}.{}) — re-run ./scripts/install-agent-sidecar.sh",
            REX_AGENT_MIN_PYTHON_MAJOR, REX_AGENT_MIN_PYTHON_MINOR
        ));
    }
    let proto_gen_str = proto_gen.display().to_string();
    let import_check = Command::new(&venv_python)
        .env("PYTHONPATH", &proto_gen_str)
        .args(["-c", "from rex.v1 import rex_pb2"])
        .status()
        .map_err(|err| format!("rex-agent proto import smoke failed to run: {err}"))?;
    if !import_check.success() {
        return Err(format!(
            "rex-agent proto import failed (PYTHONPATH={proto_gen_str}) — run rex proto install && ./scripts/install-agent-sidecar.sh"
        ));
    }
    Ok(())
}

fn parse_python_version(raw: &str) -> Option<(u32, u32)> {
    let mut parts = raw.split('.');
    let major: u32 = parts.next()?.parse().ok()?;
    let minor: u32 = parts.next()?.parse().ok()?;
    Some((major, minor))
}

/// Whether doctor should run rex-agent-specific venv/import checks for this binary name.
pub fn rex_agent_doctor_applies(binary: &str) -> bool {
    is_rex_agent_binary(binary)
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
        let hint = sidecar_install_hint("rex-agent").unwrap();
        assert!(hint.contains("install-agent-sidecar"));
    }

    #[test]
    fn rex_agent_doctor_applies_to_basename_only() {
        assert!(rex_agent_doctor_applies("rex-agent"));
        assert!(rex_agent_doctor_applies("/opt/bin/rex-agent"));
        assert!(!rex_agent_doctor_applies("rex-sidecar-stub"));
    }

    #[test]
    fn parse_python_version_accepts_major_minor() {
        assert_eq!(parse_python_version("3.12"), Some((3, 12)));
        assert_eq!(parse_python_version("3.9"), Some((3, 9)));
        assert_eq!(parse_python_version("bad"), None);
    }
}
