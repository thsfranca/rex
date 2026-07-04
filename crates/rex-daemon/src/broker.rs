//! Host capability broker (MVP: workspace `fs.read`, `fs.list`, `fs.write`, `exec.shell`).

use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::access_policy::{AccessDecision, PolicyDeny};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum BrokerError {
    #[error("workspace root not configured")]
    NoWorkspaceRoot,
    #[error("access policy denied ({code}): {message}")]
    PolicyDenied { code: String, message: String },
    #[error("path escapes workspace: {0}")]
    EscapesWorkspace(String),
    #[error("path not found: {0}")]
    NotFound(String),
    #[error("read failed: {0}")]
    Io(String),
    #[error("write too large: {0} bytes (max {1})")]
    WriteTooLarge(usize, usize),
    #[error("plan_too_large: {0} bytes (max {1})")]
    PlanTooLarge(usize, usize),
    #[error("command not allowlisted: {0}")]
    CommandNotAllowed(String),
    #[error("shell execution failed: {0}")]
    ShellFailed(String),
}

const MAX_WRITE_BYTES: usize = 65_536;
const MAX_LIST_DIR_ENTRIES: usize = 256;
const TRUNCATION_MARKER: &str = " [rex: tool output truncated]";
const TOOL_RESULT_CLOSE: &str = "\n<<END>>";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrokerDirEntry {
    pub name: String,
    pub is_dir: bool,
}

impl From<PolicyDeny> for BrokerError {
    fn from(deny: PolicyDeny) -> Self {
        Self::PolicyDenied {
            code: deny.code.to_string(),
            message: deny.message,
        }
    }
}

pub fn workspace_root_from_config() -> Result<PathBuf, BrokerError> {
    crate::settings::get()
        .resolve_workspace_root()
        .map_err(|_| BrokerError::NoWorkspaceRoot)
}

fn max_tool_result_bytes() -> usize {
    crate::settings::get().broker_max_tool_result_bytes()
}

fn tool_result_open_delimiter(tool: &str) -> String {
    format!("<<TOOL_RESULT:{tool}>>\n")
}

fn delimited_overhead_bytes(tool: &str) -> usize {
    tool_result_open_delimiter(tool).len() + TOOL_RESULT_CLOSE.len()
}

/// Truncate `text` to fit within `max_body_bytes` (UTF-8), ending at a line boundary.
/// Appends [`TRUNCATION_MARKER`] when truncated.
pub fn truncate_tool_result_at_line_boundary(text: &str, max_body_bytes: usize) -> String {
    if text.len() <= max_body_bytes {
        return text.to_string();
    }
    let marker_len = TRUNCATION_MARKER.len();
    let budget = max_body_bytes.saturating_sub(marker_len);
    if budget == 0 {
        return TRUNCATION_MARKER.to_string();
    }
    let bytes = text.as_bytes();
    let mut end = budget.min(bytes.len());
    while end > 0 && std::str::from_utf8(&bytes[..end]).is_err() {
        end -= 1;
    }
    if end == 0 {
        return TRUNCATION_MARKER.to_string();
    }
    let prefix = std::str::from_utf8(&bytes[..end]).expect("utf8 boundary");
    let body = match prefix.rfind('\n') {
        Some(idx) if idx > 0 => &prefix[..idx],
        Some(_) => "",
        None => "",
    };
    if body.is_empty() {
        TRUNCATION_MARKER.to_string()
    } else {
        format!("{body}{TRUNCATION_MARKER}")
    }
}

/// Wrap `body` as `<<TOOL_RESULT:tool>>` … `<<END>>`, truncating at line boundaries when needed.
pub fn format_delimited_tool_result(tool: &str, body: &str) -> String {
    let max_total = max_tool_result_bytes();
    let overhead = delimited_overhead_bytes(tool);
    let mut max_body = max_total.saturating_sub(overhead);
    loop {
        let truncated = truncate_tool_result_at_line_boundary(body, max_body);
        let formatted = format!(
            "{open}{truncated}{TOOL_RESULT_CLOSE}",
            open = tool_result_open_delimiter(tool),
        );
        if formatted.len() <= max_total || max_body == 0 {
            return formatted;
        }
        max_body = max_body.saturating_sub(1);
    }
}

fn format_shell_result_body(stdout: &str, stderr: &str) -> String {
    if stdout.is_empty() && stderr.is_empty() {
        return String::new();
    }
    if stderr.is_empty() {
        return stdout.to_string();
    }
    if stdout.is_empty() {
        return format!("stderr:\n{stderr}");
    }
    format!("stdout:\n{stdout}\n---\nstderr:\n{stderr}")
}

pub fn broker_read_file(relative_path: &str, mode: &str) -> Result<String, BrokerError> {
    match crate::access_policy::evaluate_fs_read(relative_path, mode) {
        AccessDecision::Allow => {}
        AccessDecision::Deny(deny) => return Err(deny.into()),
    }
    let root = workspace_root_from_config()?;
    let resolved = resolve_under_workspace(&root, relative_path)?;
    let content = std::fs::read_to_string(&resolved).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            BrokerError::NotFound(relative_path.to_string())
        } else {
            BrokerError::Io(e.to_string())
        }
    })?;
    Ok(format_delimited_tool_result("fs.read", &content))
}

pub fn broker_list_dir(
    relative_path: &str,
    mode: &str,
) -> Result<Vec<BrokerDirEntry>, BrokerError> {
    match crate::access_policy::evaluate_fs_list(relative_path, mode) {
        AccessDecision::Allow => {}
        AccessDecision::Deny(deny) => return Err(deny.into()),
    }
    let root = workspace_root_from_config()?;
    let resolved = resolve_list_dir(&root, relative_path)?;
    if !resolved.is_dir() {
        return Err(BrokerError::Io(format!(
            "not a directory: {}",
            relative_path.trim()
        )));
    }
    let mut entries = Vec::new();
    for entry in fs::read_dir(&resolved).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            BrokerError::NotFound(relative_path.trim().to_string())
        } else {
            BrokerError::Io(e.to_string())
        }
    })? {
        let entry = entry.map_err(|e| BrokerError::Io(e.to_string()))?;
        let file_type = entry
            .file_type()
            .map_err(|e| BrokerError::Io(e.to_string()))?;
        let name = entry.file_name().to_string_lossy().to_string();
        entries.push(BrokerDirEntry {
            name,
            is_dir: file_type.is_dir(),
        });
        if entries.len() >= MAX_LIST_DIR_ENTRIES {
            break;
        }
    }
    entries.sort_by(|a, b| {
        a.name
            .to_ascii_lowercase()
            .cmp(&b.name.to_ascii_lowercase())
    });
    Ok(entries)
}

pub fn broker_write_file(
    relative_path: &str,
    content: &str,
    mode: &str,
) -> Result<(), BrokerError> {
    match crate::access_policy::evaluate_fs_write(relative_path, mode) {
        AccessDecision::Allow => {}
        AccessDecision::Deny(deny) => return Err(deny.into()),
    }
    if content.len() > MAX_WRITE_BYTES {
        return Err(BrokerError::WriteTooLarge(content.len(), MAX_WRITE_BYTES));
    }
    let root = workspace_root_from_config()?;
    let resolved = resolve_under_workspace_for_write(&root, relative_path)?;
    if let Some(parent) = resolved.parent() {
        fs::create_dir_all(parent).map_err(|e| BrokerError::Io(e.to_string()))?;
    }
    fs::write(&resolved, content).map_err(|e| BrokerError::Io(e.to_string()))
}

pub fn broker_save_plan(relative_path: &str, content: &str, mode: &str) -> Result<(), BrokerError> {
    match crate::access_policy::evaluate_plan_save(relative_path, mode) {
        AccessDecision::Allow => {}
        AccessDecision::Deny(deny) => return Err(deny.into()),
    }
    if content.len() > MAX_WRITE_BYTES {
        return Err(BrokerError::PlanTooLarge(content.len(), MAX_WRITE_BYTES));
    }
    let path = crate::access_policy::normalize_plan_save_path(relative_path);
    let root = workspace_root_from_config()?;
    let resolved = resolve_under_workspace_for_write(&root, &path)?;
    if let Some(parent) = resolved.parent() {
        fs::create_dir_all(parent).map_err(|e| BrokerError::Io(e.to_string()))?;
    }
    fs::write(&resolved, content).map_err(|e| BrokerError::Io(e.to_string()))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellResult {
    pub stdout: String,
    pub stderr: String,
}

pub fn broker_exec_shell(command: &str, mode: &str) -> Result<ShellResult, BrokerError> {
    match crate::access_policy::evaluate_exec_shell(mode) {
        AccessDecision::Allow => {}
        AccessDecision::Deny(deny) => return Err(deny.into()),
    }
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return Err(BrokerError::CommandNotAllowed("(empty)".to_string()));
    }
    let mut parts = trimmed.split_whitespace();
    let program = parts
        .next()
        .ok_or_else(|| BrokerError::CommandNotAllowed("(empty)".to_string()))?;
    if !shell_allowlist().contains(&program.to_ascii_lowercase()) {
        return Err(BrokerError::CommandNotAllowed(program.to_string()));
    }
    let root = workspace_root_from_config()?;
    let output = std::process::Command::new(program)
        .args(parts)
        .current_dir(&root)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .map_err(|e| BrokerError::ShellFailed(e.to_string()))?;
    if !output.status.success() {
        return Err(BrokerError::ShellFailed(format!(
            "exit={:?}",
            output.status.code()
        )));
    }
    let raw_stdout = String::from_utf8_lossy(&output.stdout);
    let raw_stderr = String::from_utf8_lossy(&output.stderr);
    let body = format_shell_result_body(&raw_stdout, &raw_stderr);
    Ok(ShellResult {
        stdout: format_delimited_tool_result("exec.shell", &body),
        stderr: String::new(),
    })
}

fn shell_allowlist() -> Vec<String> {
    crate::settings::get()
        .broker_shell_allowlist()
        .iter()
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty())
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebSearchHit {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

pub fn broker_web_search(query: &str, mode: &str) -> Result<Vec<WebSearchHit>, BrokerError> {
    match crate::access_policy::evaluate_broker(
        crate::access_policy::BrokerCapability::WebSearch,
        mode,
        None,
    ) {
        AccessDecision::Allow => {}
        AccessDecision::Deny(deny) => return Err(deny.into()),
    }
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Err(BrokerError::Io("query must not be empty".to_string()));
    }
    let settings = crate::settings::get();
    let provider = settings.search_provider();
    let max_results = settings.search_max_results();
    if provider == "mock" {
        return Ok(vec![WebSearchHit {
            title: format!("Mock result for \"{trimmed}\""),
            url: "https://example.com/mock".to_string(),
            snippet: format!("provider=mock max_results={max_results}"),
        }]);
    }
    Err(BrokerError::Io(
        "web search provider not configured; set search.provider to mock for local demos"
            .to_string(),
    ))
}

fn resolve_under_workspace_for_write(
    root: &Path,
    relative_path: &str,
) -> Result<PathBuf, BrokerError> {
    let rel = Path::new(relative_path.trim());
    if rel.is_absolute() {
        return Err(BrokerError::EscapesWorkspace(relative_path.to_string()));
    }
    let mut candidate = root.to_path_buf();
    for component in rel.components() {
        match component {
            Component::Normal(part) => candidate.push(part),
            Component::CurDir => {}
            Component::ParentDir => {
                return Err(BrokerError::EscapesWorkspace(relative_path.to_string()));
            }
            _ => return Err(BrokerError::EscapesWorkspace(relative_path.to_string())),
        }
    }
    let canonical_root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let parent = candidate
        .parent()
        .map(|p| p.canonicalize().unwrap_or_else(|_| p.to_path_buf()))
        .unwrap_or_else(|| canonical_root.clone());
    if !parent.starts_with(&canonical_root) {
        return Err(BrokerError::EscapesWorkspace(relative_path.to_string()));
    }
    Ok(candidate)
}

fn resolve_under_workspace(root: &Path, relative_path: &str) -> Result<PathBuf, BrokerError> {
    let rel = Path::new(relative_path.trim());
    if rel.is_absolute() {
        return Err(BrokerError::EscapesWorkspace(relative_path.to_string()));
    }
    let mut candidate = root.to_path_buf();
    for component in rel.components() {
        match component {
            Component::Normal(part) => candidate.push(part),
            Component::CurDir => {}
            Component::ParentDir => {
                return Err(BrokerError::EscapesWorkspace(relative_path.to_string()));
            }
            _ => return Err(BrokerError::EscapesWorkspace(relative_path.to_string())),
        }
    }
    let canonical_root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let canonical = candidate
        .canonicalize()
        .map_err(|_| BrokerError::NotFound(relative_path.to_string()))?;
    if !canonical.starts_with(&canonical_root) {
        return Err(BrokerError::EscapesWorkspace(relative_path.to_string()));
    }
    Ok(canonical)
}

fn resolve_list_dir(root: &Path, relative_path: &str) -> Result<PathBuf, BrokerError> {
    let trimmed = relative_path.trim();
    if trimmed.is_empty() {
        return Ok(root.canonicalize().unwrap_or_else(|_| root.to_path_buf()));
    }
    resolve_under_workspace(root, trimmed)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceSearchKind {
    Basename,
    Content,
}

const WORKSPACE_SEARCH_MAX_DEPTH: usize = 12;
const WORKSPACE_SEARCH_MAX_DIRS: usize = 96;
const WORKSPACE_SEARCH_MAX_MATCHES: usize = 8;
const WORKSPACE_SEARCH_SKIP_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    "__pycache__",
    ".venv",
    "venv",
    "dist",
    "build",
];

pub fn broker_workspace_search(
    query: &str,
    kind: WorkspaceSearchKind,
    mode: &str,
    max_results: usize,
) -> Result<String, BrokerError> {
    match crate::access_policy::evaluate_workspace_search(mode) {
        AccessDecision::Allow => {}
        AccessDecision::Deny(deny) => return Err(deny.into()),
    }
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Err(BrokerError::Io("query must not be empty".to_string()));
    }
    let limit = max_results.clamp(1, WORKSPACE_SEARCH_MAX_MATCHES);
    let root = workspace_root_from_config()?;
    let matches = match kind {
        WorkspaceSearchKind::Basename => search_paths_by_basename(&root, trimmed, limit)?,
        WorkspaceSearchKind::Content => search_paths_by_content(&root, trimmed, limit)?,
    };
    let body = if matches.is_empty() {
        "no matches".to_string()
    } else {
        matches.join("\n")
    };
    Ok(format_delimited_tool_result("workspace.search", &body))
}

fn search_paths_by_basename(
    root: &Path,
    query: &str,
    limit: usize,
) -> Result<Vec<String>, BrokerError> {
    let needle = query.to_ascii_lowercase();
    let mut matches = Vec::new();
    let mut queue = vec![(String::new(), 0usize)];
    let mut visited = 0usize;
    while let Some((dir_path, depth)) = queue.pop() {
        if visited >= WORKSPACE_SEARCH_MAX_DIRS || matches.len() >= limit {
            break;
        }
        visited += 1;
        let resolved = if dir_path.is_empty() {
            root.to_path_buf()
        } else {
            resolve_under_workspace(root, &dir_path)?
        };
        for entry in fs::read_dir(&resolved).map_err(|e| BrokerError::Io(e.to_string()))? {
            if matches.len() >= limit {
                break;
            }
            let entry = entry.map_err(|e| BrokerError::Io(e.to_string()))?;
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') && name != ".rex" {
                continue;
            }
            let rel = if dir_path.is_empty() {
                name.clone()
            } else {
                format!("{dir_path}/{name}")
            };
            let file_type = entry
                .file_type()
                .map_err(|e| BrokerError::Io(e.to_string()))?;
            if file_type.is_dir() {
                if WORKSPACE_SEARCH_SKIP_DIRS.contains(&name.as_str())
                    || depth >= WORKSPACE_SEARCH_MAX_DEPTH
                {
                    continue;
                }
                queue.push((rel, depth + 1));
                continue;
            }
            if name.to_ascii_lowercase().contains(&needle)
                || rel.to_ascii_lowercase().contains(&needle)
            {
                matches.push(rel);
            }
        }
    }
    Ok(matches)
}

fn search_paths_by_content(
    root: &Path,
    query: &str,
    limit: usize,
) -> Result<Vec<String>, BrokerError> {
    let needle = query.to_ascii_lowercase();
    let mut matches = Vec::new();
    let mut queue = vec![(String::new(), 0usize)];
    let mut visited = 0usize;
    while let Some((dir_path, depth)) = queue.pop() {
        if visited >= WORKSPACE_SEARCH_MAX_DIRS || matches.len() >= limit {
            break;
        }
        visited += 1;
        let resolved = if dir_path.is_empty() {
            root.to_path_buf()
        } else {
            resolve_under_workspace(root, &dir_path)?
        };
        for entry in fs::read_dir(&resolved).map_err(|e| BrokerError::Io(e.to_string()))? {
            if matches.len() >= limit {
                break;
            }
            let entry = entry.map_err(|e| BrokerError::Io(e.to_string()))?;
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') && name != ".rex" {
                continue;
            }
            let rel = if dir_path.is_empty() {
                name.clone()
            } else {
                format!("{dir_path}/{name}")
            };
            let file_type = entry
                .file_type()
                .map_err(|e| BrokerError::Io(e.to_string()))?;
            if file_type.is_dir() {
                if WORKSPACE_SEARCH_SKIP_DIRS.contains(&name.as_str())
                    || depth >= WORKSPACE_SEARCH_MAX_DEPTH
                {
                    continue;
                }
                queue.push((rel, depth + 1));
                continue;
            }
            if !is_searchable_extension(&rel) {
                continue;
            }
            let full = resolve_under_workspace(root, &rel)?;
            let Ok(text) = fs::read_to_string(&full) else {
                continue;
            };
            for (line_no, line) in text.lines().enumerate() {
                if line.to_ascii_lowercase().contains(&needle) {
                    matches.push(format!("{rel}:{}", line_no + 1));
                    break;
                }
            }
        }
    }
    Ok(matches)
}

fn is_searchable_extension(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.ends_with(".rs")
        || lower.ends_with(".py")
        || lower.ends_with(".md")
        || lower.ends_with(".toml")
        || lower.ends_with(".json")
        || lower.ends_with(".yaml")
        || lower.ends_with(".yml")
        || lower.ends_with(".tsx")
        || lower.ends_with(".ts")
        || lower.ends_with(".jsx")
        || lower.ends_with(".js")
        || lower.ends_with(".go")
        || lower.ends_with(".sh")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::fs;
    use std::sync::Arc;

    struct SettingsGuard {
        prev_cwd: std::path::PathBuf,
    }

    impl Drop for SettingsGuard {
        fn drop(&mut self) {
            crate::settings::reset_for_test();
            if self.prev_cwd.is_dir() {
                let _ = std::env::set_current_dir(&self.prev_cwd);
            } else {
                let _ = std::env::set_current_dir(std::env::temp_dir());
            }
        }
    }

    fn init_workspace_root(root: &str) -> SettingsGuard {
        crate::settings::reset_for_test();
        let prev_cwd = std::env::current_dir().unwrap_or_else(|_| std::env::temp_dir());
        let path = std::path::PathBuf::from(root);
        std::fs::create_dir_all(&path).expect("workspace dir");
        std::env::set_current_dir(&path).expect("chdir workspace");
        let cfg = rex_config::RexConfig::defaults();
        crate::settings::init_for_test(Arc::new(rex_config::LoadedConfig::for_test(
            std::path::PathBuf::from("/tmp/rex-broker-test"),
            cfg,
        )));
        SettingsGuard { prev_cwd }
    }

    #[test]
    #[serial]
    fn reads_file_under_workspace() {
        let dir = std::env::temp_dir().join(format!("rex-broker-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("tmpdir");
        let file = dir.join("hello.txt");
        fs::write(&file, "broker-ok").expect("write");
        let _guard = init_workspace_root(&dir.display().to_string());
        let content = broker_read_file("hello.txt", "agent").expect("read");
        assert!(content.contains("<<TOOL_RESULT:fs.read>>"));
        assert!(content.contains("broker-ok"));
        assert!(content.contains("<<END>>"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    #[serial]
    fn writes_file_under_workspace() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir =
            std::env::temp_dir().join(format!("rex-broker-write-{}-{}", std::process::id(), nonce));
        fs::create_dir_all(&dir).expect("tmpdir");
        let _guard = init_workspace_root(&dir.display().to_string());
        broker_write_file("out.txt", "written-by-broker", "agent").expect("write");
        let content = broker_read_file("out.txt", "agent").expect("read back");
        assert!(content.contains("written-by-broker"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    #[serial]
    fn ask_mode_denies_write() {
        let dir = std::env::temp_dir().join(format!("rex-broker-ask-write-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("tmpdir");
        let _guard = init_workspace_root(&dir.display().to_string());
        let err = broker_write_file("out.txt", "x", "ask").unwrap_err();
        assert!(matches!(
            err,
            BrokerError::PolicyDenied {
                code,
                ..
            } if code == "mode_denied"
        ));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    #[serial]
    fn denies_protected_env_file() {
        let dir = std::env::temp_dir().join(format!("rex-broker-policy-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("tmpdir");
        let _guard = init_workspace_root(&dir.display().to_string());
        let err = broker_read_file(".env", "agent").unwrap_err();
        assert!(matches!(err, BrokerError::PolicyDenied { .. }));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    #[serial]
    fn exec_shell_runs_allowlisted_echo() {
        let root = std::env::temp_dir().join(format!("rex-broker-shell-{}", std::process::id()));
        fs::create_dir_all(&root).expect("tmpdir");
        let _guard = init_workspace_root(&root.display().to_string());
        let out = broker_exec_shell("echo broker-shell-ok", "agent").expect("echo");
        assert!(out.stdout.contains("<<TOOL_RESULT:exec.shell>>"));
        assert!(out.stdout.contains("broker-shell-ok"));
        assert!(out.stderr.is_empty());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    #[serial]
    fn ask_mode_denies_exec() {
        let root = std::env::temp_dir().join(format!("rex-broker-ask-exec-{}", std::process::id()));
        fs::create_dir_all(&root).expect("tmpdir");
        let _guard = init_workspace_root(&root.display().to_string());
        let err = broker_exec_shell("echo hi", "ask").unwrap_err();
        assert!(matches!(
            err,
            BrokerError::PolicyDenied {
                code,
                ..
            } if code == "mode_denied"
        ));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    #[serial]
    fn exec_shell_denies_disallowed_command() {
        let root =
            std::env::temp_dir().join(format!("rex-broker-shell-deny-{}", std::process::id()));
        fs::create_dir_all(&root).expect("tmpdir");
        let _guard = init_workspace_root(&root.display().to_string());
        let err = broker_exec_shell("rm -rf /", "agent").unwrap_err();
        assert!(matches!(err, BrokerError::CommandNotAllowed(_)));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    #[serial]
    fn rejects_parent_traversal() {
        let dir = std::env::temp_dir().join(format!("rex-broker-deny-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("tmpdir");
        let _guard = init_workspace_root(&dir.display().to_string());
        let err = broker_read_file("../etc/passwd", "agent").unwrap_err();
        assert_eq!(
            err,
            BrokerError::EscapesWorkspace("../etc/passwd".to_string())
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    #[serial]
    fn lists_workspace_root_and_subdirectory() {
        let dir = std::env::temp_dir().join(format!("rex-broker-list-{}", std::process::id()));
        fs::create_dir_all(dir.join("src")).expect("srcdir");
        fs::write(dir.join("hello.txt"), "x").expect("file");
        let _guard = init_workspace_root(&dir.display().to_string());
        let root_entries = broker_list_dir("", "ask").expect("list root");
        assert!(root_entries
            .iter()
            .any(|e| e.name == "hello.txt" && !e.is_dir));
        assert!(root_entries.iter().any(|e| e.name == "src" && e.is_dir));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    #[serial]
    fn list_dir_denies_protected_path() {
        let dir = std::env::temp_dir().join(format!("rex-broker-list-deny-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("tmpdir");
        let _guard = init_workspace_root(&dir.display().to_string());
        let err = broker_list_dir(".env", "ask").unwrap_err();
        assert!(matches!(err, BrokerError::PolicyDenied { .. }));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    #[serial]
    fn list_dir_caps_entries() {
        let dir = std::env::temp_dir().join(format!("rex-broker-list-cap-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("tmpdir");
        for i in 0..300 {
            fs::write(dir.join(format!("file-{i:03}.txt")), "x").expect("file");
        }
        let _guard = init_workspace_root(&dir.display().to_string());
        let entries = broker_list_dir("", "agent").expect("list");
        assert_eq!(entries.len(), MAX_LIST_DIR_ENTRIES);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    #[serial]
    fn truncates_large_read_content_at_line_boundary() {
        let mut cfg = rex_config::RexConfig::defaults();
        cfg.broker.max_tool_result_bytes = 64;
        crate::settings::reset_for_test();
        crate::settings::init_for_test(Arc::new(rex_config::LoadedConfig::for_test(
            std::path::PathBuf::from("/tmp/rex-broker-test"),
            cfg,
        )));
        let long = "line-one\nline-two\nline-three\nline-four";
        let out = format_delimited_tool_result("fs.read", long);
        assert!(out.contains("<<TOOL_RESULT:fs.read>>"));
        assert!(out.contains("<<END>>"));
        assert!(out.len() <= 64);
        assert!(!out.contains("line-three"));
        crate::settings::reset_for_test();
    }

    #[test]
    fn truncate_at_line_boundary_never_splits_line() {
        let body = format!("abcdefgh\n{}", "y".repeat(50));
        let out = truncate_tool_result_at_line_boundary(&body, 9 + TRUNCATION_MARKER.len());
        assert_eq!(out, format!("abcdefgh{TRUNCATION_MARKER}"));
    }

    #[test]
    #[serial]
    fn delimited_result_respects_utf8_byte_budget() {
        let mut cfg = rex_config::RexConfig::defaults();
        cfg.broker.max_tool_result_bytes = 96;
        crate::settings::reset_for_test();
        crate::settings::init_for_test(Arc::new(rex_config::LoadedConfig::for_test(
            std::path::PathBuf::from("/tmp/rex-broker-test"),
            cfg,
        )));
        let body = format!("café\n{}", "second-line\n".repeat(8));
        let out = format_delimited_tool_result("fs.read", &body);
        assert!(out.len() <= 96);
        assert!(out.contains("<<TOOL_RESULT:fs.read>>"));
        assert!(out.contains(TRUNCATION_MARKER) || !out.contains("second-line\nsecond-line"));
        crate::settings::reset_for_test();
    }

    #[test]
    #[serial]
    fn workspace_search_finds_basename_match() {
        let dir = std::env::temp_dir().join(format!("rex-broker-search-{}", std::process::id()));
        fs::create_dir_all(dir.join("docs")).expect("mkdir");
        fs::write(dir.join("docs/ROADMAP.md"), "# roadmap").expect("write");
        let _guard = init_workspace_root(&dir.display().to_string());
        let results = broker_workspace_search("roadmap", WorkspaceSearchKind::Basename, "ask", 10)
            .expect("search");
        assert!(results.contains("docs/ROADMAP.md"));
        let _ = fs::remove_dir_all(&dir);
    }
}
