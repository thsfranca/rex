use std::fs::OpenOptions;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use rex_config::REX_ROOT_ENV;
use rex_config::LoadedConfig;
use rex_proto::rex::v1::GetSystemStatusRequest;
use tokio::sync::Mutex;
use tokio::time::sleep;

use crate::domain::REQUEST_TIMEOUT_SECONDS;
use crate::error::CliError;
use crate::transport::connect_client;

const POLL_INTERVAL_MS: u64 = 250;
const LOCK_FILE_NAME: &str = ".daemon-autostart.lock";

static ENSURE_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnsureOptions {
    pub no_autostart: bool,
}

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

pub async fn ensure_daemon_ready(opts: EnsureOptions) -> Result<(), CliError> {
    let mutex = ENSURE_MUTEX.get_or_init(|| Mutex::new(()));
    let _guard = mutex.lock().await;

    let loaded = load_config()?;
    let socket_path = loaded.daemon_socket().to_string();
    let log_path = loaded.daemon_log_path();
    let timeout_secs = loaded.daemon_ready_timeout_secs();
    let auto_start = loaded.daemon_auto_start() && !opts.no_autostart;

    if probe_daemon().await.is_ok() {
        return Ok(());
    }

    if !auto_start {
        return Err(CliError::daemon_unavailable_manual(&socket_path));
    }

    let lock_path = loaded.rex_root.join(LOCK_FILE_NAME);
    let lock = try_acquire_lock(&lock_path);
    if lock.is_some() {
        spawn_detached_daemon(&loaded, &log_path)?;
    }

    poll_until_ready(timeout_secs, &log_path).await
}

async fn probe_daemon() -> Result<(), CliError> {
    let mut client = connect_client(None).await?;
    let mut request = tonic::Request::new(GetSystemStatusRequest {});
    request.set_timeout(Duration::from_secs(REQUEST_TIMEOUT_SECONDS));
    client.get_system_status(request).await?;
    Ok(())
}

fn load_config() -> Result<LoadedConfig, CliError> {
    rex_config::load_merged().map_err(|err| {
        CliError::DaemonUnavailable {
            socket_path: crate::domain::SOCKET_PATH.to_string(),
            suffix: format!("; failed to load config: {err}"),
        }
    })
}

fn try_acquire_lock(path: &Path) -> Option<AutostartLock> {
    match OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
    {
        Ok(_) => Some(AutostartLock {
            path: path.to_path_buf(),
            held: true,
        }),
        Err(err) if err.kind() == io::ErrorKind::AlreadyExists => None,
        Err(err) => {
            eprintln!("rex: warning: could not acquire daemon autostart lock at {}: {err}", path.display());
            None
        }
    }
}

fn spawn_detached_daemon(loaded: &LoadedConfig, log_path: &Path) -> Result<(), CliError> {
    let rex_binary = std::env::current_exe().map_err(|source| {
        CliError::daemon_spawn_failed(log_path, format!("could not resolve rex binary: {source}"))
    })?;

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
        .arg("daemon")
        .stdin(Stdio::null())
        .stdout(Stdio::from(log_file))
        .stderr(Stdio::from(stderr))
        .env(REX_ROOT_ENV, &loaded.rex_root)
        .spawn()
        .map_err(|source| {
            CliError::daemon_spawn_failed(
                log_path,
                format!("could not start Rex: {source}"),
            )
        })?;

    Ok(())
}

async fn poll_until_ready(timeout_secs: u64, log_path: &Path) -> Result<(), CliError> {
    let deadline = Instant::now() + Duration::from_secs(timeout_secs);
    while Instant::now() < deadline {
        if probe_daemon().await.is_ok() {
            return Ok(());
        }
        sleep(Duration::from_millis(POLL_INTERVAL_MS)).await;
    }
    Err(CliError::daemon_ready_timeout(
        log_path,
        timeout_secs,
    ))
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
        let first = try_acquire_lock(&lock_path);
        assert!(first.is_some());
        assert!(try_acquire_lock(&lock_path).is_none());
        drop(first);
        assert!(try_acquire_lock(&lock_path).is_some());
        let _ = std::fs::remove_file(&lock_path);
    }
}
