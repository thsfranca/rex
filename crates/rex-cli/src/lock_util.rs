//! PID lock files for single-holder resources (daemon autostart, harness sessions).

use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

pub struct PidLock {
    path: PathBuf,
    held: bool,
}

impl Drop for PidLock {
    fn drop(&mut self) {
        if self.held {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

impl PidLock {
    pub fn path(&self) -> &Path {
        &self.path
    }
}

enum LockAcquireError {
    Contended,
    Io(io::Error),
}

pub fn try_acquire_lock(path: &Path) -> Option<PidLock> {
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
                "rex: warning: could not acquire lock at {}: {err}",
                path.display()
            );
            None
        }
    }
}

pub fn lock_holder_alive(path: &Path) -> bool {
    read_lock_pid(path)
        .map(process_alive)
        .unwrap_or(false)
}

fn create_lock_file(path: &Path) -> Result<PidLock, LockAcquireError> {
    let pid = std::process::id();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(LockAcquireError::Io)?;
    }
    match OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
    {
        Ok(mut file) => {
            let _ = writeln!(file, "{pid}");
            Ok(PidLock {
                path: path.to_path_buf(),
                held: true,
            })
        }
        Err(err) if err.kind() == io::ErrorKind::AlreadyExists => Err(LockAcquireError::Contended),
        Err(err) => Err(LockAcquireError::Io(err)),
    }
}

pub(crate) fn read_lock_pid(path: &Path) -> Option<u32> {
    let contents = std::fs::read_to_string(path).ok()?;
    contents.split_whitespace().next()?.parse().ok()
}

fn process_alive(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    #[cfg(unix)]
    {
        std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lock_acquire_is_exclusive() {
        let lock_path = std::env::temp_dir().join(format!(
            "rex-lock-test-{}",
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
