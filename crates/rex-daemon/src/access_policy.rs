//! Centralized access policy for brokered sidecar tool requests (RC-05 / R012).

use std::path::Path;

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

/// Evaluate `fs.read` before host execution.
pub fn evaluate_fs_read(relative_path: &str) -> AccessDecision {
    evaluate_fs_path(relative_path, "read")
}

/// Evaluate `fs.list` before host execution.
pub fn evaluate_fs_list(relative_path: &str) -> AccessDecision {
    let trimmed = relative_path.trim();
    if trimmed.is_empty() || trimmed == "." {
        return AccessDecision::Allow;
    }
    evaluate_fs_path(trimmed, "list")
}

fn evaluate_fs_path(relative_path: &str, _capability: &str) -> AccessDecision {
    let trimmed = relative_path.trim();
    if trimmed.is_empty() {
        return AccessDecision::Deny(PolicyDeny {
            code: "path_empty",
            message: "path must not be empty".to_string(),
        });
    }
    if is_protected_path(trimmed) {
        return AccessDecision::Deny(PolicyDeny {
            code: "protected_path",
            message: format!("read denied for protected path: {trimmed}"),
        });
    }
    AccessDecision::Allow
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
        assert_eq!(evaluate_fs_read("src/main.rs"), AccessDecision::Allow);
    }

    #[test]
    fn denies_env_file() {
        match evaluate_fs_read(".env") {
            AccessDecision::Deny(d) => assert_eq!(d.code, "protected_path"),
            _ => panic!("expected deny"),
        }
    }

    #[test]
    fn denies_git_config() {
        match evaluate_fs_read(".git/config") {
            AccessDecision::Deny(d) => assert_eq!(d.code, "protected_path"),
            _ => panic!("expected deny"),
        }
    }

    #[test]
    fn list_allows_workspace_root() {
        assert_eq!(evaluate_fs_list("."), AccessDecision::Allow);
        assert_eq!(evaluate_fs_list(""), AccessDecision::Allow);
    }

    #[test]
    fn list_denies_env_file() {
        match evaluate_fs_list(".env") {
            AccessDecision::Deny(d) => assert_eq!(d.code, "protected_path"),
            _ => panic!("expected deny"),
        }
    }
}
