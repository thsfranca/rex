//! Centralized access policy for brokered sidecar tool requests (RC-05 / R012 / R020).

use std::path::Path;

use crate::l1_cache::normalize_mode;

/// Stable deny codes for logs and broker responses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyDeny {
    pub code: &'static str,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccessDecision {
    Allow,
    Deny(PolicyDeny),
}

/// Broker capability evaluated before host execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrokerCapability {
    FsRead,
    FsList,
    FsWrite,
    ExecShell,
}

impl BrokerCapability {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::FsRead => "fs.read",
            Self::FsList => "fs.list",
            Self::FsWrite => "fs.write",
            Self::ExecShell => "exec.shell",
        }
    }
}

/// Evaluate a broker tool request (mode matrix + protected paths).
pub fn evaluate_broker(
    capability: BrokerCapability,
    mode: &str,
    relative_path: Option<&str>,
) -> AccessDecision {
    let mode = normalize_mode(mode);
    if !capability_allowed_in_mode(capability, &mode) {
        return AccessDecision::Deny(PolicyDeny {
            code: "mode_denied",
            message: format!("{} denied for mode {mode}", capability.as_str()),
        });
    }

    match capability {
        BrokerCapability::FsRead => {
            let path = relative_path.unwrap_or("");
            let trimmed = path.trim();
            if trimmed.is_empty() {
                return AccessDecision::Deny(PolicyDeny {
                    code: "path_empty",
                    message: "path must not be empty".to_string(),
                });
            }
            evaluate_path_protected(trimmed, "read")
        }
        BrokerCapability::FsList => {
            let path = relative_path.unwrap_or("").trim();
            if path.is_empty() {
                return AccessDecision::Allow;
            }
            evaluate_path_protected(path, "list")
        }
        BrokerCapability::FsWrite => {
            let path = relative_path.unwrap_or("");
            let trimmed = path.trim();
            if trimmed.is_empty() {
                return AccessDecision::Deny(PolicyDeny {
                    code: "path_empty",
                    message: "path must not be empty".to_string(),
                });
            }
            evaluate_path_protected(trimmed, "write")
        }
        BrokerCapability::ExecShell => AccessDecision::Allow,
    }
}

fn capability_allowed_in_mode(capability: BrokerCapability, mode: &str) -> bool {
    match capability {
        BrokerCapability::FsRead | BrokerCapability::FsList => true,
        BrokerCapability::FsWrite | BrokerCapability::ExecShell => mode == "agent",
    }
}

fn evaluate_path_protected(trimmed: &str, operation: &str) -> AccessDecision {
    if is_protected_path(trimmed) {
        return AccessDecision::Deny(PolicyDeny {
            code: "protected_path",
            message: format!("{operation} denied for protected path: {trimmed}"),
        });
    }
    AccessDecision::Allow
}

/// Evaluate `fs.read` before host execution.
pub fn evaluate_fs_read(relative_path: &str, mode: &str) -> AccessDecision {
    evaluate_broker(BrokerCapability::FsRead, mode, Some(relative_path))
}

/// Evaluate `fs.list` before host execution.
pub fn evaluate_fs_list(relative_path: &str, mode: &str) -> AccessDecision {
    evaluate_broker(BrokerCapability::FsList, mode, Some(relative_path))
}

/// Evaluate `fs.write` before host execution.
pub fn evaluate_fs_write(relative_path: &str, mode: &str) -> AccessDecision {
    evaluate_broker(BrokerCapability::FsWrite, mode, Some(relative_path))
}

/// Evaluate `exec.shell` before host execution.
pub fn evaluate_exec_shell(mode: &str) -> AccessDecision {
    evaluate_broker(BrokerCapability::ExecShell, mode, None)
}

fn is_protected_path(relative_path: &str) -> bool {
    let path = Path::new(relative_path.trim());
    let normalized = path
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join("/");
    let lower = normalized.to_ascii_lowercase();

    if lower.contains(".git/config") || lower.ends_with(".git/config") {
        return true;
    }
    if lower.contains(".env") {
        return true;
    }
    if lower.ends_with(".pem") || lower.ends_with(".key") {
        return true;
    }
    if lower.starts_with(".ssh/") || lower.contains("/.ssh/") {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_workspace_relative_path() {
        assert_eq!(
            evaluate_fs_read("src/main.rs", "agent"),
            AccessDecision::Allow
        );
    }

    #[test]
    fn denies_env_file() {
        match evaluate_fs_read(".env", "agent") {
            AccessDecision::Deny(d) => assert_eq!(d.code, "protected_path"),
            _ => panic!("expected deny"),
        }
    }

    #[test]
    fn denies_env_list() {
        match evaluate_fs_list(".env", "ask") {
            AccessDecision::Deny(d) => assert_eq!(d.code, "protected_path"),
            _ => panic!("expected deny"),
        }
    }

    #[test]
    fn allows_list_workspace_root() {
        assert_eq!(evaluate_fs_list("", "ask"), AccessDecision::Allow);
    }

    #[test]
    fn denies_git_config() {
        match evaluate_fs_read(".git/config", "agent") {
            AccessDecision::Deny(d) => assert_eq!(d.code, "protected_path"),
            _ => panic!("expected deny"),
        }
    }

    #[test]
    fn ask_mode_denies_write() {
        match evaluate_fs_write("out.txt", "ask") {
            AccessDecision::Deny(d) => assert_eq!(d.code, "mode_denied"),
            _ => panic!("expected deny"),
        }
    }

    #[test]
    fn plan_mode_denies_write() {
        match evaluate_fs_write("out.txt", "plan") {
            AccessDecision::Deny(d) => assert_eq!(d.code, "mode_denied"),
            _ => panic!("expected deny"),
        }
    }

    #[test]
    fn agent_mode_allows_write() {
        assert_eq!(evaluate_fs_write("out.txt", "agent"), AccessDecision::Allow);
    }

    #[test]
    fn agent_mode_denies_protected_write() {
        match evaluate_fs_write(".env", "agent") {
            AccessDecision::Deny(d) => assert_eq!(d.code, "protected_path"),
            _ => panic!("expected deny"),
        }
    }

    #[test]
    fn ask_mode_denies_exec() {
        match evaluate_exec_shell("ask") {
            AccessDecision::Deny(d) => assert_eq!(d.code, "mode_denied"),
            _ => panic!("expected deny"),
        }
    }

    #[test]
    fn agent_mode_allows_exec() {
        assert_eq!(evaluate_exec_shell("agent"), AccessDecision::Allow);
    }

    #[test]
    fn read_allowed_in_plan_mode() {
        assert_eq!(
            evaluate_fs_read("src/main.rs", "plan"),
            AccessDecision::Allow
        );
    }
}
