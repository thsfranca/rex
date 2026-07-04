use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use rex_config::{ensure_project_workspace_root, DaemonSocketScope, LoadedConfig, REX_ROOT_ENV};
use rex_proto::rex::v1::{GetSystemStatusRequest, GetSystemStatusResponse};
use tokio::sync::Mutex;
use tokio::time::sleep;

use crate::domain::REQUEST_TIMEOUT_SECONDS;
use crate::error::CliError;
use crate::transport::connect_client;

const POLL_INTERVAL_MS: u64 = 250;

static ENSURE_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();

struct AutostartLock {
    path: PathBuf,
    held: bool,
}

impl Drop for AutostartLock {
    fn drop(&mut self) {
        if self.held {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

pub async fn ensure_daemon_ready() -> Result<(), CliError> {
    let mutex = ENSURE_MUTEX.get_or_init(|| Mutex::new(()));
    let _guard = mutex.lock().await;

    let loaded = prepare_loaded_config()?;
    let socket_path = loaded.daemon_socket().to_string();
    let log_path = loaded.daemon_log_path();
    let timeout_secs = loaded.daemon_ready_timeout_secs();

    if probe_daemon(&loaded).await.is_ok() {
        return Ok(());
    }

    loaded.ensure_sockets_dir().map_err(|err| CliError::DaemonUnavailable {
        socket_path: socket_path.clone(),
        suffix: format!("; {err}"),
    })?;

    let lock_path = loaded.daemon_autostart_lock_path();
    if let Some(parent) = lock_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let autostart_lock = try_acquire_autostart_lock(&lock_path);

    if autostart_lock.is_some() {
        if probe_daemon(&loaded).await.is_err() {
            spawn_detached_daemon(&loaded, &log_path)?;
        }
    }

    let result = poll_until_ready(&loaded, timeout_secs, &log_path).await;
    drop(autostart_lock);
    result
}

fn try_acquire_autostart_lock(path: &Path) -> Option<AutostartLock> {
    match create_lock_file(path) {
        Ok(lock) => Some(lock),
        Err(LockAcquireError::Contended) => {
            if lock_holder_alive(path) {
                return None;
            }
            let _ = std::fs::remove_file(path);
            create_lock_file(path).ok()
        }
        Err(LockAcquireError::Io(err)) => {
            eprintln!(
                "rex: warning: could not acquire daemon autostart lock at {}: {err}",
                path.display()
            );
            None
        }
    }
}

enum LockAcquireError {
    Contended,
    Io(io::Error),
}

fn create_lock_file(path: &Path) -> Result<AutostartLock, LockAcquireError> {
    let pid = std::process::id();
    match OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
    {
        Ok(mut file) => {
            let _ = writeln!(file, "{pid}");
            Ok(AutostartLock {
                path: path.to_path_buf(),
                held: true,
            })
        }
        Err(err) if err.kind() == io::ErrorKind::AlreadyExists => Err(LockAcquireError::Contended),
        Err(err) => Err(LockAcquireError::Io(err)),
    }
}

fn read_lock_pid(path: &Path) -> Option<u32> {
    let contents = std::fs::read_to_string(path).ok()?;
    contents.split_whitespace().next()?.parse().ok()
}

fn lock_holder_alive(path: &Path) -> bool {
    read_lock_pid(path)
        .map(process_alive)
        .unwrap_or(false)
}

fn process_alive(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    #[cfg(unix)]
    {
        std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
        true
    }
}

async fn probe_daemon(loaded: &LoadedConfig) -> Result<(), CliError> {
    let mut client = connect_client(None).await?;
    let mut request = tonic::Request::new(GetSystemStatusRequest {});
    request.set_timeout(Duration::from_secs(REQUEST_TIMEOUT_SECONDS));
    let response = client.get_system_status(request).await?;
    validate_workspace_match(loaded, &response.into_inner())
}

fn validate_workspace_match(
    loaded: &LoadedConfig,
    status: &GetSystemStatusResponse,
) -> Result<(), CliError> {
    if loaded.effective.daemon.effective_socket_scope() == DaemonSocketScope::Global {
        return Ok(());
    }
    let expected = loaded
        .resolve_workspace_root()
        .map_err(|_| CliError::workspace_not_configured())?;
    let reported = status.workspace_root.trim();
    if reported.is_empty() {
        return Err(CliError::workspace_mismatch(
            loaded.daemon_socket(),
            expected.display().to_string(),
            reported.to_string(),
        ));
    }
    if paths_equal(reported, &expected.display().to_string()) {
        Ok(())
    } else {
        Err(CliError::workspace_mismatch(
            loaded.daemon_socket(),
            expected.display().to_string(),
            reported.to_string(),
        ))
    }
}

fn paths_equal(left: &str, right: &str) -> bool {
    let left_path = PathBuf::from(left);
    let right_path = PathBuf::from(right);
    let left_canon = std::fs::canonicalize(&left_path).unwrap_or(left_path);
    let right_canon = std::fs::canonicalize(&right_path).unwrap_or(right_path);
    left_canon == right_canon
}

fn prepare_loaded_config() -> Result<LoadedConfig, CliError> {
    let loaded = load_config()?;
    if loaded.effective.daemon.effective_socket_scope() != DaemonSocketScope::PerWorkspace {
        return Ok(loaded);
    }
    let workspace_root = loaded
        .resolve_workspace_root()
        .map_err(|_| CliError::workspace_not_configured())?;
    ensure_project_workspace_root(&workspace_root).map_err(|err| CliError::DaemonUnavailable {
        socket_path: loaded.daemon_socket().to_string(),
        suffix: format!("; could not write project config: {err}"),
    })?;
    load_config()
}

fn load_config() -> Result<LoadedConfig, CliError> {
    rex_config::load_merged().map_err(|err| {
        CliError::DaemonUnavailable {
            socket_path: crate::domain::SOCKET_PATH.to_string(),
            suffix: format!("; failed to load config: {err}"),
        }
    })
}

fn resolve_rex_binary(log_path: &Path) -> Result<PathBuf, CliError> {
    if let Ok(path) = std::env::var("CARGO_BIN_EXE_rex") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Ok(path);
        }
    }
    std::env::current_exe().map_err(|source| {
        CliError::daemon_spawn_failed(log_path, format!("could not resolve rex binary: {source}"))
    })
}

fn spawn_detached_daemon(loaded: &LoadedConfig, log_path: &Path) -> Result<(), CliError> {
    let spawn_cwd = match loaded.effective.daemon.effective_socket_scope() {
        DaemonSocketScope::Global => std::env::current_dir().map_err(|source| {
            CliError::daemon_spawn_failed(
                log_path,
                format!("could not resolve spawn cwd: {source}"),
            )
        })?,
        DaemonSocketScope::PerWorkspace => loaded
            .resolve_workspace_root()
            .map_err(|_| {
                CliError::daemon_spawn_failed(log_path, "workspace root not configured".to_string())
            })?,
    };

    let rex_binary = resolve_rex_binary(log_path)?;

    if let Some(parent) = log_path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| {
            CliError::daemon_spawn_failed(
                log_path,
                format!("could not create log directory {}: {source}", parent.display()),
            )
        })?;
    }

    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .map_err(|source| {
            CliError::daemon_spawn_failed(
                log_path,
                format!("could not open log file {}: {source}", log_path.display()),
            )
        })?;

    let stderr = log_file
        .try_clone()
        .map_err(|source| CliError::daemon_spawn_failed(log_path, source.to_string()))?;

    std::process::Command::new(rex_binary)
        .arg("__rex_internal_daemon")
        .stdin(Stdio::null())
        .stdout(Stdio::from(log_file))
        .stderr(Stdio::from(stderr))
        .current_dir(&spawn_cwd)
        .env(REX_ROOT_ENV, &loaded.rex_root)
        .spawn()
        .map_err(|source| {
            CliError::daemon_spawn_failed(log_path, format!("could not start Rex: {source}"))
        })?;

    Ok(())
}

async fn poll_until_ready(
    loaded: &LoadedConfig,
    timeout_secs: u64,
    log_path: &Path,
) -> Result<(), CliError> {
    let deadline = Instant::now() + Duration::from_secs(timeout_secs);
    while Instant::now() < deadline {
        if probe_daemon(loaded).await.is_ok() {
            return Ok(());
        }
        sleep(Duration::from_millis(POLL_INTERVAL_MS)).await;
    }
    Err(CliError::daemon_ready_timeout(log_path, timeout_secs))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lock_acquire_is_exclusive() {
        let lock_path = std::env::temp_dir().join(format!(
            "rex-autostart-lock-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&lock_path);
        let first = try_acquire_autostart_lock(&lock_path);
        assert!(first.is_some());
        assert!(try_acquire_autostart_lock(&lock_path).is_none());
        drop(first);
        assert!(try_acquire_autostart_lock(&lock_path).is_some());
        let _ = std::fs::remove_file(&lock_path);
    }

    #[test]
    fn lock_records_holder_pid() {
        let lock_path = std::env::temp_dir().join(format!(
            "rex-autostart-pid-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&lock_path);
        let lock = try_acquire_autostart_lock(&lock_path).expect("lock");
        assert_eq!(read_lock_pid(&lock_path), Some(std::process::id()));
        assert!(lock_holder_alive(&lock_path));
        drop(lock);
        let _ = std::fs::remove_file(&lock_path);
    }

    #[test]
    fn dead_pid_lock_is_reclaimed() {
        let lock_path = std::env::temp_dir().join(format!(
            "rex-autostart-dead-pid-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&lock_path);
        std::fs::write(&lock_path, "999999999\n").expect("write stale lock");
        assert!(!lock_holder_alive(&lock_path));
        let lock = try_acquire_autostart_lock(&lock_path).expect("reclaim");
        assert_eq!(read_lock_pid(&lock_path), Some(std::process::id()));
        drop(lock);
        let _ = std::fs::remove_file(&lock_path);
    }
}
