use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::error::ConfigError;
use crate::merge::LoadedConfig;
use crate::model::{DaemonSocketScope, RexConfig};
use crate::workspace::resolve_workspace_root_for_effective;

const SOCKETS_DIR: &str = "sockets";
const HASH_HEX_LEN: usize = 16;

#[derive(Debug)]
pub struct ResolvedSockets {
    pub daemon_socket: String,
    pub host_sidecar_socket: String,
}

pub fn resolve_sockets(config: &RexConfig, rex_root: &Path) -> Result<ResolvedSockets, ConfigError> {
    match config.daemon.effective_socket_scope() {
        DaemonSocketScope::Global => {
            let host_name = config.host_sidecar_name();
            Ok(ResolvedSockets {
                daemon_socket: config.daemon.resolved_socket().to_string(),
                host_sidecar_socket: config
                    .sidecars
                    .list
                    .iter()
                    .find(|entry| entry.name == host_name)
                    .map(|entry| entry.socket.clone())
                    .unwrap_or_else(|| crate::model::DEFAULT_SIDECAR_SOCKET.to_string()),
            })
        }
        DaemonSocketScope::PerWorkspace => {
            let workspace_root = resolve_workspace_root_for_effective(config).map_err(|_| {
                ConfigError::Validation(
                    "daemon.socket_scope is per_workspace but workspace.root is not configured"
                        .to_string(),
                )
            })?;
            let hash = workspace_hash(&workspace_root);
            let dir = rex_root.join(SOCKETS_DIR);
            Ok(ResolvedSockets {
                daemon_socket: dir
                    .join(format!("ws-{hash}.sock"))
                    .to_string_lossy()
                    .into_owned(),
                host_sidecar_socket: dir
                    .join(format!("ws-{hash}-sidecar.sock"))
                    .to_string_lossy()
                    .into_owned(),
            })
        }
    }
}

impl LoadedConfig {
    pub fn daemon_autostart_lock_path(&self) -> PathBuf {
        if self.effective.daemon.effective_socket_scope() == DaemonSocketScope::Global {
            return self.rex_root.join(".daemon-autostart.lock");
        }
        let hash = self
            .resolve_workspace_root()
            .map(|root| workspace_hash(&root))
            .unwrap_or_else(|_| "unknown".to_string());
        self.rex_root
            .join(SOCKETS_DIR)
            .join(format!("ws-{hash}.autostart.lock"))
    }

    pub fn host_sidecar_socket(&self) -> &str {
        &self.resolved_host_sidecar_socket
    }

    pub fn ensure_sockets_dir(&self) -> Result<(), ConfigError> {
        if self.effective.daemon.effective_socket_scope() != DaemonSocketScope::PerWorkspace {
            return Ok(());
        }
        let dir = self.rex_root.join(SOCKETS_DIR);
        std::fs::create_dir_all(&dir).map_err(ConfigError::Io)?;
        Ok(())
    }
}

pub fn workspace_hash(workspace_root: &Path) -> String {
    let normalized = workspace_root.to_string_lossy();
    let digest = Sha256::digest(normalized.as_bytes());
    hex_encode(&digest[..HASH_HEX_LEN / 2])
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::RexConfig;

    #[test]
    fn per_workspace_sockets_are_stable_for_same_root() {
        let mut cfg = RexConfig::defaults();
        cfg.daemon.socket_scope = Some(DaemonSocketScope::PerWorkspace);
        cfg.workspace.root = "/tmp/project-a".to_string();
        let rex_root = PathBuf::from("/home/operator/.rex");
        let first = resolve_sockets(&cfg, &rex_root).expect("resolve");
        let second = resolve_sockets(&cfg, &rex_root).expect("resolve");
        assert_eq!(first.daemon_socket, second.daemon_socket);
        assert!(first.daemon_socket.contains("/sockets/ws-"));
        assert!(first.host_sidecar_socket.ends_with("-sidecar.sock"));
    }

    #[test]
    fn per_workspace_sockets_differ_for_different_roots() {
        let rex_root = PathBuf::from("/home/operator/.rex");
        let mut cfg_a = RexConfig::defaults();
        cfg_a.daemon.socket_scope = Some(DaemonSocketScope::PerWorkspace);
        cfg_a.workspace.root = "/tmp/project-a".to_string();
        let mut cfg_b = RexConfig::defaults();
        cfg_b.daemon.socket_scope = Some(DaemonSocketScope::PerWorkspace);
        cfg_b.workspace.root = "/tmp/project-b".to_string();
        let a = resolve_sockets(&cfg_a, &rex_root).expect("a");
        let b = resolve_sockets(&cfg_b, &rex_root).expect("b");
        assert_ne!(a.daemon_socket, b.daemon_socket);
    }

    #[test]
    fn global_scope_uses_configured_socket() {
        let mut cfg = RexConfig::defaults();
        cfg.daemon.socket_scope = Some(DaemonSocketScope::Global);
        cfg.daemon.socket = Some("/tmp/custom.sock".to_string());
        let resolved = resolve_sockets(&cfg, Path::new("/home/operator/.rex")).expect("resolve");
        assert_eq!(resolved.daemon_socket, "/tmp/custom.sock");
    }

    #[test]
    fn per_workspace_without_root_errors() {
        let mut cfg = RexConfig::defaults();
        cfg.daemon.socket_scope = Some(DaemonSocketScope::PerWorkspace);
        cfg.workspace.root = String::new();
        let err = resolve_sockets(&cfg, Path::new("/home/operator/.rex")).expect_err("missing");
        assert!(err.to_string().contains("workspace.root"));
    }
}
