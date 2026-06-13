/// Local Unix Domain Socket used by the daemon.
pub const SOCKET_PATH: &str = "/tmp/rex.sock";

/// Timeout budget for establishing a daemon connection.
pub const CONNECT_TIMEOUT_SECONDS: u64 = 2;

/// Timeout budget for unary RPC operations (status, etc.).
pub const REQUEST_TIMEOUT_SECONDS: u64 = 5;

/// End-to-end budget for a streaming `StreamInference` RPC (multi-step turns).
pub fn stream_request_timeout_seconds(idle_timeout_secs: u64) -> u64 {
    idle_timeout_secs.saturating_mul(10).max(120)
}

/// Timeout budget for receiving each stream item from daemon (fallback when config is unavailable).
pub const STREAM_ITEM_TIMEOUT_SECONDS: u64 = 120;

/// Retry attempts for initial stream start when daemon is still booting.
pub const STREAM_START_RETRY_ATTEMPTS: u32 = 5;

/// Delay between retry attempts for initial stream start.
pub const STREAM_START_RETRY_DELAY_MS: u64 = 150;

/// High-level lifecycle outcomes for `complete` command execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamLifecycle {
    Completed,
    Incomplete,
}
